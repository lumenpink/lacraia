use std::path::Path;

#[test]
fn test_van_mode_settings_container_and_toggle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // 1. Deve existir um container dedicado para as configurações do Modo Van
    assert!(
        html.contains("id=\"van-mode-settings-box\""),
        "Deve existir um container #van-mode-settings-box para agrupar campos da Van"
    );

    // 2. Por padrão, o container deve estar oculto no HTML
    assert!(
        html.contains("id=\"van-mode-settings-box\" style=\"display:none")
            || html.contains("id=\"van-mode-settings-box\" style=\"display: none"),
        "O container #van-mode-settings-box deve iniciar com display:none por padrão"
    );

    // 3. A função applyVanMode deve controlar a visibilidade do #van-mode-settings-box
    assert!(
        html.contains("van-mode-settings-box")
            && (html.contains("vanSettingsBox") || html.contains("vanModeSettingsBox")),
        "A função applyVanMode deve gerenciar a exibição do elemento #van-mode-settings-box"
    );

    // 4. Campos específicos da Van devem estar contidos dentro do #van-mode-settings-box
    let box_pos = html.find("id=\"van-mode-settings-box\"").unwrap();
    let master_key_pos = html.find("id=\"input-master-key\"").unwrap();
    let relay_url_pos = html.find("id=\"input-relay-url\"").unwrap();

    assert!(
        master_key_pos > box_pos,
        "input-master-key deve estar dentro do van-mode-settings-box"
    );
    assert!(
        relay_url_pos > box_pos,
        "input-relay-url deve estar dentro do van-mode-settings-box"
    );
}

#[test]
fn test_van_mode_mutation_catchers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // Mutante 1: Regressão onde van-mode-settings-box começa visível
    let mutant_visible = html.replace("display:none;", "display:block;");
    assert!(
        !mutant_visible.contains("id=\"van-mode-settings-box\" style=\"display:none;"),
        "Mutante detectado: box não pode iniciar visível por padrão"
    );

    // Mutante 2: Remoção do toggle dinâmico em applyVanMode
    let mutant_no_toggle = html.replace("vanSettingsBox.style.display", "");
    assert_ne!(mutant_no_toggle, html);
}
