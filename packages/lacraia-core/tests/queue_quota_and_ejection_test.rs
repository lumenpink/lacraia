use lacraia_core::{
    build_app, AppState, CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub,
    QueueDaemon, TrackRecord,
};
use lacraia_shared::{CommandAction, Role, WsMessage};
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
async fn test_passenger_queue_quota_limit() {
    let (db, _hub, _app) = setup_test_context();

    // 1. Criar cabine e registrar passageiro
    db.create_cabin("van-furacao", "Van Furacão", "tok_pilot", "pass_key")
        .unwrap();
    let psg = db
        .add_passenger("van-furacao", "Carluxo", Some("iPhone"), "tok_carluxo")
        .unwrap();

    // 2. Criar 4 faixas no catálogo
    for i in 1..=4 {
        db.upsert_track(&TrackRecord {
            id: format!("trk_{}", i),
            artist: "Artista Teste".into(),
            title: format!("Música {}", i),
            duration_sec: 180,
            cached_path: None,
            genres: None,
            last_played_at: None,
        })
        .unwrap();
    }

    // 3. Passageiro enfileira 3 faixas com sucesso (limite máximo permitido)
    assert!(db
        .enqueue_track_with_quota("trk_1", Some(&psg.name), "van-furacao", 3)
        .is_ok());
    assert!(db
        .enqueue_track_with_quota("trk_2", Some(&psg.name), "van-furacao", 3)
        .is_ok());
    assert!(db
        .enqueue_track_with_quota("trk_3", Some(&psg.name), "van-furacao", 3)
        .is_ok());

    // 4. Tentativa da 4ª faixa pelo mesmo passageiro deve falhar com erro de cota
    let err = db
        .enqueue_track_with_quota("trk_4", Some(&psg.name), "van-furacao", 3)
        .unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("Cota de fila excedida") || err_str.contains("máximo"),
        "A 4ª faixa deve ser rejeitada pela cota por passageiro: {}",
        err_str
    );

    // 5. Outro passageiro na mesma cabine ainda deve conseguir enfileirar normalmente
    let psg2 = db
        .add_passenger("van-furacao", "Marielle", Some("Android"), "tok_marielle")
        .unwrap();
    assert!(
        db.enqueue_track_with_quota("trk_4", Some(&psg2.name), "van-furacao", 3)
            .is_ok(),
        "Outro passageiro não deve ser bloqueado pela cota de terceiros"
    );
}

#[tokio::test]
async fn test_ejection_purges_queue_and_records_history() {
    let (db, hub, _app) = setup_test_context();

    // 1. Criar cabine e registrar 2 passageiros
    db.create_cabin("van-furacao", "Van Furacão", "tok_pilot", "pass_key")
        .unwrap();
    let bad_boy = db
        .add_passenger("van-furacao", "Invasor", Some("iPhone"), "tok_invasor")
        .unwrap();
    let loyal = db
        .add_passenger("van-furacao", "Fiel", Some("Android"), "tok_fiel")
        .unwrap();

    for i in 1..=3 {
        db.upsert_track(&TrackRecord {
            id: format!("trk_sabotagem_{}", i),
            artist: "Invasor".into(),
            title: format!("Sabotagem {}", i),
            duration_sec: 120,
            cached_path: None,
            genres: None,
            last_played_at: None,
        })
        .unwrap();
    }
    db.upsert_track(&TrackRecord {
        id: "trk_legitima".into(),
        artist: "Fiel".into(),
        title: "Funk do Bom".into(),
        duration_sec: 200,
        cached_path: None,
        genres: None,
        last_played_at: None,
    })
    .unwrap();

    // 2. Invasor enfileira 2 músicas, e o Fiel enfileira 1
    db.enqueue_track_with_quota("trk_sabotagem_1", Some(&bad_boy.name), "van-furacao", 3)
        .unwrap();
    db.enqueue_track_with_quota("trk_sabotagem_2", Some(&bad_boy.name), "van-furacao", 3)
        .unwrap();
    db.enqueue_track_with_quota("trk_legitima", Some(&loyal.name), "van-furacao", 3)
        .unwrap();

    let q_before = db.get_queue_for_cabin("van-furacao").unwrap();
    assert_eq!(q_before.len(), 3);

    // 3. Piloto aciona a alavanca de ejeção do Invasor via PlaybackHub
    hub.dispatch_command_with_role(
        CommandAction::EjectPassenger {
            passenger_id: bad_boy.id.clone(),
        },
        "van-furacao",
        Some(Role::Pilot),
    )
    .await
    .unwrap();

    // 4. As músicas do invasor devem ter sido PURGADAS da fila
    let q_after = db.get_queue_for_cabin("van-furacao").unwrap();
    assert_eq!(
        q_after.len(),
        1,
        "Apenas a música legítima deve restar na fila"
    );
    assert_eq!(q_after[0].1.id, "trk_legitima");

    // 5. O histórico do SQLite deve registrar as faixas purgadas com status 'ejected'
    let history = db
        .get_playback_history(Some("van-furacao"), Some(&bad_boy.name), 10)
        .unwrap();
    assert_eq!(
        history.len(),
        2,
        "O histórico deve registrar as 2 músicas purgadas do passageiro ejetado"
    );
    assert!(
        history.iter().all(|h| h.status == "ejected"),
        "Todas as músicas purgadas devem possuir o status 'ejected' no histórico"
    );
}
