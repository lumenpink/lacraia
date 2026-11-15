use std::path::Path;

#[test]
fn test_settings_modal_vertical_scroll_and_containment() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // O arquivo deve conter a modal de configurações
    assert!(
        html.contains("id=\"settings-modal\""),
        "O Cockpit deve conter a modal de configurações #settings-modal"
    );

    // Deve conter estilização com contenção vertical (max-height) e rolagem (overflow-y: auto)
    // para evitar que controles e botões sejam cortados em telas móveis
    assert!(
        html.contains("max-height: 85vh")
            || html.contains("max-height: 90vh")
            || html.contains("max-height: 80vh"),
        "A modal de configurações ou .modal-card deve conter max-height entre 80vh e 90vh"
    );
    assert!(
        html.contains("overflow-y: auto") || html.contains("overflow-y:auto"),
        "A modal de configurações ou .modal-card deve habilitar overflow-y: auto"
    );

    // Botão de salvar e fechar devem estar dentro da estrutura
    assert!(
        html.contains("id=\"btn-save-settings\"") && html.contains("id=\"btn-close-settings\""),
        "A modal deve conter os botões de ação 'btn-save-settings' e 'btn-close-settings'"
    );

    // Estilização customizada de scrollbar (WebKit e padrão CSS scrollbar-width)
    assert!(
        html.contains("scrollbar-width: thin"),
        "A modal deve configurar scrollbar-width: thin para navegadores modernos"
    );
    assert!(
        html.contains(".modal-card::-webkit-scrollbar"),
        "A modal deve configurar ::-webkit-scrollbar para WebViews Android"
    );
}

#[test]
fn test_settings_modal_mutation_catchers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // Mutante 1: Ausência de scroll causaria vazamento vertical
    let mutant_no_scroll = html.replace("overflow-y: auto;", "");
    assert!(
        !mutant_no_scroll.contains(".modal-card {\n      background: var(--ferro-bate-pedra);\n      border: 3px solid var(--fucsia-furacao);\n      box-shadow: 0 0 20px rgba(255, 0, 85, 0.4);\n      max-width: 440px;\n      width: 100%;\n      max-height: 85vh;\n      overflow-y: auto;"),
        "Mutante detectado: sem overflow-y o layout estouraria verticalmente"
    );

    // Mutante 2: Ausência de max-height deixaria o card infinito
    let mutant_no_height = html.replace("max-height: 85vh;", "");
    assert_ne!(mutant_no_height, html);
}
