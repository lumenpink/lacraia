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
        html.contains("id=\"btn-save-settings\"") || html.contains("id=\"btn-close-settings\""),
        "A modal deve conter botões de ação acessíveis"
    );
}
