use axum::{
    extract::{ws::WebSocketUpgrade, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use lacraia_shared::{Track, WsMessage};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::broadcast;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::info;

pub mod db;
pub mod queue_daemon;
pub mod resolver;
pub mod similarity;
pub mod ws;

use db::Database;
use queue_daemon::QueueDaemon;
use resolver::MediaResolver;
use similarity::{CompositeSimilarityEngine, SimilarityEngine};
use ws::PlaybackHub;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub similarity: Arc<CompositeSimilarityEngine>,
    pub resolver: Arc<MediaResolver>,
    pub hub: Arc<PlaybackHub>,
    pub cache_dir: PathBuf,
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

    let daemon = Arc::new(QueueDaemon::new(
        db.clone(),
        resolver.clone(),
        2, // Lookahead window of 2 tracks
        std::time::Duration::from_secs(5),
    ));
    daemon.start();

    let (tx, _rx) = broadcast::channel::<WsMessage>(100);
    let hub = Arc::new(PlaybackHub::new(db.clone(), tx));
    hub.clone().start_telemetry_ticker();

    let state = AppState {
        db,
        similarity,
        resolver,
        hub,
        cache_dir: cache_dir.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/radio", get(handle_radio))
        .route("/api/resolve", get(handle_resolve))
        .route("/api/cabin/join", post(handle_cabin_join))
        .route("/api/cabin/:id", get(handle_cabin_info))
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

#[derive(Debug, Deserialize)]
pub struct CabinJoinRequest {
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    pub name: String,
    pub device: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CabinJoinResponse {
    pub token: String,
    #[serde(rename = "passengerId")]
    pub passenger_id: String,
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct CabinDetailsResponse {
    pub id: String,
    pub name: String,
    #[serde(rename = "isLocked")]
    pub is_locked: bool,
    #[serde(rename = "passengersCount")]
    pub passengers_count: usize,
}

async fn handle_cabin_join(
    State(state): State<AppState>,
    Json(payload): Json<CabinJoinRequest>,
) -> Result<Json<CabinJoinResponse>, (StatusCode, String)> {
    if state.db.is_cabin_locked(&payload.cabin_id).unwrap_or(false) {
        return Err((
            StatusCode::FORBIDDEN,
            "CABIN_LOCKED: Portas trancadas pelo piloto".into(),
        ));
    }

    // Generate random dynamic token
    let token = format!("psg_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..12]);
    match state.db.add_passenger(&payload.cabin_id, &payload.name, payload.device.as_deref(), &token) {
        Ok(record) => {
            // Broadcast crew update via websocket hub
            let passengers = state.db.get_active_passengers(&payload.cabin_id).unwrap_or_default();
            let is_locked = state.db.is_cabin_locked(&payload.cabin_id).unwrap_or(false);
            let _ = state.hub.tx.send(WsMessage::CrewUpdate(lacraia_shared::CrewUpdate {
                cabin_id: payload.cabin_id.clone(),
                is_locked,
                passengers,
            }));

            Ok(Json(CabinJoinResponse {
                token: record.token,
                passenger_id: record.id,
                cabin_id: record.cabin_id,
                role: "PASSENGER".into(),
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to register passenger: {}", e),
        )),
    }
}

async fn handle_cabin_info(
    State(state): State<AppState>,
    axum::extract::Path(cabin_id): axum::extract::Path<String>,
) -> Result<Json<CabinDetailsResponse>, (StatusCode, String)> {
    let is_locked = state.db.is_cabin_locked(&cabin_id).unwrap_or(false);
    let passengers = state.db.get_active_passengers(&cabin_id).unwrap_or_default();

    Ok(Json(CabinDetailsResponse {
        id: cabin_id.clone(),
        name: format!("Cabine {}", cabin_id),
        is_locked,
        passengers_count: passengers.len(),
    }))
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
                    requested_by: None,
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
            requested_by: None,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Stream resolution failed: {}", e),
        )),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<ws::WsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state.hub, params))
}
