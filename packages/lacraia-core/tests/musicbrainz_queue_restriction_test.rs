use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use lacraia_core::{
    build_app, AppState, CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub,
};
use lacraia_shared::WsMessage;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn setup_test_context() -> (Database, axum::Router) {
    let db = Database::open_in_memory().unwrap();
    db.seed_defaults_if_empty().unwrap();

    let cache_dir = tempfile::tempdir().unwrap().path().to_path_buf();
    let (tx, _rx) = broadcast::channel::<WsMessage>(16);
    let hub = Arc::new(PlaybackHub::new(db.clone(), tx));
    let similarity = Arc::new(CompositeSimilarityEngine::new(None, db.clone()));
    let resolver = Arc::new(MediaResolver::new(cache_dir.clone(), db.clone()));

    let state = AppState {
        db: db.clone(),
        similarity,
        resolver,
        hub,
        cache_dir,
        daemon: None,
    };

    let app = build_app(state, None);
    (db, app)
}

#[tokio::test]
async fn test_reject_arbitrary_free_text_in_queue_append() {
    let (_db, app) = setup_test_context();

    // 1. Tentar enfileirar texto livre arbitrário sem validação do MusicBrainz
    let payload = serde_json::json!({
        "track_id": "musica_inventada_sem_metadados",
        "cabin_id": "default",
        "action": "QUEUE_APPEND"
    });

    let req = Request::builder()
        .uri("/api/queue")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();

    // DEVE REJEITAR com 400 Bad Request
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "Payloads sem identificadores do MusicBrainz devem ser rejeitados com 400"
    );

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let err_msg = String::from_utf8_lossy(&bytes);
    assert!(
        err_msg.contains("MUSICBRAINZ_REQUIRED"),
        "Mensagem de erro deve conter o código MUSICBRAINZ_REQUIRED, obtido: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_accept_musicbrainz_validated_track_in_queue_append() {
    let (_db, app) = setup_test_context();

    // 2. Enfileirar faixa com identificador canônico do MusicBrainz
    let payload = serde_json::json!({
        "track_id": "mb:5b719463-e578-43bb-a320-c75c8d002f23",
        "cabin_id": "default",
        "action": "QUEUE_APPEND"
    });

    let req = Request::builder()
        .uri("/api/queue")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Faixas identificadas pelo MusicBrainz (mb:UUID) devem ser aceitas com sucesso"
    );
}
