#[test]
fn test_spec_gost_2_0_conformance() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    // 1. Validar GOSTVER
    let gostver = std::fs::read_to_string(root.join("GOSTVER")).expect("ler GOSTVER");
    assert!(
        gostver.contains("GOST_VERSION=\"2.0.0\""),
        "GOSTVER deve apontar para GOST_VERSION=\"2.0.0\""
    );
    assert!(
        gostver.contains("GOST_PROFILE_URL=\"https://lilya.2lp.in/profile.md\""),
        "GOSTVER deve apontar para o perfil lilya"
    );

    // 2. Validar SPEC.md
    let spec = std::fs::read_to_string(root.join("SPEC.md")).expect("ler SPEC.md");
    let spec_lower = spec.to_lowercase();

    // Padrão normativo e perfil
    assert!(
        spec.contains("GOST `2.0.0`"),
        "SPEC.md deve referenciar o padrão GOST `2.0.0`"
    );
    assert!(
        spec_lower.contains("lilya"),
        "SPEC.md deve referenciar o perfil lilya"
    );
    assert!(
        spec.contains("1.1.0"),
        "SPEC.md deve conter a versão da especificação 1.1.0"
    );

    // Ausência estrita de termos imperialistas
    assert!(
        !spec_lower.contains("soberan"),
        "SPEC.md não pode conter termos relacionados a soberania/soberano"
    );

    // 9 seções essenciais em português
    let secoes_essenciais = [
        "1. Escopo e Filosofia",
        "2. Anatomia do Repositório",
        "3. Arquitetura do Workspace",
        "4. Topologia de Operação",
        "5. Armazenamento SQLite",
        "6. Pipeline de Áudio",
        "7. Dimensionamento Estatístico",
        "8. Protocolo WebSocket",
        "9. Pirâmide de Testes",
    ];

    let mut last_idx = 0;
    for secao in secoes_essenciais {
        let idx = spec
            .find(secao)
            .unwrap_or_else(|| panic!("SPEC.md deve conter a seção normativa '{secao}'"));
        assert!(
            idx >= last_idx,
            "Seção '{secao}' deve aparecer após a seção anterior em ordem cronológica"
        );
        last_idx = idx;
    }

    // Licença GPL-3.0 e autoria
    assert!(
        spec.contains("GNU General Public License v3.0 (GPL-3.0)"),
        "SPEC.md deve declarar licença GPL-3.0"
    );
    assert!(
        spec.contains("Lumen Pink"),
        "SPEC.md deve registrar a autoria"
    );
}
