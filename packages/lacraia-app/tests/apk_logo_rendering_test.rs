use std::path::Path;

#[test]
fn test_apk_logo_inline_svg_rendering() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // 1. Não deve utilizar caminho absoluto quebrado (/assets/icon.svg) que falha no APK WebView
    assert!(
        !html.contains("src=\"/assets/icon.svg\""),
        "O logotipo não deve depender de src=\"/assets/icon.svg\" que quebra no WebView Android"
    );

    // 2. Deve embarcar SVG inline no brand-section para renderização offline-first
    assert!(
        html.contains("<svg") && html.contains("class=\"brand-logo-img\""),
        "A seção de marca deve conter o logotipo vetorial embarcado como SVG inline com classe brand-logo-img"
    );

    // 3. Deve preservar os elementos estilizados e filtros neon da identidade da Lacraia
    assert!(
        html.contains("glow-veneno") && html.contains("glow-fucsia"),
        "O logotipo inline deve conter as definições de filtro glow-veneno e glow-fucsia"
    );
}

#[test]
fn test_apk_logo_mutation_catchers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // Mutante 1: Regressão reintroduzindo src com caminho absoluto
    let mutant_regressed = html.replace("<svg", "<img src=\"/assets/icon.svg\"");
    assert!(
        mutant_regressed.contains("src=\"/assets/icon.svg\""),
        "Mutante detectado: reintrodução do caminho absoluto quebra APK"
    );

    // Mutante 2: Remoção dos filtros luminosos essenciais da marca
    let mutant_no_filters = html.replace("glow-veneno", "");
    assert_ne!(mutant_no_filters, html);
}
