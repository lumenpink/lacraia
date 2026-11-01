use lacraia_core::{
    build_app, AppState, CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub,
    QueueDaemon,
};
use lacraia_shared::WsMessage;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, oneshot};

pub struct EmbeddedCoreHandle {
    pub port: u16,
    pub shutdown_tx: Option<oneshot::Sender<()>>,
}

impl EmbeddedCoreHandle {
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

pub struct EmbeddedCoreManager;

impl EmbeddedCoreManager {
    /// Checks whether an external or already running server is alive at the specified URL.
    pub async fn is_server_alive(url: &str) -> bool {
        let health_url = format!("{}/health", url.trim_end_matches('/'));
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_millis(300))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        match client.get(&health_url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Starts an autonomous embedded Lacraia Core daemon on the preferred port (or fallback if occupied).
    pub async fn start_embedded_server(
        preferred_port: u16,
        db_path: Option<&Path>,
        cache_dir: Option<&Path>,
    ) -> Result<EmbeddedCoreHandle, Box<dyn std::error::Error + Send + Sync>> {
        // 1. Bind listener with port fallback
        let (listener, actual_port) = Self::bind_listener_with_fallback(preferred_port).await?;

        // 2. Setup SQLite DB
        let db = if let Some(p) = db_path {
            if let Some(parent) = p.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            Database::open(p)?
        } else {
            let default_db = PathBuf::from("./lacraia.sqlite");
            Database::open(default_db)?
        };
        db.seed_defaults_if_empty()?;

        // 3. Setup Cache directory
        let cache_path = if let Some(c) = cache_dir {
            c.to_path_buf()
        } else {
            PathBuf::from("./cache")
        };
        tokio::fs::create_dir_all(&cache_path).await?;

        // 4. Setup engines and daemons
        let similarity = Arc::new(CompositeSimilarityEngine::new(None, db.clone()));
        let resolver = Arc::new(MediaResolver::new(cache_path.clone(), db.clone()));
        let (tx, _rx) = broadcast::channel::<WsMessage>(100);
        let hub = Arc::new(PlaybackHub::new(db.clone(), tx));
        let daemon = Arc::new(QueueDaemon::new(
            db.clone(),
            resolver.clone(),
            similarity.clone(),
            hub.clone(),
            5,
            Duration::from_secs(3),
        ));

        let state = AppState {
            db,
            similarity,
            resolver,
            hub,
            cache_dir: cache_path,
            daemon: Some(daemon),
        };

        // 5. Build router & spawn axum server
        let router = build_app(state, None);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            let server = axum::serve(listener, router).with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });
            if let Err(e) = server.await {
                tracing::warn!("Embedded core server finished with error: {}", e);
            }
        });

        Ok(EmbeddedCoreHandle {
            port: actual_port,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    async fn bind_listener_with_fallback(
        preferred_port: u16,
    ) -> Result<(TcpListener, u16), Box<dyn std::error::Error + Send + Sync>> {
        if preferred_port == 0 {
            let addr = SocketAddr::from(([127, 0, 0, 1], 0));
            let listener = TcpListener::bind(addr).await?;
            let port = listener.local_addr()?.port();
            return Ok((listener, port));
        }

        let mut port_candidate = preferred_port;
        let max_port = preferred_port.saturating_add(50);

        while port_candidate <= max_port {
            let addr = SocketAddr::from(([127, 0, 0, 1], port_candidate));
            match TcpListener::bind(addr).await {
                Ok(listener) => return Ok((listener, port_candidate)),
                Err(_) => {
                    port_candidate += 1;
                }
            }
        }

        // Se todo o intervalo estiver ocupado, busca porta efêmera livre no sistema
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).await?;
        let port = listener.local_addr()?.port();
        Ok((listener, port))
    }
}
