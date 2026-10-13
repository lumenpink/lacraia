use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn get_repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Root do repositório deve ser acessível")
        .to_path_buf()
}

#[test]
fn test_distribution_manifest_version_json() {
    let repo_root = get_repo_root();
    let version_json_path = repo_root.join("site/version.json");

    assert!(
        version_json_path.exists(),
        "site/version.json deve existir para autodescoberta pelo Obtainium"
    );

    let content = fs::read_to_string(&version_json_path).expect("Leitura de site/version.json");
    let manifest: Value =
        serde_json::from_str(&content).expect("site/version.json deve ser JSON válido");

    let version = manifest["version"]
        .as_str()
        .expect("version deve ser string");
    assert!(!version.is_empty(), "version não deve ser vazio");

    let apk_url = manifest["apk_url"]
        .as_str()
        .expect("apk_url deve ser string");
    assert!(
        apk_url.ends_with("/lacraia-app.apk"),
        "apk_url padrão deve apontar para lacraia-app.apk: {}",
        apk_url
    );

    let package_id = manifest["package_id"]
        .as_str()
        .expect("package_id deve ser string");
    assert_eq!(
        package_id, "in.twolp.lacraia.app",
        "package_id deve usar o identificador canônico soberano in.twolp"
    );
}

#[test]
fn test_obtainium_html_autodiscovery() {
    let repo_root = get_repo_root();
    let index_html_path = repo_root.join("site/index.html");

    let html = fs::read_to_string(&index_html_path).expect("Leitura de site/index.html");

    // Validação de links diretos servidos em lacraia.2lp.in
    assert!(
        html.contains("https://lacraia.2lp.in/lacraia-app.apk"),
        "site/index.html deve conter link direto para lacraia-app.apk no domínio oficial"
    );
    assert!(
        html.contains("https://lacraia.2lp.in/simbiotyka.apk"),
        "site/index.html deve conter link direto para simbiotyka.apk no domínio oficial"
    );

    // Validação de instruções e Package IDs para o Obtainium
    assert!(
        html.contains("in.twolp.lacraia.app"),
        "site/index.html deve declarar o Package ID in.twolp.lacraia.app para Obtainium"
    );
    assert!(
        html.contains("in.twolp.lacraia.simbiotyka"),
        "site/index.html deve declarar o Package ID in.twolp.lacraia.simbiotyka para Obtainium"
    );
    assert!(
        html.contains("Obtainium") || html.contains("OBTAINIUM"),
        "site/index.html deve conter seção com orientações para o Obtainium"
    );
}

#[test]
fn test_site_headers_for_apk_and_manifest() {
    let repo_root = get_repo_root();
    let headers_path = repo_root.join("site/_headers");

    let headers = fs::read_to_string(&headers_path).expect("Leitura de site/_headers");

    assert!(
        headers.contains("application/vnd.android.package-archive"),
        "site/_headers deve declarar o Content-Type oficial para APKs Android"
    );
    assert!(
        headers.contains("/version.json"),
        "site/_headers deve configurar cabeçalhos para /version.json"
    );
}

#[test]
fn test_android_gradle_canonical_package_ids() {
    let repo_root = get_repo_root();
    let app_gradle_path = repo_root.join("packages/lacraia-android/app/build.gradle.kts");
    let simbiotyka_gradle_path =
        repo_root.join("packages/lacraia-android/simbiotyka/build.gradle.kts");

    let app_gradle = fs::read_to_string(&app_gradle_path).expect("Leitura de app/build.gradle.kts");
    let simbiotyka_gradle = fs::read_to_string(&simbiotyka_gradle_path)
        .expect("Leitura de simbiotyka/build.gradle.kts");

    assert!(
        app_gradle.contains("in.twolp.lacraia.app"),
        "app/build.gradle.kts deve declarar applicationId in.twolp.lacraia.app"
    );
    assert!(
        simbiotyka_gradle.contains("in.twolp.lacraia.simbiotyka"),
        "simbiotyka/build.gradle.kts deve declarar applicationId in.twolp.lacraia.simbiotyka"
    );
}

#[test]
fn test_ci_workflow_apk_publishing_step() {
    let repo_root = get_repo_root();
    let ci_yaml_path = repo_root.join(".forgejo/workflows/ci.yml");

    let ci_yaml = fs::read_to_string(&ci_yaml_path).expect("Leitura de .forgejo/workflows/ci.yml");

    assert!(
        ci_yaml.contains("android-apks"),
        "Workflow de CI deve gerenciar artefato android-apks"
    );
    assert!(
        ci_yaml.contains("lacraia-app.apk"),
        "Workflow de CI deve publicar lacraia-app.apk"
    );
    assert!(
        ci_yaml.contains("simbiotyka.apk"),
        "Workflow de CI deve publicar simbiotyka.apk"
    );
    assert!(
        ci_yaml.contains("version.json"),
        "Workflow de CI deve atualizar version.json no deploy do site"
    );
}
