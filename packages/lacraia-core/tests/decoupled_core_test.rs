use lacraia_core::{
    CompositeSimilarityEngine, Database, MediaResolver, PlaybackHub, QueueDaemon, QueueState,
    TrackRecord,
};
use lacraia_shared::WsMessage;
use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::test]
async fn test_decoupled_core_offline_queue_step() {
    let db = Database::open_in_memory().unwrap();
    let tmp_cache = std::env::temp_dir().join(format!("lacraia_decoupled_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&tmp_cache);

    // 1. Inserir faixa com arquivo em cache local existente
    let fake_file = tmp_cache.join("autonomo.opus");
    std::fs::write(&fake_file, b"AUTONOMOUS_OPUS_SAMPLE").unwrap();

    let trk = TrackRecord {
        id: "trk-auto-01".into(),
        artist: "Lacraia".into(),
        title: "Paredao Autonomo".into(),
        duration_sec: 180,
        cached_path: Some(fake_file.to_str().unwrap().to_string()),
        genres: None,
        last_played_at: None,
    };
    db.upsert_track(&trk).unwrap();
    db.enqueue_track_with_meta(&trk.id, Some("OFFLINE_AUTONOMOUS"), "default")
        .unwrap();

    // 2. Resolver em modo offline estrito (sem servidor externo)
    let resolver = Arc::new(MediaResolver::new_offline(tmp_cache.clone(), db.clone()));
    let similarity = Arc::new(CompositeSimilarityEngine::new_offline(db.clone()));
    let (tx, _rx) = broadcast::channel::<WsMessage>(16);
    let hub = Arc::new(PlaybackHub::new(db.clone(), tx));

    let daemon = Arc::new(QueueDaemon::new_with_temporal_window(
        db.clone(),
        resolver.clone(),
        similarity.clone(),
        hub.clone(),
        3600,
        7200,
        std::time::Duration::from_millis(50),
    ));

    // Executar ciclo do daemon
    let processed = daemon.step().await.expect("passo do daemon");
    assert!(
        processed >= 1,
        "Daemon deve processar faixas locais cacheadas mesmo sem servidor externo"
    );

    let queue = db.get_queue_for_cabin("default").unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(
        queue[0].0.state,
        QueueState::Cached,
        "Faixa local deve transicionar para Cached com sucesso no modo autônomo"
    );
}

#[tokio::test]
async fn test_decoupled_resolver_local_cache_bypass() {
    let db = Database::open_in_memory().unwrap();
    let tmp_cache = std::env::temp_dir().join(format!("lacraia_bypass_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&tmp_cache);

    let fake_audio = tmp_cache.join("cached_local.opus");
    std::fs::write(&fake_audio, b"LOCAL_AUDIO_PAYLOAD").unwrap();

    let track = TrackRecord {
        id: "trk-local-99".into(),
        artist: "Chico Science".into(),
        title: "Maracatu Atomico".into(),
        duration_sec: 285,
        cached_path: Some(fake_audio.to_str().unwrap().to_string()),
        genres: None,
        last_played_at: None,
    };
    db.upsert_track(&track).unwrap();

    // Resolver offline: deve servir diretamente do disco sem invocar rede
    let resolver = MediaResolver::new_offline(tmp_cache.clone(), db.clone());
    let res = resolver
        .resolve_and_cache("Chico Science", "Maracatu Atomico", 285)
        .await
        .expect("resolução de faixa local em disco");

    assert_eq!(res.track_id, "trk-local-99");
    assert_eq!(res.stream_url, "/cache/cached_local.opus");
    assert_eq!(res.cache_path, fake_audio.to_str().unwrap());
}

#[test]
fn test_html_cockpit_autonomous_mode_and_offline_resilience() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html = std::fs::read_to_string(root.join("site/app/index.html"))
        .expect("ler site/app/index.html");

    // Deve conter sinalização explícita de Modo Autônomo Local
    assert!(
        html.contains("MODO AUTÔNOMO LOCAL"),
        "HTML deve exibir 'MODO AUTÔNOMO LOCAL' quando desconectado do servidor remoto"
    );

    // Deve conter fallback para DEMO_PLAYLIST na ausência de fila remota
    assert!(
        html.contains("DEMO_PLAYLIST"),
        "HTML deve possuir DEMO_PLAYLIST para garantir reprodução mesmo sem servidor"
    );

    // Deve possuir tolerância a falhas na sincronização
    assert!(
        html.contains("syncQueue") && html.contains("syncCabinInfo"),
        "HTML deve implementar funções de sincronização desacopladas"
    );
}
