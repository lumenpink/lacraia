use lacraia_core::{
    build_app, AppState, CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub,
    QueueDaemon, TrackRecord,
};
use lacraia_shared::WsMessage;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

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
