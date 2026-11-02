use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use lacraia_core::{
    build_app, AppState, CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub,
    TrackRecord,
};
use lacraia_shared::{Playlist, PlaylistItem, WsMessage};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn setup_test_context() -> (Database, axum::Router) {
    let db = Database::open_in_memory().unwrap();
    db.seed_defaults_if_empty().unwrap();

    let cache_dir = tempfile::tempdir().unwrap().path().to_path_buf();
    let (tx, _rx) = broadcast::channel::<WsMessage>(100);
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
async fn test_db_playlist_lifecycle_and_text_metadata_only() {
    let (db, _app) = setup_test_context();

    let items = vec![
        PlaylistItem {
            title: "Vai Lacraia".into(),
            artist: "MC Serginho".into(),
            album: Some("Furacão 2000".into()),
            duration_sec: 240,
        },
        PlaylistItem {
            title: "Construção".into(),
            artist: "Chico Buarque".into(),
            album: Some("Construção".into()),
            duration_sec: 258,
        },
    ];

    let created = db
        .create_playlist("Funk & Samba da Periferia", "default", &items)
        .expect("Failed to create playlist");

    assert_eq!(created.name, "Funk & Samba da Periferia");
    assert_eq!(created.cabin_id, "default");
    assert_eq!(created.items.len(), 2);
    assert_eq!(created.items[0].title, "Vai Lacraia");
    assert_eq!(created.items[1].artist, "Chico Buarque");

    // Fetch lists for cabin
    let lists = db.get_playlists_for_cabin("default").unwrap();
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].name, "Funk & Samba da Periferia");

    // Fetch by id
    let fetched = db.get_playlist_by_id(&created.id).unwrap();
    assert!(fetched.is_some());
    let pl = fetched.unwrap();
    assert_eq!(pl.items.len(), 2);

    // Delete playlist
    let deleted = db.delete_playlist(&created.id).unwrap();
    assert!(deleted);
    let after_delete = db.get_playlists_for_cabin("default").unwrap();
    assert!(after_delete.is_empty());
}

#[tokio::test]
async fn test_save_queue_and_load_playlist() {
    let (db, _app) = setup_test_context();

    // Enqueue some tracks
    let trk1 = TrackRecord {
        id: "trk_pl_1".into(),
        title: "Trem das Onze".into(),
        artist: "Demonios da Garoa".into(),
        duration_sec: 180,
        cached_path: None,
        genres: None,
        last_played_at: None,
    };
    let trk2 = TrackRecord {
        id: "trk_pl_2".into(),
        title: "Chega de Saudade".into(),
        artist: "Joao Gilberto".into(),
        duration_sec: 150,
        cached_path: None,
        genres: None,
        last_played_at: None,
    };
    db.upsert_track(&trk1).unwrap();
    db.upsert_track(&trk2).unwrap();

    db.enqueue_track_with_meta(&trk1.id, Some("Motorista"), "default")
        .unwrap();
    db.enqueue_track_with_meta(&trk2.id, Some("Motorista"), "default")
        .unwrap();

    // Save active queue as playlist
    let saved = db
        .save_current_queue_as_playlist("Samba Paulista", "default")
        .expect("Failed to save queue as playlist");
    assert_eq!(saved.name, "Samba Paulista");
    assert_eq!(saved.items.len(), 2);
    assert_eq!(saved.items[0].title, "Trem das Onze");

    // Clear queue
    db.clear_queue_for_cabin("default").unwrap();
    assert_eq!(db.get_queue_for_cabin("default").unwrap().len(), 0);

    // Load saved playlist back into queue
    let loaded_count = db
        .load_playlist_to_queue(&saved.id, "default", Some("ZapImport"))
        .expect("Failed to load playlist to queue");
    assert_eq!(loaded_count, 2);

    let queue = db.get_queue_for_cabin("default").unwrap();
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[0].1.title, "Trem das Onze");
    assert_eq!(queue[1].1.title, "Chega de Saudade");
}

#[tokio::test]
async fn test_api_playlists_endpoints() {
    let (db, app) = setup_test_context();

    // 1. Create playlist via POST /api/playlists
    let body_json = serde_json::json!({
        "name": "Nostalgia 2000",
        "cabinId": "default",
        "items": [
            { "title": "Cerol na Mão", "artist": "Bonde do Tigrão", "durationSec": 190 },
            { "title": "Tô Nem Aí", "artist": "Luka", "durationSec": 215 }
        ]
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/playlists")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body_json).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: Playlist = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(created.name, "Nostalgia 2000");
    assert_eq!(created.items.len(), 2);

    // 2. GET /api/playlists
    let req = Request::builder()
        .method("GET")
        .uri("/api/playlists?cabin_id=default")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let list: Vec<Playlist> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Nostalgia 2000");

    // 3. POST /api/playlists/:id/load
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/playlists/{}/load", created.id))
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"cabinId":"default"}"#))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let q = db.get_queue_for_cabin("default").unwrap();
    assert_eq!(q.len(), 2);
    assert_eq!(q[0].1.title, "Cerol na Mão");

    // 4. DELETE /api/playlists/:id
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/playlists/{}", created.id))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let remaining = db.get_playlists_for_cabin("default").unwrap();
    assert!(remaining.is_empty());
}
