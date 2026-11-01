use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use lacraia_core::{
    build_app, AppState, CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub,
    QueueDaemon, TrackRecord,
};
use lacraia_shared::WsMessage;
use serde_json::Value;
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
async fn test_van_permanent_hash_and_pin_rotation() {
    let (db, _hub, _app) = setup_test_context();

    // 1. Criar cabine com hash permanente da van
    db.create_cabin("van-furacao", "Van Furacão", "tok_pilot", "pass_key")
        .unwrap();

    let cabin = db.get_cabin("van-furacao").unwrap().unwrap();
    assert!(
        !cabin.van_hash.is_empty(),
        "Cabine deve possuir um van_hash criptográfico fixo para o QR Code"
    );
    assert_eq!(
        cabin.pin.len(),
        3,
        "PIN inicial da van deve possuir exatamente 3 dígitos"
    );

    // 2. Busca de cabine pelo van_hash
    let found = db.get_cabin_by_van_hash(&cabin.van_hash).unwrap().unwrap();
    assert_eq!(found.id, "van-furacao");

    // 3. Verificação de PIN
    assert!(
        db.verify_cabin_pin("van-furacao", &cabin.pin).unwrap(),
        "PIN correto deve validar com sucesso"
    );
    assert!(
        !db.verify_cabin_pin("van-furacao", "000").unwrap() || cabin.pin == "000",
        "PIN incorreto deve falhar na validação"
    );

    // 4. Rotação do PIN pelo motorista
    let old_pin = cabin.pin.clone();
    let new_pin = db.rotate_cabin_pin("van-furacao").unwrap();
    assert_ne!(
        old_pin, new_pin,
        "Novo PIN deve ser diferente do PIN anterior"
    );
    assert!(
        db.verify_cabin_pin("van-furacao", &new_pin).unwrap(),
        "Novo PIN deve ser válido após rotação"
    );
    assert!(
        !db.verify_cabin_pin("van-furacao", &old_pin).unwrap(),
        "PIN antigo deve ser invalidado imediatamente"
    );
}

#[tokio::test]
async fn test_bouncer_approval_flow() {
    let (db, _hub, _app) = setup_test_context();

    // 1. Criar cabine e cadastrar faixa
    db.create_cabin("van-furacao", "Van Furacão", "tok_pilot", "pass_key")
        .unwrap();
    db.upsert_track(&TrackRecord {
        id: "trk_reggae".into(),
        artist: "Tribo de Jah".into(),
        title: "Reggae na Estrada".into(),
        duration_sec: 210,
        cached_path: None,
        genres: None,
        last_played_at: None,
    })
    .unwrap();

    // 2. Por padrão, o modo porteiro está desligado
    assert!(
        !db.is_bouncer_enabled("van-furacao").unwrap(),
        "Modo porteiro deve iniciar desligado por padrão"
    );

    // 3. O motorista ativa o modo porteiro
    db.set_bouncer_mode("van-furacao", true).unwrap();
    assert!(
        db.is_bouncer_enabled("van-furacao").unwrap(),
        "Modo porteiro deve estar ativo"
    );

    // 4. Novo passageiro tenta embarcar sob o modo porteiro ativo
    let psg = db
        .add_passenger("van-furacao", "Estranho", Some("iPhone"), "tok_estranho")
        .unwrap();

    // O passageiro não está aprovado ainda
    assert!(
        !db.is_passenger_approved(&psg.id).unwrap(),
        "Novo passageiro com porteiro ativo deve aguardar aprovação"
    );

    // Tentativa de enfileirar deve falhar por falta de aprovação
    let err = db
        .enqueue_track_with_quota("trk_reggae", Some(&psg.name), "van-furacao", 3)
        .unwrap_err();
    assert!(
        err.to_string().contains("porteiro") || err.to_string().contains("aprovado"),
        "Enfileiramento de passageiro não-aprovado deve falhar: {}",
        err
    );

    // 5. Motorista aprova o passageiro com 1 toque
    db.approve_passenger(&psg.id, true).unwrap();
    assert!(
        db.is_passenger_approved(&psg.id).unwrap(),
        "Passageiro deve constar como aprovado após liberação do motorista"
    );

    // 6. Agora o enfileiramento funciona normalmente
    let q_id = db
        .enqueue_track_with_quota("trk_reggae", Some(&psg.name), "van-furacao", 3)
        .unwrap();
    assert!(q_id > 0);
}

#[tokio::test]
async fn test_van_security_api_endpoints() {
    let (db, _hub, app) = setup_test_context();
    let master_key = db.get_master_key().unwrap();

    // 1. Criar cabine via POST /api/cabin/create
    let create_payload = serde_json::json!({
        "id": "furacao-2000",
        "name": "Van Furacão 2000",
        "masterKey": master_key
    });
    let req = Request::builder()
        .uri("/api/cabin/create")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&create_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: Value = serde_json::from_slice(&bytes).unwrap();
    let van_hash = created["vanHash"].as_str().unwrap().to_string();
    let initial_pin = created["pin"].as_str().unwrap().to_string();
    let pilot_token = created["pilotToken"].as_str().unwrap().to_string();
    let passenger_key = created["passengerKey"].as_str().unwrap().to_string();
    assert_eq!(initial_pin.len(), 3);

    // 2. Resolver van via van_hash em GET /api/cabin/by-van/:van_hash
    let req = Request::builder()
        .uri(format!("/api/cabin/by-van/{}", van_hash))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let resolved: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(resolved["id"], "furacao-2000");
    assert_eq!(resolved["bouncerEnabled"], false);
    assert!(
        resolved.get("pin").is_none() || resolved["pin"].is_null(),
        "Endpoint público de passageiro por van_hash NÃO deve vazar o PIN!"
    );

    // 3. Validar PIN em POST /api/cabin/verify-pin
    let verify_payload = serde_json::json!({
        "cabinId": "furacao-2000",
        "pin": initial_pin
    });
    let req = Request::builder()
        .uri("/api/cabin/verify-pin")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&verify_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["valid"], true);

    // 4. Rotacionar PIN em POST /api/cabin/rotate-pin (somente piloto ou master)
    let rotate_payload = serde_json::json!({
        "cabinId": "furacao-2000",
        "token": pilot_token
    });
    let req = Request::builder()
        .uri("/api/cabin/rotate-pin")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&rotate_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let rotated: Value = serde_json::from_slice(&bytes).unwrap();
    let new_pin = rotated["pin"].as_str().unwrap().to_string();
    assert_ne!(new_pin, initial_pin);

    // 5. Ativar Modo Porteiro em POST /api/cabin/bouncer
    let bouncer_payload = serde_json::json!({
        "cabinId": "furacao-2000",
        "token": pilot_token,
        "enabled": true
    });
    let req = Request::builder()
        .uri("/api/cabin/bouncer")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&bouncer_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. Passageiro entra na cabine via POST /api/cabin/join
    let join_payload = serde_json::json!({
        "cabinId": "furacao-2000",
        "name": "Passageiro Suspeito",
        "key": passenger_key
    });
    let req = Request::builder()
        .uri("/api/cabin/join")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&join_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let joined: Value = serde_json::from_slice(&bytes).unwrap();
    let passenger_id = joined["passengerId"].as_str().unwrap().to_string();

    // 7. Passageiro tenta enfileirar faixa antes da aprovação -> 403 FORBIDDEN
    let enqueue_payload = serde_json::json!({
        "trackId": "online:track123",
        "cabinId": "furacao-2000",
        "requestedBy": "Passageiro Suspeito"
    });
    let req = Request::builder()
        .uri("/api/queue")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&enqueue_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Passageiro não aprovado pelo porteiro deve receber 403 Forbidden"
    );

    // 8. Piloto aprova passageiro via POST /api/cabin/approve-passenger
    let approve_payload = serde_json::json!({
        "cabinId": "furacao-2000",
        "token": pilot_token,
        "passengerId": passenger_id,
        "approved": true
    });
    let req = Request::builder()
        .uri("/api/cabin/approve-passenger")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&approve_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 9. Passageiro tenta enfileirar novamente -> Sucesso 200 OK
    let req = Request::builder()
        .uri("/api/queue")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&enqueue_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
