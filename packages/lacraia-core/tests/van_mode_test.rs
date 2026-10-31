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

#[test]
fn test_default_settings_is_monousuario() {
    let db = Database::open_in_memory().expect("abrir banco em memória");
    let settings = db
        .get_system_settings()
        .expect("obter configurações padrão");

    assert!(
        !settings.van_mode_enabled,
        "O perfil padrão da aplicação DEVE ser monousuário (van_mode_enabled = false)"
    );
}

#[test]
fn test_van_mode_toggle_persistence() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    // 1. Inicializar e ativar o modo van
    {
        let db = Database::open(db_path).unwrap();
        let mut settings = db.get_system_settings().unwrap();
        assert!(!settings.van_mode_enabled);

        settings.van_mode_enabled = true;
        db.update_system_settings(&settings).unwrap();

        let reloaded = db.get_system_settings().unwrap();
        assert!(
            reloaded.van_mode_enabled,
            "Modo van ativado deve persistir no banco"
        );
    }

    // 2. Reabrir o banco e verificar persistência entre sessões
    {
        let db = Database::open(db_path).unwrap();
        let settings = db.get_system_settings().unwrap();
        assert!(
            settings.van_mode_enabled,
            "Modo van ativado deve continuar persistido após reabertura"
        );

        // Desativar de volta para monousuário
        let mut updated = settings;
        updated.van_mode_enabled = false;
        db.update_system_settings(&updated).unwrap();

        let reloaded = db.get_system_settings().unwrap();
        assert!(
            !reloaded.van_mode_enabled,
            "Desativação do modo van deve persistir"
        );
    }
}

#[tokio::test]
async fn test_van_mode_api_endpoints() {
    let db = Database::open_in_memory().unwrap();
    let master_key = db.get_master_key().unwrap();
    let cache_dir = std::env::temp_dir().join(format!("lacraia_van_test_{}", uuid::Uuid::new_v4()));
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

    // 1. GET /api/config: por padrão vanModeEnabled é false
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
        json.get("vanModeEnabled"),
        Some(&Value::Bool(false)),
        "GET /api/config deve retornar vanModeEnabled = false por padrão"
    );

    // 2. POST /api/config: ativar vanModeEnabled = true
    let payload = serde_json::json!({
        "targetQueueDurationSec": 7200,
        "targetCacheDurationSec": 3600,
        "maxDiskCacheMb": 1024,
        "vanModeEnabled": true
    });
    let req = Request::builder()
        .uri("/api/config")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Master-Key", &master_key)
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. GET /api/config: deve refletir vanModeEnabled = true
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
        json.get("vanModeEnabled"),
        Some(&Value::Bool(true)),
        "GET /api/config deve refletir vanModeEnabled = true após atualização"
    );
}

#[test]
fn test_html_van_mode_contract() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html =
        std::fs::read_to_string(root.join("site/app/index.html")).expect("ler site/app/index.html");

    assert!(
        html.contains("check-van-mode"),
        "HTML deve conter o controle toggle 'check-van-mode'"
    );
    assert!(
        html.contains("applyVanMode"),
        "HTML deve implementar a função applyVanMode para alternância reativa"
    );
    assert!(
        html.contains("vanModeEnabled"),
        "HTML deve sincronizar com o parâmetro vanModeEnabled"
    );

    // Validar presença de todos os IDs de elementos controlados pelo Modo Van
    let elementos_van = [
        "cabin-badge",
        "btn-cabin-access",
        "door-badge",
        "role-badge",
        "tab-btn-deck-03",
        "deck-03",
    ];

    for id in elementos_van {
        assert!(
            html.contains(id),
            "Elemento coletivo '{id}' deve existir no template HTML para ser controlado pelo modo van"
        );
    }
}
