use lacraia_core::{
    build_app, AppState, CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub,
    QueueDaemon,
};
use lacraia_shared::WsMessage;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::broadcast;
use tracing::info;

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
    db.seed_defaults_if_empty()?;

    let lastfm_key = std::env::var("LASTFM_API_KEY").ok();
    let similarity = Arc::new(CompositeSimilarityEngine::new(lastfm_key, db.clone()));
    let resolver = Arc::new(MediaResolver::new(cache_dir.clone(), db.clone()));

    let (tx, _rx) = broadcast::channel::<WsMessage>(100);
    let hub = Arc::new(PlaybackHub::new(db.clone(), tx));
    hub.clone().start_telemetry_ticker();

    let daemon = Arc::new(QueueDaemon::new(
        db.clone(),
        resolver.clone(),
        similarity.clone(),
        hub.clone(),
        5, // Lookahead window of 5 tracks ahead
        std::time::Duration::from_secs(3),
    ));
    daemon.clone().start();

    let state = AppState {
        db,
        similarity,
        resolver,
        hub,
        cache_dir: cache_dir.clone(),
        daemon: Some(daemon.clone()),
    };

    let site_dir = std::env::var("LACRAIA_SITE_DIR").map(PathBuf::from).ok().or_else(|| {
        let default_dir = PathBuf::from("./site");
        if default_dir.exists() {
            Some(default_dir)
        } else {
            None
        }
    });

    let app = build_app(state, site_dir);

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
