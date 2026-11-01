#[cfg(feature = "desktop")]
mod tests {
    use lacraia_app::embedded_core::{EmbeddedCoreHandle, EmbeddedCoreManager};

    #[tokio::test]
    async fn test_embedded_core_lifecycle_and_http_endpoints() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test_lacraia.sqlite");
        let cache_dir = tmp.path().join("cache");

        // 1. Iniciar servidor embutido na porta efêmera 0
        let mut handle = EmbeddedCoreManager::start_embedded_server(
            0,
            Some(&db_path),
            Some(&cache_dir),
        )
        .await
        .expect("iniciar servidor embutido");

        let port = handle.port;
        assert!(port > 0, "Porta alocada deve ser maior que 0");

        let base_url = format!("http://127.0.0.1:{}", port);

        // 2. Provedor de vitalidade (healthcheck)
        let alive = EmbeddedCoreManager::is_server_alive(&base_url).await;
        assert!(alive, "Servidor embutido deve responder ao probe de vitalidade");

        // 3. Consulta à API de fila do servidor embutido
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/api/queue?cabin=default", base_url))
            .send()
            .await
            .expect("requisicao /api/queue");

        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.expect("json da fila");
        assert!(body.is_array(), "Fila inicial deve ser um array");

        // 4. Desligamento gracioso
        handle.stop();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let alive_after = EmbeddedCoreManager::is_server_alive(&base_url).await;
        assert!(
            !alive_after,
            "Servidor embutido deve estar inativo após parada graciosa"
        );
    }

    #[tokio::test]
    async fn test_is_server_alive_probe_returns_false_for_dead_port() {
        let dead_url = "http://127.0.0.1:59999";
        let alive = EmbeddedCoreManager::is_server_alive(dead_url).await;
        assert!(!alive, "Probe em porta morta deve retornar false");
    }

    #[tokio::test]
    async fn test_embedded_core_port_occupied_fallback() {
        // Bloquear temporariamente uma porta fixa
        let blocker = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let occupied_port = blocker.local_addr().unwrap().port();

        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test_occ.sqlite");
        let cache_dir = tmp.path().join("cache_occ");

        // Tentar iniciar na porta ocupada: deve buscar fallback gracioso para a próxima porta
        let mut handle = EmbeddedCoreManager::start_embedded_server(
            occupied_port,
            Some(&db_path),
            Some(&cache_dir),
        )
        .await
        .expect("fallback para proxima porta disponivel");

        assert_ne!(
            handle.port, occupied_port,
            "Servidor embutido deve ter escolhido uma porta alternativa livre"
        );
        handle.stop();
    }
}
