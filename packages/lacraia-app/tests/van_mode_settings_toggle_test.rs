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
}
