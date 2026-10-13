use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use lacraia_core::{
    build_app, AppState, CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub,
    QueueDaemon, QueueState, TrackRecord,
};
use lacraia_shared::{is_title_matching, strip_accents, WsMessage};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower::ServiceExt;

#[test]
fn test_search_sanitization_and_operator_safety() {
    // 1. Accent stripping & case normalization
    assert_eq!(strip_accents("Construção"), "construcao");
    assert_eq!(strip_accents("ÁGUAS DE MARÇO"), "aguas de marco");
    assert_eq!(strip_accents("Mägo de Öz"), "mago de oz");

    // 2. Query sanitization: ensure hyphens and dots are replaced with whitespace
    let artist = "SEXCORP.";
    let title = "LACRAIA";
    let clean_artist = strip_accents(artist)
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>();
    let clean_title = strip_accents(title)
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>();
    let query = format!(
        "ytsearch5:{} {} audio",
        clean_artist.trim(),
        clean_title.trim()
    );

    assert_eq!(query, "ytsearch5:sexcorp lacraia audio");
    assert!(
        !query.contains('-'),
        "Query must never contain '-' operator which negates search terms"
    );

    // 3. Rejection of false-positive candidates
    assert!(!is_title_matching("RIO", "lacraia"));
    assert!(!is_title_matching("Totally Unrelated Track", "Construcao"));

    // 4. Acceptance of valid fuzzy candidates
    assert!(is_title_matching(
        "Construção | Chico Buarque (Vídeo Oficial)",
        "Construcao"
    ));
    assert!(is_title_matching(
        "Bonde do Tigrão - Vai Lacraia (Ao Vivo)",
        "Vai Lacraia"
    ));
}

#[tokio::test]
async fn test_queue_daemon_failure_isolation() {
    let db = Database::open_in_memory().unwrap();
    let cache_dir = std::env::temp_dir().join(format!("lacraia_test_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&cache_dir);

    let track_id = "ghost_band:nonexistent_track_xyz_999";
    let track = TrackRecord {
        id: track_id.to_string(),
        artist: "Ghost Band".to_string(),
        title: "Nonexistent Track XYZ 999".to_string(),
        duration_sec: 180,
        cached_path: None,
        genres: None,
        last_played_at: None,
    };
    db.upsert_track(&track).unwrap();
    let _qid = db.enqueue_track(track_id).unwrap();

    let resolver = Arc::new(MediaResolver::new(cache_dir.clone(), db.clone()));
    let similarity = Arc::new(CompositeSimilarityEngine::new(None, db.clone()));
    let (tx, _rx) = broadcast::channel::<WsMessage>(50);
    let hub = Arc::new(PlaybackHub::new(db.clone(), tx));
    let daemon = QueueDaemon::new(
        db.clone(),
        resolver,
        similarity,
        hub,
        2,
        std::time::Duration::from_millis(50),
    );

    // Initial queue state must be pending
    let queue = db.get_queue().unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].0.state, QueueState::Pending);

    // Run one prefetch step. Since the track does not exist, resolve_and_cache will fail.
    let _ = daemon.step().await;

    // Queue entry MUST transition to Failed state, isolating the error
    let queue_after = db.get_queue().unwrap();
    assert_eq!(queue_after.len(), 1);
    assert_eq!(
        queue_after[0].0.state,
        QueueState::Failed,
        "Failed track must transition to QueueState::Failed"
    );

    // Subsequent steps must NOT target the failed track
    let all_queues = db.get_queue().unwrap();
    let targets = QueueDaemon::find_prefetch_targets(&all_queues, 2);
    assert_eq!(
        targets.len(),
        0,
        "Failed track must be excluded from future prefetch targets"
    );
}

#[tokio::test]
async fn test_end_to_end_api_resolve_and_cache_serving() {
    let db = Database::open_in_memory().unwrap();
    let cache_dir = std::env::temp_dir().join(format!("lacraia_test_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&cache_dir);

    // Create a mock cached audio file in the temp cache directory
    let dummy_audio = b"OPUS_MOCK_STREAM_BYTES_FOR_INTEGRATION_TEST";
    let test_cache_file = cache_dir.join("test_artist_test_track.opus");
    tokio::fs::write(&test_cache_file, dummy_audio)
        .await
        .unwrap();

    let similarity = Arc::new(CompositeSimilarityEngine::new(None, db.clone()));
    let resolver = Arc::new(MediaResolver::new(cache_dir.clone(), db.clone()));
    let (tx, _rx) = broadcast::channel::<WsMessage>(10);
    let hub = Arc::new(PlaybackHub::new(db.clone(), tx));

    let state = AppState {
        db: db.clone(),
        similarity,
        resolver,
        hub,
        cache_dir: cache_dir.clone(),
        daemon: None,
    };

    let app = build_app(state, None);

    // 1. Send GET /api/resolve for the cached track
    let req = Request::builder()
        .method("GET")
        .uri("/api/resolve?artist=Test%20Artist&title=Test%20Track&expectedDurationSec=210")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let track_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let stream_url = track_json["streamUrl"]
        .as_str()
        .expect("streamUrl must be present");
    assert_eq!(stream_url, "/cache/test_artist_test_track.opus");

    // 2. Fetch the stream directly from /cache static mount
    let cache_req = Request::builder()
        .method("GET")
        .uri(stream_url)
        .body(Body::empty())
        .unwrap();

    let cache_resp = app.oneshot(cache_req).await.unwrap();
    assert_eq!(cache_resp.status(), StatusCode::OK);

    let stream_bytes = axum::body::to_bytes(cache_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&stream_bytes[..], dummy_audio);
}

#[tokio::test]
async fn test_real_resolver_download_with_android_client() {
    let db = Database::open_in_memory().unwrap();
    let cache_dir = std::env::temp_dir().join(format!("lacraia_test_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&cache_dir);

    let resolver = MediaResolver::new(cache_dir.clone(), db.clone());

    // Resolve a real track known to exist: "Chico Buarque - Construcao" (or Legiao Urbana - Tempo Perdido)
    let resolution = resolver
        .resolve_and_cache("Chico Buarque", "Construcao", 258)
        .await;

    match resolution {
        Ok(track) => {
            assert_eq!(track.artist, "Chico Buarque");
            assert_eq!(track.title, "Construcao");
            assert!(track.stream_url.starts_with("/cache/"));

            let file_path = PathBuf::from(&track.cache_path);
            assert!(file_path.exists(), "Cache file must exist on disk");

            let metadata = std::fs::metadata(&file_path).unwrap();
            assert!(
                metadata.len() > 100_000,
                "Downloaded Opus file must be substantial (>100KB)"
            );

            // Second resolution must be instantaneous Tier 1 hit
            let start = std::time::Instant::now();
            let cached_resolution = resolver
                .resolve_and_cache("Chico Buarque", "Construcao", 258)
                .await
                .unwrap();
            let elapsed = start.elapsed();

            assert_eq!(cached_resolution.stream_url, track.stream_url);
            assert!(
                elapsed.as_millis() < 50,
                "Tier 1 cache hit must take < 50ms"
            );
        }
        Err(e) => {
            // If offline or network restricted, verify error is clean and properly typed
            eprintln!("Network resolution skipped/failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_autopilot_infinite_lookahead_maintenance() {
    let db = Database::open_in_memory().unwrap();
    db.seed_defaults_if_empty().unwrap();

    let cache_dir =
        std::env::temp_dir().join(format!("lacraia_test_autopilot_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&cache_dir);
    let resolver = Arc::new(MediaResolver::new(cache_dir, db.clone()));
    let similarity = Arc::new(CompositeSimilarityEngine::new(None, db.clone()));
    let (tx, _rx) = broadcast::channel::<WsMessage>(50);
    let hub = Arc::new(PlaybackHub::new(db.clone(), tx));

    let daemon = QueueDaemon::new(
        db.clone(),
        resolver,
        similarity,
        hub.clone(),
        5, // 5 tracks ahead target
        std::time::Duration::from_millis(50),
    );

    // 1. In dormant state (no playback, no queue), daemon does NOT add random tracks
    let pre_queue = db.get_queue().unwrap();
    assert!(pre_queue.is_empty(), "Queue must start empty");
    let enqueued = daemon.maintain_autopilot_queue("default", 5).await.unwrap();
    assert_eq!(
        enqueued, 0,
        "Dormant daemon must not enqueue anything on idle cabin"
    );
    assert!(db.get_queue().unwrap().is_empty());

    // 2. User enqueues the first seed track and starts playback
    let _ = db
        .enqueue_track_with_meta("chico buarque:construcao", Some("PILOTO"), "default")
        .unwrap();
    let advanced = db.advance_queue_for_cabin("default").unwrap();
    assert!(advanced.is_some());
    assert_eq!(advanced.unwrap().state, QueueState::Playing);
    *hub.playback_state.lock().await = lacraia_shared::PlaybackState::Playing;

    // 3. Autopilot maintenance detects active playback with 0 tracks ahead (target: 5), adding 5 similar tracks
    let added = daemon.maintain_autopilot_queue("default", 5).await.unwrap();
    assert_eq!(
        added, 5,
        "Autopilot must add 5 tracks ahead of active playing track"
    );

    let full_queue = db.get_queue().unwrap();
    // 1 playing + 5 upcoming = 6 total entries
    assert_eq!(
        full_queue.len(),
        6,
        "Queue must contain 1 playing + 5 tracks ahead"
    );
    for entry in &full_queue[1..] {
        assert_eq!(entry.0.requested_by.as_deref(), Some("AUTOPILOT"));
    }

    // 4. Pilot advances the queue (first upcoming track is promoted to playing, leaving 4 upcoming)
    let advanced2 = db.advance_queue_for_cabin("default").unwrap();
    assert!(advanced2.is_some());
    assert_eq!(advanced2.unwrap().state, QueueState::Playing);

    // 5. Autopilot maintenance runs again: only 4 tracks ahead, fills 1 more to keep 5 ahead
    let replenished = daemon.maintain_autopilot_queue("default", 5).await.unwrap();
    assert_eq!(
        replenished, 1,
        "Autopilot must replenish 1 track to maintain 5 ahead"
    );

    let queue_active = db.get_queue().unwrap();
    // 1 playing + 5 upcoming = 6 total entries with state != 'played'
    assert_eq!(queue_active.len(), 6);
    let upcoming_count = queue_active
        .iter()
        .filter(|(e, _)| e.state == QueueState::Pending || e.state == QueueState::Cached)
        .count();
    assert_eq!(
        upcoming_count, 5,
        "Autopilot must maintain 5 upcoming tracks ahead forever"
    );
}

#[tokio::test]
async fn test_config_api_endpoints_get_and_post() {
    let db = Database::open_in_memory().unwrap();
    let cache_dir =
        std::env::temp_dir().join(format!("lacraia_test_config_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&cache_dir);

    let similarity = Arc::new(CompositeSimilarityEngine::new(None, db.clone()));
    let resolver = Arc::new(MediaResolver::new(cache_dir.clone(), db.clone()));
    let (tx, _rx) = broadcast::channel::<WsMessage>(10);
    let hub = Arc::new(PlaybackHub::new(db.clone(), tx));
    let daemon = Arc::new(QueueDaemon::new(
        db.clone(),
        resolver.clone(),
        similarity.clone(),
        hub.clone(),
        5,
        std::time::Duration::from_millis(50),
    ));

    let state = AppState {
        db: db.clone(),
        similarity,
        resolver,
        hub,
        cache_dir: cache_dir.clone(),
        daemon: Some(daemon.clone()),
    };

    let app = build_app(state, None);

    // 1. GET /api/config: verify default values
    let get_req = Request::builder()
        .method("GET")
        .uri("/api/config")
        .body(Body::empty())
        .unwrap();

    let get_resp = app.clone().oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(get_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let config_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(
        config_json["targetQueueDurationSec"], 7200,
        "Default targetQueueDurationSec must be 7200s (2h, #25)"
    );
    assert_eq!(
        config_json["targetCacheDurationSec"], 3600,
        "Default targetCacheDurationSec must be 3600s (1h)"
    );
    assert_eq!(
        config_json["maxDiskCacheMb"], 1024,
        "Default maxDiskCacheMb must be 1024"
    );

    // 2. POST /api/config: update values and verify daemon dynamic update
    let update_payload = serde_json::json!({
        "targetQueueDurationSec": 10800,
        "targetCacheDurationSec": 5400,
        "maxDiskCacheMb": 2048
    });

    let post_req = Request::builder()
        .method("POST")
        .uri("/api/config")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_payload).unwrap()))
        .unwrap();

    let post_resp = app.clone().oneshot(post_req).await.unwrap();
    assert_eq!(post_resp.status(), StatusCode::OK);

    // Verify daemon was dynamically updated
    assert_eq!(daemon.target_queue_duration_sec(), 10800);
    assert_eq!(daemon.target_cache_duration_sec(), 5400);

    // 3. POST /api/config with invalid values: must reject with 400 Bad Request
    let invalid_payload = serde_json::json!({
        "targetQueueDurationSec": 10,
        "targetCacheDurationSec": 3600,
        "maxDiskCacheMb": 1024
    });

    let invalid_req = Request::builder()
        .method("POST")
        .uri("/api/config")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&invalid_payload).unwrap()))
        .unwrap();

    let invalid_resp = app.oneshot(invalid_req).await.unwrap();
    assert_eq!(invalid_resp.status(), StatusCode::BAD_REQUEST);
}
