use lacraia_app::server_config::ServerConfig;
use std::path::Path;

#[test]
fn test_server_config_defaults_and_sanitization() {
    let cfg = ServerConfig::default();
    assert_eq!(cfg.server_url, "http://localhost:2011");
    assert_eq!(cfg.cabin_id, "default");

    // Sanitização de URLs
    assert_eq!(
        ServerConfig::sanitize_server_url("localhost:2011"),
        "http://localhost:2011"
    );
    assert_eq!(
        ServerConfig::sanitize_server_url("http://localhost:2011/"),
        "http://localhost:2011"
    );
    assert_eq!(
        ServerConfig::sanitize_server_url("https://mylacraia.2lp.in///"),
        "https://mylacraia.2lp.in"
    );
    assert_eq!(
        ServerConfig::sanitize_server_url(""),
        "http://localhost:2011"
    );
    assert_eq!(
        ServerConfig::sanitize_server_url("   192.168.1.100:2011   "),
        "http://192.168.1.100:2011"
    );
}

#[test]
fn test_server_config_save_and_load() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg_file = tmp.path().join("client.toml");

    let cfg = ServerConfig {
        server_url: "http://paredao.local:2011".to_string(),
        cabin_id: "cabine_vip".to_string(),
    };

    cfg.save(Some(&cfg_file)).expect("salvar config");

    let loaded = ServerConfig::load(Some(&cfg_file));
    assert_eq!(loaded.server_url, "http://paredao.local:2011");
    assert_eq!(loaded.cabin_id, "cabine_vip");

    // Mutante: Arquivo inexistente deve retornar defaults
    let non_existent = tmp.path().join("nao_existe.toml");
    let fallback = ServerConfig::load(Some(&non_existent));
    assert_eq!(fallback, ServerConfig::default());

    // Mutante: Conteúdo inválido deve retornar defaults sem pânico
    let invalid_file = tmp.path().join("invalido.toml");
    std::fs::write(&invalid_file, b"NOT_A_VALID_TOML = [[[}}").unwrap();
    let safe_fallback = ServerConfig::load(Some(&invalid_file));
    assert_eq!(safe_fallback, ServerConfig::default());
}

#[test]
fn test_android_main_activity_cockpit_isolation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let kt_path = root
        .join("packages/lacraia-android/app/src/main/java/in/twolp/lacraia/app/MainActivity.kt");
    let kt_content = std::fs::read_to_string(&kt_path).expect("ler MainActivity.kt");

    // Não deve conter a URL do site de apresentação institucional como fallback padrão
    assert!(
        !kt_content.contains("https://lacraia.2lp.in"),
        "MainActivity.kt não deve carregar o site de apresentação institucional"
    );

    // Deve conter configuração persistente de servidor e padrão de cockpit na raiz
    assert!(
        kt_content.contains("server_endpoint"),
        "MainActivity.kt deve gerenciar a preferência server_endpoint"
    );
    assert!(
        kt_content.contains("10.0.2.2:2011"),
        "MainActivity.kt deve sugerir 10.0.2.2:2011 para o emulador Android"
    );
    assert!(
        !kt_content.contains("/app"),
        "MainActivity.kt não deve depender de caminho /app; o Cockpit é o único caminho"
    );
}
