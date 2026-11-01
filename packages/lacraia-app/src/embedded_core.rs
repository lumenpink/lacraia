use std::path::Path;

pub struct EmbeddedCoreHandle {
    pub port: u16,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
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
    pub async fn is_server_alive(_url: &str) -> bool {
        false
    }

    pub async fn start_embedded_server(
        _preferred_port: u16,
        _db_path: Option<&Path>,
        _cache_dir: Option<&Path>,
    ) -> Result<EmbeddedCoreHandle, Box<dyn std::error::Error + Send + Sync>> {
        Err("servidor embutido nao implementado".into())
    }
}
