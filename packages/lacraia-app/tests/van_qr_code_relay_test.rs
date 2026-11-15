use std::path::Path;

#[test]
fn test_qr_code_points_to_microrelay_with_van_hash() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // 1. Deve existir função de resolução pública de base URL para evitar links locais em APK/localhost
    assert!(
        html.contains("resolvePublicPassengerBaseUrl"),
        "HTML deve implementar a função resolvePublicPassengerBaseUrl para sanear URLs no APK"
    );

    // 2. updateQrModalContent deve incluir vanHash e parâmetros de relay
    assert!(
        html.contains("relayBase") && html.contains("resolvePublicPassengerBaseUrl()"),
        "updateQrModalContent deve usar a base do microrelay para passageiros"
    );

    // 3. connectWs deve aceitar parâmetro de query relay vindo do link do QR Code
    assert!(
        html.contains("urlParams.get(\"relay\")") || html.contains("urlParams.get('relay')"),
        "connectWs deve aceitar o parâmetro relay fornecido pelo QR Code escaneado"
    );
}

#[test]
fn test_qr_code_relay_mutation_catchers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let html_path = root.join("site/app/index.html");
    let html = std::fs::read_to_string(&html_path).expect("ler site/app/index.html");

    // Mutante 1: Regressão onde o link volta a apontar para window.location.origin diretamente
    let mutant_regressed = html.replace(
        "const relayBase = resolvePublicPassengerBaseUrl();",
        "const relayBase = window.location.origin;",
    );
    assert_ne!(mutant_regressed, html);

    // Mutante 2: Falha ao propagar relay na query string
    let mutant_no_relay_param = html.replace(
        "passengerUrl += `&relay=${encodeURIComponent(storedRelay)}`;",
        "",
    );
    assert_ne!(mutant_no_relay_param, html);
}
