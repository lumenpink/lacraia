#[test]
fn test_landing_page_e_readme_manifesto_rua() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let site_html =
        std::fs::read_to_string(root.join("site/index.html")).expect("ler site/index.html");
    let readme = std::fs::read_to_string(root.join("README.md")).expect("ler README.md");

    let site_lower = site_html.to_lowercase();
    let readme_lower = readme.to_lowercase();

    let proibidos = [
        "afinidade acústica",
        "soberania acústica",
        "compilação e disparo do core",
        "distribuição 100% soberana",
    ];

    for termo in proibidos {
        assert!(
            !site_lower.contains(termo),
            "site/index.html ainda contém jargão técnico: '{termo}'"
        );
        assert!(
            !readme_lower.contains(termo),
            "README.md ainda contém jargão técnico: '{termo}'"
        );
    }

    assert!(
        site_lower.contains("paredão") && site_lower.contains("rua"),
        "site/index.html deve valorizar a identidade de paredão e rua"
    );
    assert!(
        readme_lower.contains("paredão") && readme_lower.contains("rua"),
        "README.md deve valorizar a identidade de paredão e rua"
    );
}
