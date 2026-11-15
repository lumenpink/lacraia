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
        html.contains("LIMITE DE ESPAÇO NO APARELHO") || html.contains("LIMITE DE ARMAZENAMENTO"),
        "O rótulo de cota de disco deve ser simplificado"
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
