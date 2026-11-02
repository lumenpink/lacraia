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

    // Opção A: User-Agent nativo e inicialização plug-and-play sem diálogo bloqueante no onCreate
    assert!(
        kt_content.contains("LacraiaApp"),
        "MainActivity.kt deve identificar o User-Agent com 'LacraiaApp'"
    );
    assert!(
        kt_content.contains("LACRAIA_NATIVE"),
        "MainActivity.kt deve injetar window.LACRAIA_NATIVE = true para bridge JS"
    );
    assert!(
        !kt_content.contains("showServerConfigDialog(defaultEndpoint)"),
        "MainActivity.kt não deve abrir pop-up bloqueante de servidor no onCreate"
    );
}

#[test]
fn test_cockpit_native_app_detection_and_bancada_toggle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // 1. O botão BANCADA deve estar presente no HTML para o ambiente Web/PWA
    assert!(
        html.contains("BANCADA"),
        "O Cockpit deve conter o botão BANCADA no HTML para a versão Web/PWA"
    );

    // 2. Deve conter lógica isNativeApp detectando User-Agent e window.LACRAIA_NATIVE
    assert!(
        html.contains("isNativeApp") || html.contains("LACRAIA_NATIVE"),
        "Cockpit deve implementar a detecção isNativeApp via User-Agent ou bridge nativa"
    );
    assert!(
        html.contains("LacraiaApp"),
        "Cockpit deve verificar a presença de 'LacraiaApp' no User-Agent"
    );

    // 3. O botão BANCADA deve ser ocultado para apps nativos
    assert!(
        html.contains("btn-bancada") || html.contains("bancada-link"),
        "O botão BANCADA deve possuir um ID dedicado para controle de exibição"
    );

    // 4. Modal CONFIG deve possuir seletor de Homelab / Servidor Remoto
    assert!(
        html.contains("homelab") || html.contains("input-remote-server"),
        "O modal CONFIG deve possuir opção para configurar servidor Homelab remoto"
    );
}

#[test]
fn test_android_cockpit_offline_and_error_fallback() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let kt_path = root
        .join("packages/lacraia-android/app/src/main/java/in/twolp/lacraia/app/MainActivity.kt");
    let kt_content = std::fs::read_to_string(&kt_path).expect("ler MainActivity.kt");

    assert!(
        kt_content.contains("android_asset")
            || kt_content.contains("file:///android_asset/index.html"),
        "MainActivity.kt deve conter rota de fallback para o Cockpit embarcado em assets"
    );
    assert!(
        kt_content.contains("onReceivedError"),
        "MainActivity.kt deve tratar onReceivedError no WebViewClient para evitar tela preta"
    );
    assert!(
        kt_content.contains("allowFileAccess = true"),
        "MainActivity.kt deve habilitar allowFileAccess para os assets locais"
    );

    let gradle_path = root.join("packages/lacraia-android/app/build.gradle.kts");
    let gradle_content = std::fs::read_to_string(&gradle_path).expect("ler build.gradle.kts");
    assert!(
        gradle_content.contains("assets.srcDirs") && gradle_content.contains("site/app"),
        "build.gradle.kts deve embarcar site/app nos assets da aplicação"
    );
}
