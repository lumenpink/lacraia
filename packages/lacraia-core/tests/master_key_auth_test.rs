use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use lacraia_core::{
    build_app, AppState, CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub,
};
use lacraia_shared::WsMessage;
use serde_json::Value;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::sync::broadcast;
use tower::ServiceExt;

#[tokio::test]
async fn test_master_key_generation_and_persistence() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    let key1 = {
        let db = Database::open(db_path).expect("open database first time");
        let mk = db.get_master_key().expect("get master key");
        assert!(
            mk.starts_with("mst_"),
            "Master key deve iniciar com prefixo mst_"
        );
        assert!(
            mk.len() >= 20,
            "Master key deve ter entropia suficiente (>= 20 caracteres)"
        );
        mk
    };

    let key2 = {
        let db = Database::open(db_path).expect("open database second time");
        db.get_master_key().expect("get master key on reopen")
    };

    assert_eq!(
        key1, key2,
        "A chave master deve persistir de forma consistente entre reinicializações do banco"
    );
}

#[tokio::test]
async fn test_config_endpoints_master_key_protection() {
    let tmp = NamedTempFile::new().unwrap();
    let db = Database::open(tmp.path().to_str().unwrap()).unwrap();
    let active_master = db.get_master_key().unwrap();

    let cache_dir = std::env::temp_dir().join(format!("lacraia_test_mk_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&cache_dir);

    let similarity = Arc::new(CompositeSimilarityEngine::new(None, db.clone()));
    let resolver = Arc::new(MediaResolver::new(cache_dir.clone(), db.clone()));
    let (tx, _rx) = broadcast::channel::<WsMessage>(16);
    let hub = Arc::new(PlaybackHub::new(db.clone(), tx));

    let app_state = AppState {
        db: db.clone(),
        similarity,
        resolver,
        hub,
        cache_dir: cache_dir.clone(),
        daemon: None,
    };
    let app = build_app(app_state, None);

    // 1. GET /api/config deve retornar a chave master no JSON
    let req = Request::builder()
        .uri("/api/config")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(
        json.get("masterKey").and_then(|v| v.as_str()),
        Some(active_master.as_str()),
        "GET /api/config deve disponibilizar a chave master gerada"
    );

    // 2. POST /api/config SEM chave master deve ser rejeitado com 401
    let unauth_payload = serde_json::json!({
        "targetQueueDurationSec": 3600,
        "targetCacheDurationSec": 1800,
        "maxDiskCacheMb": 512
    });
    let req = Request::builder()
        .uri("/api/config")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&unauth_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Tentativa de salvar configurações sem chave master deve retornar 401"
    );

    // 3. POST /api/config com chave master INVÁLIDA deve ser rejeitado com 401
    let req = Request::builder()
        .uri("/api/config")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Master-Key", "chave_errada_123")
        .body(Body::from(serde_json::to_vec(&unauth_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Tentativa de salvar configurações com chave incorreta deve retornar 401"
    );

    // 4. POST /api/config com chave master VÁLIDA via header deve ter sucesso 200
    let valid_payload = serde_json::json!({
        "targetQueueDurationSec": 7200,
        "targetCacheDurationSec": 3600,
        "maxDiskCacheMb": 1024
    });
    let req = Request::builder()
        .uri("/api/config")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Master-Key", &active_master)
        .body(Body::from(serde_json::to_vec(&valid_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "POST /api/config com X-Master-Key válida deve ser aprovado"
    );

    // 5. POST /api/config com chave master VÁLIDA embutida no corpo do JSON deve ter sucesso 200
    let valid_body_payload = serde_json::json!({
        "targetQueueDurationSec": 7200,
        "targetCacheDurationSec": 3600,
        "maxDiskCacheMb": 1024,
        "masterKey": active_master
    });
    let req = Request::builder()
        .uri("/api/config")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&valid_body_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "POST /api/config com masterKey no corpo deve ser aprovado"
    );
}

#[tokio::test]
async fn test_mutation_resistance_master_key() {
    let tmp = NamedTempFile::new().unwrap();
    let db = Database::open(tmp.path().to_str().unwrap()).unwrap();
    let real_master = db.get_master_key().unwrap();

    // 1. Chave vazia ou somente espaços
    assert!(!lacraia_core::is_valid_master("", &real_master));
    assert!(!lacraia_core::is_valid_master("   ", &real_master));

    // 2. Chave parcial truncada (prefix match não pode passar)
    let partial = &real_master[..real_master.len() - 1];
    assert!(!lacraia_core::is_valid_master(partial, &real_master));

    // 3. Chave com caractere extra adicionado
    let extended = format!("{}_extra", real_master);
    assert!(!lacraia_core::is_valid_master(&extended, &real_master));

    // 4. Concorrência: leituras simultâneas devem retornar a mesma chave
    let mut handles = Vec::new();
    for _ in 0..8 {
        let db_clone = db.clone();
        let expected = real_master.clone();
        handles.push(tokio::spawn(async move {
            let key = db_clone.get_master_key().unwrap();
            assert_eq!(key, expected);
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
}
