use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use lacraia_core::{
    build_app, AppState, CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub,
    QueueDaemon, TrackRecord,
};
use lacraia_shared::{CommandAction, WsMessage};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn setup_test_context() -> (Database, Arc<PlaybackHub>, axum::Router) {
    let db = Database::open_in_memory().unwrap();
    db.seed_defaults_if_empty().unwrap();

    let cache_dir = tempfile::tempdir().unwrap().path().to_path_buf();
    let (tx, _rx) = broadcast::channel::<WsMessage>(100);
    let hub = Arc::new(PlaybackHub::new(db.clone(), tx));
    let similarity = Arc::new(CompositeSimilarityEngine::new(None, db.clone()));
    let resolver = Arc::new(MediaResolver::new(cache_dir.clone(), db.clone()));
    let daemon = Arc::new(QueueDaemon::new(
        db.clone(),
        resolver.clone(),
        similarity.clone(),
        hub.clone(),
        5,
        Duration::from_secs(3),
    ));

    let state = AppState {
        db: db.clone(),
        similarity,
        resolver,
        hub: hub.clone(),
        cache_dir,
        daemon: Some(daemon),
    };

    let app = build_app(state, None);
    (db, hub, app)
}

#[tokio::test]
async fn test_db_clear_queue() {
    let (db, _hub, _app) = setup_test_context();
    let _ = db.create_cabin("cab1", "Cabine 1", "pilot1", "pass1");

    for i in 1..=3 {
        let trk = TrackRecord {
            id: format!("trk_clear_{}", i),
            title: format!("Música {}", i),
            artist: "Artista".into(),
            duration_sec: 180,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&trk).unwrap();
        db.enqueue_track_with_meta(&trk.id, Some("User"), "cab1")
            .unwrap();
    }

    let q_before = db.get_queue_for_cabin("cab1").unwrap();
    assert_eq!(q_before.len(), 3);

    let deleted = db.clear_queue_for_cabin("cab1").unwrap();
    assert_eq!(deleted, 3);

    let q_after = db.get_queue_for_cabin("cab1").unwrap();
    assert_eq!(q_after.len(), 0);
}

#[tokio::test]
async fn test_api_clear_queue_endpoint() {
    let (db, _hub, app) = setup_test_context();
    let _ = db.create_cabin("cab_clear", "Cabine Clear", "pilot_clear", "pass_key");

    let trk = TrackRecord {
        id: "trk_api_clear".into(),
        title: "Som de Teste".into(),
        artist: "DJ Teste".into(),
        duration_sec: 210,
        cached_path: None,
        genres: None,
        last_played_at: None,
    };
    db.upsert_track(&trk).unwrap();
    db.enqueue_track_with_meta(&trk.id, Some("User"), "cab_clear")
        .unwrap();

    let q_before = db.get_queue_for_cabin("cab_clear").unwrap();
    assert_eq!(q_before.len(), 1);

    // POST /api/queue/clear
    let req = Request::builder()
        .method("POST")
        .uri("/api/queue/clear")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"cabinId":"cab_clear"}"#))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let q_after = db.get_queue_for_cabin("cab_clear").unwrap();
    assert_eq!(q_after.len(), 0);
}

#[tokio::test]
async fn test_hub_dispatch_queue_clear() {
    let (db, hub, _app) = setup_test_context();

    let trk = TrackRecord {
        id: "trk_hub_clear".into(),
        title: "Som Teste".into(),
        artist: "DJ Som".into(),
        duration_sec: 200,
        cached_path: None,
        genres: None,
        last_played_at: None,
    };
    db.upsert_track(&trk).unwrap();
    db.enqueue_track_with_meta(&trk.id, Some("User"), "default")
        .unwrap();

    let q_before = db.get_queue_for_cabin("default").unwrap();
    assert_eq!(q_before.len(), 1);

    // Dispatch QueueClear command
    hub.dispatch_command_with_role(CommandAction::QueueClear, "default", None)
        .await
        .unwrap();

    let q_after = db.get_queue_for_cabin("default").unwrap();
    assert_eq!(q_after.len(), 0);
}
