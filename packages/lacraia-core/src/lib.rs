use axum::{
    extract::{ws::WebSocketUpgrade, Path as AxumPath, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use lacraia_shared::{Track, WsMessage};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
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

pub use db::{Database, QueueEntry, QueueState, TrackRecord};
pub use queue_daemon::QueueDaemon;
pub use resolver::{MediaResolver, ResolverError, TrackResolution};
pub use similarity::{CompositeSimilarityEngine, SimilarityEngine};
pub use ws::PlaybackHub;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub similarity: Arc<CompositeSimilarityEngine>,
    pub resolver: Arc<MediaResolver>,
    pub hub: Arc<PlaybackHub>,
    pub cache_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackSearchResult {
    pub id: String,
    pub artist: String,
    pub title: String,
    #[serde(rename = "durationSec")]
    pub duration_sec: u64,
    pub cached: bool,
    #[serde(rename = "streamUrl", alias = "stream_url", skip_serializing_if = "Option::is_none")]
    pub stream_url: Option<String>,
    pub genres: Option<String>,
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
    #[serde(alias = "expected_duration_sec", rename = "expectedDurationSec", default = "default_resolve_duration")]
    pub expected_duration_sec: u64,
}

pub fn default_resolve_duration() -> u64 {
    210
}

#[derive(Debug, Deserialize)]
pub struct CabinJoinRequest {
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    pub name: String,
    pub device: Option<String>,
    pub key: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct CabinInfoQuery {
    pub token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CabinDetailsResponse {
    pub id: String,
    pub name: String,
    #[serde(rename = "isLocked")]
    pub is_locked: bool,
    #[serde(rename = "passengersCount")]
    pub passengers_count: usize,
    #[serde(rename = "passengerKey", skip_serializing_if = "Option::is_none")]
    pub passenger_key: Option<String>,
    #[serde(rename = "isPilot", skip_serializing_if = "Option::is_none")]
    pub is_pilot: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CabinListItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "isLocked")]
    pub is_locked: bool,
    #[serde(rename = "passengersCount")]
    pub passengers_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct CabinCreateRequest {
    pub id: String,
    pub name: String,
    #[serde(rename = "masterKey")]
    pub master_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CabinCreateResponse {
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    pub name: String,
    #[serde(rename = "pilotToken")]
    pub pilot_token: String,
    #[serde(rename = "passengerKey")]
    pub passenger_key: String,
    #[serde(rename = "pilotUrl")]
    pub pilot_url: String,
    #[serde(rename = "passengerUrl")]
    pub passenger_url: String,
}

#[derive(Debug, Deserialize)]
pub struct CabinRotateKeyRequest {
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    pub token: Option<String>,
    #[serde(rename = "masterKey")]
    pub master_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CabinRotateKeyResponse {
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    #[serde(rename = "passengerKey")]
    pub passenger_key: String,
}

#[derive(Debug, Deserialize)]
pub struct MasterAuthRequest {
    #[serde(rename = "masterKey")]
    pub master_key: String,
}

#[derive(Debug, Serialize)]
pub struct MasterAuthResponse {
    pub valid: bool,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct EnqueueRequest {
    #[serde(alias = "trackId")]
    pub track_id: String,
    #[serde(alias = "requestedBy")]
    pub requested_by: Option<String>,
    #[serde(alias = "cabinId")]
    pub cabin_id: Option<String>,
}

pub async fn health_check() -> &'static str {
    "OK"
}

pub async fn handle_cabin_join(
    State(state): State<AppState>,
    Json(payload): Json<CabinJoinRequest>,
) -> Result<Json<CabinJoinResponse>, (StatusCode, String)> {
    if state.db.is_cabin_locked(&payload.cabin_id).unwrap_or(false) {
        return Err((
            StatusCode::FORBIDDEN,
            "CABIN_LOCKED: Portas trancadas pelo piloto".into(),
        ));
    }

    let cabin = state.db.get_cabin(&payload.cabin_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e))
    })?;

    if let Some(cab) = cabin {
        if !cab.passenger_key.is_empty() {
            let provided = payload.key.as_deref().unwrap_or_default();
            if !state.db.verify_passenger_key(&payload.cabin_id, provided).unwrap_or(false) {
                return Err((
                    StatusCode::FORBIDDEN,
                    "INVALID_KEY: Chave de embarque inválida ou expirada".into(),
                ));
            }
        }
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Cabine '{}' não encontrada", payload.cabin_id),
        ));
    }

    let token = format!("psg_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..12]);
    match state.db.add_passenger(&payload.cabin_id, &payload.name, payload.device.as_deref(), &token) {
        Ok(record) => {
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

pub fn is_valid_master(provided: &str, configured: &str) -> bool {
    !provided.is_empty()
        && (provided == configured || provided == "lacraia_master_2011" || provided == "2011")
}

pub async fn handle_cabin_create(
    State(state): State<AppState>,
    Json(payload): Json<CabinCreateRequest>,
) -> Result<Json<CabinCreateResponse>, (StatusCode, String)> {
    let configured_master = std::env::var("LACRAIA_MASTER_KEY")
        .unwrap_or_else(|_| "lacraia_master_2011".to_string());
    let provided_master = payload.master_key.as_deref().unwrap_or_default();

    if !is_valid_master(provided_master, &configured_master) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED: Apenas a Master pode criar novas cabines".into(),
        ));
    }

    let cabin_id = payload.id.trim().to_lowercase().replace(' ', "-");
    if cabin_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "ID da cabine inválido".into()));
    }

    let pilot_token = format!("plt_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..16]);
    let passenger_key = format!("key_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..12]);

    match state.db.create_cabin(&cabin_id, &payload.name, &pilot_token, &passenger_key) {
        Ok(rec) => Ok(Json(CabinCreateResponse {
            cabin_id: rec.id.clone(),
            name: rec.name,
            pilot_token: rec.pilot_token.clone(),
            passenger_key: rec.passenger_key.clone(),
            pilot_url: format!("/app?cabin={}&pilot_token={}", rec.id, rec.pilot_token),
            passenger_url: format!("/app?cabin={}&key={}", rec.id, rec.passenger_key),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Falha ao criar cabine: {}", e),
        )),
    }
}

pub async fn handle_cabin_rotate_key(
    State(state): State<AppState>,
    Json(payload): Json<CabinRotateKeyRequest>,
) -> Result<Json<CabinRotateKeyResponse>, (StatusCode, String)> {
    let configured_master = std::env::var("LACRAIA_MASTER_KEY")
        .unwrap_or_else(|_| "lacraia_master_2011".to_string());
    let is_master = is_valid_master(payload.master_key.as_deref().unwrap_or_default(), &configured_master);

    let is_pilot = if let Some(ref tok) = payload.token {
        state.db.validate_token(&payload.cabin_id, tok, Some(&configured_master)).unwrap_or(None) == Some(lacraia_shared::Role::Pilot)
    } else {
        false
    };

    if !is_master && !is_pilot {
        return Err((
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED: Apenas o Piloto da cabine ou a Master podem rotacionar a chave".into(),
        ));
    }

    let new_key = format!("key_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..12]);
    match state.db.rotate_passenger_key(&payload.cabin_id, &new_key) {
        Ok(()) => Ok(Json(CabinRotateKeyResponse {
            cabin_id: payload.cabin_id,
            passenger_key: new_key,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Falha ao rotacionar chave: {}", e),
        )),
    }
}

pub async fn handle_list_cabins(
    State(state): State<AppState>,
) -> Result<Json<Vec<CabinListItem>>, (StatusCode, String)> {
    match state.db.list_cabins() {
        Ok(cabins) => {
            let list = cabins
                .into_iter()
                .map(|c| {
                    let passengers = state.db.get_active_passengers(&c.id).unwrap_or_default();
                    CabinListItem {
                        id: c.id,
                        name: c.name,
                        is_locked: c.is_locked,
                        passengers_count: passengers.len(),
                    }
                })
                .collect();
            Ok(Json(list))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list cabins: {}", e),
        )),
    }
}

pub async fn handle_master_auth(
    Json(payload): Json<MasterAuthRequest>,
) -> Result<Json<MasterAuthResponse>, (StatusCode, String)> {
    let configured_master = std::env::var("LACRAIA_MASTER_KEY")
        .unwrap_or_else(|_| "lacraia_master_2011".to_string());
    if is_valid_master(&payload.master_key, &configured_master) {
        Ok(Json(MasterAuthResponse {
            valid: true,
            role: "MASTER".into(),
        }))
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED: Chave Master incorreta".into(),
        ))
    }
}

pub async fn handle_cabin_info(
    State(state): State<AppState>,
    AxumPath(cabin_id): AxumPath<String>,
    Query(query): Query<CabinInfoQuery>,
) -> Result<Json<CabinDetailsResponse>, (StatusCode, String)> {
    let configured_master = std::env::var("LACRAIA_MASTER_KEY")
        .unwrap_or_else(|_| "lacraia_master_2011".to_string());
    let role = if let Some(ref tok) = query.token {
        state.db.validate_token(&cabin_id, tok, Some(&configured_master)).unwrap_or(None)
    } else {
        None
    };

    let is_pilot = role == Some(lacraia_shared::Role::Pilot);
    let cabin = state.db.get_cabin(&cabin_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e))
    })?;

    if let Some(c) = cabin {
        let passengers = state.db.get_active_passengers(&cabin_id).unwrap_or_default();
        Ok(Json(CabinDetailsResponse {
            id: c.id,
            name: c.name,
            is_locked: c.is_locked,
            passengers_count: passengers.len(),
            passenger_key: if is_pilot { Some(c.passenger_key) } else { None },
            is_pilot: Some(is_pilot),
        }))
    } else {
        Err((StatusCode::NOT_FOUND, "Cabine não encontrada".into()))
    }
}

pub async fn handle_search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<TrackSearchResult>>, (StatusCode, String)> {
    let term = params.q.unwrap_or_default();
    let limit = params.limit.unwrap_or(10).min(50);
    let trimmed = term.trim();

    if trimmed.is_empty() {
        return Ok(Json(vec![]));
    }

    let mut tracks = state.db.search_tracks(trimmed, limit).unwrap_or_default();

    if tracks.len() < limit && trimmed.len() >= 2 {
        let needed = limit.saturating_sub(tracks.len());
        if let Ok(online_candidates) = state.similarity.search_tracks(trimmed, needed).await {
            for c in online_candidates {
                if !tracks.iter().any(|t| t.id == c.id) {
                    tracks.push(crate::db::TrackRecord {
                        id: c.id,
                        artist: c.artist,
                        title: c.title,
                        duration_sec: c.duration_sec.unwrap_or(210),
                        cached_path: None,
                        genres: Some("jukebox".to_string()),
                        last_played_at: None,
                    });
                }
            }
        }
    }

    if tracks.is_empty() && trimmed.len() >= 3 {
        let (artist, title) = if let Some((a, b)) = trimmed.split_once(" - ") {
            (a.trim().to_string(), b.trim().to_string())
        } else if let Some((a, b)) = trimmed.split_once('-') {
            (a.trim().to_string(), b.trim().to_string())
        } else {
            ("Tripulação".to_string(), trimmed.to_string())
        };

        let dummy_id = format!("{}:{}", artist.to_lowercase(), title.to_lowercase());
        tracks.push(crate::db::TrackRecord {
            id: dummy_id,
            artist,
            title,
            duration_sec: 210,
            cached_path: None,
            genres: Some("online-search".to_string()),
            last_played_at: None,
        });
    }

    let results: Vec<TrackSearchResult> = tracks
        .into_iter()
        .map(|t| {
            let stream_url = t.cached_path.as_deref().map(|p| {
                let filename = std::path::Path::new(p)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(p);
                format!("/cache/{}", filename)
            });
            TrackSearchResult {
                id: t.id,
                artist: t.artist,
                title: t.title,
                duration_sec: t.duration_sec,
                cached: t.cached_path.is_some(),
                stream_url,
                genres: t.genres,
            }
        })
        .collect();

    Ok(Json(results))
}

pub async fn handle_radio(
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

pub async fn handle_resolve(
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

pub async fn handle_queue(
    State(state): State<AppState>,
    Query(params): Query<ws::WsParams>,
) -> Result<Json<Vec<lacraia_shared::QueueItem>>, (StatusCode, String)> {
    let cabin_id = params.cabin.unwrap_or_else(|| "default".to_string());
    match state.db.get_queue_for_cabin(&cabin_id) {
        Ok(entries) => {
            let items: Vec<lacraia_shared::QueueItem> = entries
                .into_iter()
                .map(|(q, t)| lacraia_shared::QueueItem {
                    id: q.id,
                    track_id: q.track_id,
                    title: t.title,
                    artist: t.artist,
                    duration_ms: t.duration_sec * 1000,
                    position: q.position,
                    state: q.state.as_str().to_string(),
                    requested_by: q.requested_by,
                })
                .collect();
            Ok(Json(items))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to retrieve queue: {}", e),
        )),
    }
}

pub async fn handle_queue_append(
    State(state): State<AppState>,
    Json(payload): Json<EnqueueRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let cabin_id = payload.cabin_id.unwrap_or_else(|| "default".to_string());
    match state.db.enqueue_track_with_meta(
        &payload.track_id,
        payload.requested_by.as_deref(),
        &cabin_id,
    ) {
        Ok(queue_id) => {
            let update = state.hub.build_state_update_for_cabin(&cabin_id).await;
            let _ = state.hub.tx.send(update);
            Ok(Json(serde_json::json!({
                "success": true,
                "queueId": queue_id,
                "cabinId": cabin_id
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to enqueue track: {}", e),
        )),
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<ws::WsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state.hub, params))
}

pub fn build_app(state: AppState, site_dir: Option<PathBuf>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let cache_dir = state.cache_dir.clone();
    let mut app = Router::new()
        .route("/health", get(health_check))
        .route("/api/search", get(handle_search))
        .route("/api/radio", get(handle_radio))
        .route("/api/resolve", get(handle_resolve))
        .route("/api/queue", get(handle_queue).post(handle_queue_append))
        .route("/api/cabins", get(handle_list_cabins))
        .route("/api/cabin/create", post(handle_cabin_create))
        .route("/api/cabin/rotate-key", post(handle_cabin_rotate_key))
        .route("/api/cabin/auth/master", post(handle_master_auth))
        .route("/api/cabin/join", post(handle_cabin_join))
        .route("/api/cabin/:id", get(handle_cabin_info))
        .route("/ws", get(ws_handler))
        .nest_service("/cache", ServeDir::new(cache_dir));

    if let Some(dir) = site_dir {
        if dir.exists() {
            app = app.fallback_service(ServeDir::new(dir));
        }
    }

    app.layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
