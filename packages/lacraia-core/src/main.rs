use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures::{SinkExt, StreamExt};
use lacraia_shared::{Command, PlaybackState, Track, WsMessage};
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{broadcast, Mutex};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::info;

pub mod db;
pub mod resolver;
pub mod similarity;

use db::Database;
use resolver::MediaResolver;
use similarity::{CompositeSimilarityEngine, SimilarityEngine};

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub similarity: Arc<CompositeSimilarityEngine>,
    pub resolver: Arc<MediaResolver>,
    pub cache_dir: PathBuf,
    pub tx: broadcast::Sender<WsMessage>,
    pub current_track: Arc<Mutex<Option<Track>>>,
    pub playback_state: Arc<Mutex<PlaybackState>>,
}

#[derive(Debug, Deserialize)]
pub struct RadioQuery {
    pub artist: String,
    pub track: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveQuery {
    pub artist: String,
    pub title: String,
    #[serde(rename = "expectedDurationSec")]
    pub expected_duration_sec: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lacraia_core=info,tower_http=info".into()),
        )
        .init();

    let cache_dir = PathBuf::from(std::env::var("LACRAIA_CACHE_DIR").unwrap_or_else(|_| "./cache".into()));
    tokio::fs::create_dir_all(&cache_dir).await?;

    let db_path = std::env::var("LACRAIA_DB_PATH").unwrap_or_else(|_| "./lacraia.sqlite".into());
    let db = Database::open(&db_path)?;

    let lastfm_key = std::env::var("LASTFM_API_KEY").ok();
    let similarity = Arc::new(CompositeSimilarityEngine::new(lastfm_key, db.clone()));
    let resolver = Arc::new(MediaResolver::new(cache_dir.clone(), db.clone()));

    let (tx, _rx) = broadcast::channel::<WsMessage>(100);

    let state = AppState {
        db,
        similarity,
        resolver,
        cache_dir: cache_dir.clone(),
        tx,
        current_track: Arc::new(Mutex::new(None)),
        playback_state: Arc::new(Mutex::new(PlaybackState::Stopped)),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/radio", get(handle_radio))
        .route("/api/resolve", get(handle_resolve))
        .route("/ws", get(ws_handler))
        .nest_service("/cache", ServeDir::new(cache_dir))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("La Craia Core listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

async fn handle_radio(
    State(state): State<AppState>,
    Query(query): Query<RadioQuery>,
) -> Result<Json<Vec<Track>>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(20);
    info!("Radio query for artist={}, track={}", query.artist, query.track);

    match state.similarity.find_similar(&query.artist, &query.track, limit).await {
        Ok(candidates) => {
            let tracks: Vec<Track> = candidates
                .into_iter()
                .map(|c| Track {
                    id: c.id,
                    title: c.title,
                    artist: c.artist,
                    album: None,
                    duration_ms: c.duration_sec.unwrap_or(210) * 1000,
                    cover_url: None,
                    stream_url: None,
                })
                .collect();
            Ok(Json(tracks))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Similarity query failed: {}", e),
        )),
    }
}

async fn handle_resolve(
    State(state): State<AppState>,
    Query(query): Query<ResolveQuery>,
) -> Result<Json<Track>, (StatusCode, String)> {
    info!(
        "Resolving stream for '{}' by '{}' (expected: {}s)",
        query.title, query.artist, query.expected_duration_sec
    );

    match state
        .resolver
        .resolve_and_cache(&query.artist, &query.title, query.expected_duration_sec)
        .await
    {
        Ok(resolution) => Ok(Json(Track {
            id: resolution.track_id,
            title: resolution.title,
            artist: resolution.artist,
            album: None,
            duration_ms: resolution.duration_sec * 1000,
            cover_url: None,
            stream_url: Some(resolution.stream_url),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Stream resolution failed: {}", e),
        )),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    let _tx = state.tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            if let Ok(cmd) = serde_json::from_str::<Command>(&text) {
                info!("Received command via WS: {:?}", cmd);
                // Dispatch command and notify clients
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
