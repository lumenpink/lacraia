use std::path::Path;

#[test]
fn test_musicbrainz_android_user_agent_and_network_security() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    // 1. Validar User-Agent canônico com contato no MainActivity.kt
    let kt_path = root
        .join("packages/lacraia-android/app/src/main/java/in/twolp/lacraia/app/MainActivity.kt");
    let kt_content = std::fs::read_to_string(&kt_path).expect("ler MainActivity.kt");
    assert!(
        kt_content.contains("contact@2lp.in") && kt_content.contains("LacraiaApp"),
        "MainActivity.kt deve configurar User-Agent canônico com contato para o MusicBrainz: ( contact@2lp.in )"
    );

    // 2. Validar Network Security Config no AndroidManifest.xml
    let manifest_path = root.join("packages/lacraia-android/app/src/main/AndroidManifest.xml");
    let manifest = std::fs::read_to_string(&manifest_path).expect("ler AndroidManifest.xml");
    assert!(
        manifest.contains("android:networkSecurityConfig=\"@xml/network_security_config\""),
        "AndroidManifest.xml deve declarar android:networkSecurityConfig"
    );

    // 3. Validar existência e regras do network_security_config.xml
    let net_xml_path =
        root.join("packages/lacraia-android/app/src/main/res/xml/network_security_config.xml");
    assert!(
        net_xml_path.exists(),
        "network_security_config.xml deve existir em res/xml/"
    );
    let net_xml = std::fs::read_to_string(&net_xml_path).expect("ler network_security_config.xml");
    assert!(
        net_xml.contains("musicbrainz.org"),
        "network_security_config.xml deve permitir explicitamente o domínio musicbrainz.org"
    );

    // 4. Validar User-Agent no core (MetaBrainzProvider)
    let similarity_path = root.join("packages/lacraia-core/src/similarity.rs");
    let sim_content = std::fs::read_to_string(&similarity_path).expect("ler similarity.rs");
    assert!(
        sim_content.contains("contact@2lp.in"),
        "similarity.rs deve conter User-Agent canônico com contato para MusicBrainz"
    );
}

#[test]
fn test_musicbrainz_mutation_catchers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    // Mutante 1: Regressão sem contato no User-Agent do Android
    let kt_path = root
        .join("packages/lacraia-android/app/src/main/java/in/twolp/lacraia/app/MainActivity.kt");
    let kt_content = std::fs::read_to_string(&kt_path).expect("ler MainActivity.kt");
    let mutant_no_contact = kt_content.replace(" ( contact@2lp.in )", "");
    assert_ne!(mutant_no_contact, kt_content);

    // Mutante 2: Remoção do domínio musicbrainz no network security config
    let net_xml_path =
        root.join("packages/lacraia-android/app/src/main/res/xml/network_security_config.xml");
    let net_xml = std::fs::read_to_string(&net_xml_path).expect("ler network_security_config.xml");
    let mutant_no_domain = net_xml.replace("musicbrainz.org", "");
    assert_ne!(mutant_no_domain, net_xml);
}
