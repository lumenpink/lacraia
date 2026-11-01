use lacraia_shared::models::{PlaybackState, Track};
use lacraia_shared::tui::{
    format_progress_bar, is_tui_requested, TuiAction, TuiRenderer, TuiState, TuiTab,
};

#[test]
fn test_tui_render_dashboard_layout_and_elements() {
    let state = TuiState {
        active_cabin: "cabine_paredao".to_string(),
        ws_connected: false,
        playback_state: PlaybackState::Playing,
        position_ms: 45000,
        duration_ms: 180000,
        volume: 8,
        station: "Manguebeat & Baile".to_string(),
        spectrum_bands: [1, 3, 5, 7, 8, 7, 6, 4, 3, 2, 5, 7, 8, 6, 4, 2],
        current_track: Some(Track {
            id: "trk_mangue_01".to_string(),
            title: "Maracatu Atômico".to_string(),
            artist: "Chico Science & Nação Zumbi".to_string(),
            album: Some("Afrociberdelia".to_string()),
            duration_ms: 180000,
            cover_url: None,
            stream_url: Some("/cache/maracatu.opus".to_string()),
            requested_by: Some("PILOTO".to_string()),
        }),
        queue: vec![Track {
            id: "trk_mangue_02".to_string(),
            title: "A Cidade".to_string(),
            artist: "Chico Science".to_string(),
            album: None,
            duration_ms: 210000,
            cover_url: None,
            stream_url: None,
            requested_by: Some("PASSAGEIRA".to_string()),
        }],
        ..Default::default()
    };

    let rendered = TuiRenderer::render_to_string(&state, 80, 24);

    // 1. Identidade e Conectividade
    assert!(
        rendered.contains("LA CRAIA"),
        "A TUI deve renderizar o cabeçalho La Craia"
    );
    assert!(
        rendered.contains("WS :: MODO AUTÔNOMO LOCAL"),
        "Em modo desconectado a TUI deve indicar Modo Autônomo Local"
    );
    assert!(
        rendered.contains("cabine_paredao"),
        "A TUI deve exibir a cabine ativa"
    );

    // 2. Deck Now Playing
    assert!(
        rendered.contains("Maracatu Atômico"),
        "A TUI deve exibir o título da faixa atual"
    );
    assert!(
        rendered.contains("Chico Science"),
        "A TUI deve exibir o artista atual"
    );
    assert!(
        rendered.contains("VOL: 8/11"),
        "A TUI deve exibir o ganho até 11"
    );

    // 3. Visualizador e Atalhos
    assert!(
        rendered.contains("█") || rendered.contains("▆") || rendered.contains("▄"),
        "A TUI deve renderizar barras do espectrograma Unicode"
    );
    assert!(
        rendered.contains("[Espaço]"),
        "A TUI deve exibir rodapé com atalhos de controle"
    );
}

#[test]
fn test_tui_key_actions_and_state_transitions() {
    let mut state = TuiState {
        volume: 8,
        playback_state: PlaybackState::Paused,
        ..Default::default()
    };

    // Espaço: Play / Pause toggle
    assert_eq!(state.handle_key(' '), TuiAction::TogglePlay);
    assert_eq!(state.playback_state, PlaybackState::Playing);
    assert_eq!(state.handle_key(' '), TuiAction::TogglePlay);
    assert_eq!(state.playback_state, PlaybackState::Paused);

    // Navegação de faixas
    assert_eq!(state.handle_key('n'), TuiAction::NextTrack);
    assert_eq!(state.handle_key('N'), TuiAction::NextTrack);
    assert_eq!(state.handle_key('p'), TuiAction::PrevTrack);
    assert_eq!(state.handle_key('P'), TuiAction::PrevTrack);

    // Ajuste de volume (0..=11)
    assert_eq!(state.handle_key('+'), TuiAction::VolumeUp);
    assert_eq!(state.volume, 9);
    assert_eq!(state.handle_key('k'), TuiAction::VolumeUp);
    assert_eq!(state.volume, 10);
    assert_eq!(state.handle_key('k'), TuiAction::VolumeUp);
    assert_eq!(state.volume, 11);
    assert_eq!(state.handle_key('+'), TuiAction::VolumeUp);
    assert_eq!(state.volume, 11, "Volume não deve exceder 11");

    assert_eq!(state.handle_key('-'), TuiAction::VolumeDown);
    assert_eq!(state.volume, 10);
    assert_eq!(state.handle_key('j'), TuiAction::VolumeDown);
    assert_eq!(state.volume, 9);

    // Alternância de abas / painéis
    assert_eq!(state.handle_key('2'), TuiAction::SwitchTab(TuiTab::Queue));
    assert_eq!(state.active_tab, TuiTab::Queue);
    assert_eq!(
        state.handle_key('3'),
        TuiAction::SwitchTab(TuiTab::Telemetry)
    );
    assert_eq!(state.active_tab, TuiTab::Telemetry);
    assert_eq!(
        state.handle_key('1'),
        TuiAction::SwitchTab(TuiTab::NowPlaying)
    );
    assert_eq!(state.active_tab, TuiTab::NowPlaying);
    assert_eq!(state.handle_key('\t'), TuiAction::NextTab);
    assert_eq!(state.active_tab, TuiTab::Queue);

    // Sair
    assert_eq!(state.handle_key('q'), TuiAction::Quit);
    assert_eq!(state.handle_key('Q'), TuiAction::Quit);
}

#[test]
fn test_cli_tui_flag_detection() {
    assert!(is_tui_requested(&["tui".to_string()]));
    assert!(is_tui_requested(&["--tui".to_string()]));
    assert!(is_tui_requested(&[
        "lacraia-app".to_string(),
        "--tui".to_string()
    ]));
    assert!(is_tui_requested(&[
        "lacraia-core".to_string(),
        "tui".to_string()
    ]));
    assert!(!is_tui_requested(&["--help".to_string()]));
    assert!(!is_tui_requested(&[]));
}

#[test]
fn test_progress_bar_calculation() {
    let bar_half = format_progress_bar(50000, 100000, 20);
    assert_eq!(bar_half.chars().count(), 20);
    assert!(bar_half.starts_with("=========="));

    let bar_zero = format_progress_bar(0, 100000, 10);
    assert_eq!(bar_zero, "----------");

    let bar_full = format_progress_bar(100000, 100000, 10);
    assert_eq!(bar_full, "==========");

    // Overflow clamp
    let bar_overflow = format_progress_bar(200000, 100000, 10);
    assert_eq!(bar_overflow, "==========");
}

#[test]
fn test_mutation_resistance_tui_actions_and_limits() {
    let mut state = TuiState {
        volume: 11,
        ..Default::default()
    };

    // Mutante 1: volume maximo travado estritamente em 11
    assert_eq!(state.handle_key('+'), TuiAction::VolumeUp);
    assert_eq!(
        state.volume, 11,
        "Mutante pego: volume nao pode ultrapassar 11"
    );

    // Mutante 2: volume minimo travado estritamente em 0
    state.volume = 0;
    assert_eq!(state.handle_key('-'), TuiAction::VolumeDown);
    assert_eq!(
        state.volume, 0,
        "Mutante pego: volume nao pode ser menor que 0"
    );

    // Mutante 3: ciclo circular de abas
    state.active_tab = TuiTab::Telemetry;
    assert_eq!(state.handle_key('\t'), TuiAction::NextTab);
    assert_eq!(
        state.active_tab,
        TuiTab::NowPlaying,
        "Mutante pego: de Telemetry o Tab deve ciclar para NowPlaying"
    );

    // Mutante 4: tecla desconhecida retorna TuiAction::None
    assert_eq!(state.handle_key('z'), TuiAction::None);
    assert_eq!(state.handle_key('9'), TuiAction::None);
}
