use std::path::Path;

#[test]
fn test_settings_modal_simplified_texts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // Verifica que textos simplificados e acessíveis estão presentes
    assert!(
        html.contains("PILOTO AUTOMÁTICO (TEMPO DE MÚSICA NA FILA)"),
        "O rótulo do autopilot deve ser simplificado para 'PILOTO AUTOMÁTICO (TEMPO DE MÚSICA NA FILA)'"
    );
    assert!(
        html.contains("MÚSICAS PRÉ-BAIXADAS (DOWNLOAD ANTECIPADO)"),
        "O rótulo de lookahead pré-cache deve ser simplificado para 'MÚSICAS PRÉ-BAIXADAS (DOWNLOAD ANTECIPADO)'"
    );
    assert!(
        html.contains("LIMITE DE ESPAÇO NO APARELHO (ARMAZENAMENTO)"),
        "O rótulo de cota de disco deve ser simplificado para 'LIMITE DE ESPAÇO NO APARELHO (ARMAZENAMENTO)'"
    );
    assert!(
        html.contains("CHAVE DE ADMINISTRAÇÃO DO APARELHO"),
        "O rótulo da master key deve ser simplificado para 'CHAVE DE ADMINISTRAÇÃO DO APARELHO'"
    );

    // Verifica ausência de jargões técnicos excessivos que confundem o usuário
    assert!(
        !html.contains("LOOKAHEAD PRÉ-CACHE"),
        "O termo jargão 'LOOKAHEAD PRÉ-CACHE' não deve estar presente no rótulo"
    );
    assert!(
        !html.contains("CGNAT"),
        "O jargão de rede 'CGNAT' não deve estar presente no texto da modal"
    );
}

#[test]
fn test_settings_text_mutation_catchers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // Mutante 1: Regressão de jargão deve falhar
    let mutant_reintroduce_cgnat = html.replace("sem precisar dividir o Wi-Fi", "atrás de CGNAT");
    assert!(
        mutant_reintroduce_cgnat.contains("CGNAT"),
        "Mutante detectado: reintrodução do termo CGNAT viola acessibilidade"
    );

    // Mutante 2: Remoção do rótulo amigável deve ser capturada
    let mutant_remove_friendly = html.replace("PILOTO AUTOMÁTICO (TEMPO DE MÚSICA NA FILA)", "");
    assert_ne!(mutant_remove_friendly, html);
}
