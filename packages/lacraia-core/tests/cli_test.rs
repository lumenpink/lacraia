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
fn test_cargo_toml_declares_canonical_lacraia_binary() {
    let repo_root = get_repo_root();
    let core_cargo_path = repo_root.join("packages/lacraia-core/Cargo.toml");
    let content =
        fs::read_to_string(&core_cargo_path).expect("Leitura de packages/lacraia-core/Cargo.toml");

    let cargo_toml: toml::Value =
        toml::from_str(&content).expect("Cargo.toml deve ser TOML válido");
    let bins = cargo_toml
        .get("bin")
        .and_then(|b| b.as_array())
        .expect("Deve haver declaração [[bin]]");

    let has_lacraia_bin = bins.iter().any(|b| {
        b.get("name").and_then(|n| n.as_str()) == Some("lacraia")
            && b.get("path").and_then(|p| p.as_str()) == Some("src/main.rs")
    });
    assert!(
        has_lacraia_bin,
        "packages/lacraia-core/Cargo.toml deve declarar [[bin]] name = 'lacraia'"
    );

    let has_legacy_bin = bins
        .iter()
        .any(|b| b.get("name").and_then(|n| n.as_str()) == Some("lacraia-core"));
    assert!(
        !has_legacy_bin,
        "packages/lacraia-core/Cargo.toml NÃO deve manter [[bin]] name = 'lacraia-core'"
    );
}

#[test]
fn test_ci_workflow_packages_lacraia_binary() {
    let repo_root = get_repo_root();
    let ci_yaml_path = repo_root.join(".forgejo/workflows/ci.yml");
    let ci_content =
        fs::read_to_string(&ci_yaml_path).expect("Leitura de .forgejo/workflows/ci.yml");

    assert!(
        ci_content.contains("-C target/x86_64-unknown-linux-musl/release lacraia"),
        "CI deve empacotar binário lacraia para x86_64-unknown-linux-musl"
    );
    assert!(
        ci_content.contains("-C target/aarch64-unknown-linux-musl/release lacraia"),
        "CI deve empacotar binário lacraia para aarch64-unknown-linux-musl"
    );
    assert!(
        ci_content.contains("lacraia.exe"),
        "CI deve empacotar executável lacraia.exe para Windows"
    );
    assert!(
        !ci_content.contains("-C target/x86_64-unknown-linux-musl/release lacraia-core"),
        "CI não deve referenciar lacraia-core no empacotamento x86_64"
    );
    assert!(
        !ci_content.contains("-C target/aarch64-unknown-linux-musl/release lacraia-core"),
        "CI não deve referenciar lacraia-core no empacotamento aarch64"
    );
    assert!(
        !ci_content.contains("lacraia-core.exe"),
        "CI não deve referenciar lacraia-core.exe no empacotamento Windows"
    );
}

#[test]
fn test_dockerfile_uses_canonical_lacraia_binary() {
    let repo_root = get_repo_root();
    let dockerfile_path = repo_root.join("Dockerfile");
    let content = fs::read_to_string(&dockerfile_path).expect("Leitura do Dockerfile");

    assert!(
        content.contains("/usr/local/bin/lacraia"),
        "Dockerfile deve copiar executável para /usr/local/bin/lacraia"
    );
    assert!(
        content.contains("ENTRYPOINT [\"/usr/bin/tini\", \"--\", \"lacraia\"]"),
        "Dockerfile deve ter ENTRYPOINT apontando para lacraia"
    );
    assert!(
        !content.contains("/usr/local/bin/lacraia-core"),
        "Dockerfile não deve manter referências a /usr/local/bin/lacraia-core"
    );
}

#[tokio::test]
async fn test_cli_version_flags_canonical_output() {
    let repo_root = get_repo_root();
    let cli_rs_path = repo_root.join("packages/lacraia-core/src/cli.rs");
    let cli_src = fs::read_to_string(&cli_rs_path).expect("Leitura de cli.rs");

    // Valida que a saída de versão emite "lacraia x.y.z" (sem o 'v' redundante)
    assert!(
        cli_src.contains("println!(\"lacraia {}\", CURRENT_VERSION)")
            || cli_src.contains("println!(\"lacraia {}\", CURRENT_VERSION);"),
        "cli.rs deve emitir 'lacraia {{CURRENT_VERSION}}' canonicamente para --version"
    );
    assert!(
        !cli_src.contains("println!(\"lacraia v{}\", CURRENT_VERSION)"),
        "cli.rs não deve emitir prefixo 'v' em --version"
    );
}

#[test]
fn test_mutation_resistance_cli_binary_and_version() {
    let _repo_root = get_repo_root();

    // Mutante 1: Regressão de binário legado em Cargo.toml
    let mutant_cargo = r#"
        [[bin]]
        name = "lacraia-core"
        path = "src/main.rs"
    "#;
    let parsed: toml::Value = toml::from_str(mutant_cargo).unwrap();
    let bins = parsed.get("bin").and_then(|b| b.as_array()).unwrap();
    let has_legacy = bins
        .iter()
        .any(|b| b.get("name").and_then(|n| n.as_str()) == Some("lacraia-core"));
    assert!(
        has_legacy,
        "Mutante 1 com binário legado 'lacraia-core' deve ser detectável"
    );

    // Mutante 2: Regressão de versão com prefixo 'v'
    let mutant_cli_src = "println!(\"lacraia v{}\", CURRENT_VERSION);";
    let has_v_prefix = mutant_cli_src.contains("lacraia v");
    assert!(
        has_v_prefix,
        "Mutante 2 com prefixo 'v' deve ser detectável"
    );

    // Mutante 3: Regressão de pacote lacraia-core em CI workflow
    let mutant_ci = "tar -czf dist.tar.gz -C target/release lacraia-core";
    let has_ci_legacy = mutant_ci.contains("lacraia-core");
    assert!(
        has_ci_legacy,
        "Mutante 3 com lacraia-core no CI deve ser detectável"
    );
}
