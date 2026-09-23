pub mod audio;
pub mod ui;

use dioxus::prelude::*;
use lacraia_shared::{PlaybackState, Track};
use ui::{format_gain_11, format_time, PlayerState, QueueController};

fn main() {
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}

const BRUTALIST_CSS: &str = r#"
:root {
  --madeira-compensado:     #2b1d14;
  --madeira-veios:          #1f140e;
  --ferro-bate-pedra:       #151618;
  --ferro-painel:           #1f2126;
  --metal-cantoneira:       #42464e;
  --fucsia-furacao:         #ff0055;
  --cetim-magenta:          #d80064;
  --glitter-roxo:           #8000ff;
  --veneno-eletrico:        #d4ff00;
  --alta-voltagem:          #ffb700;
  --fio-massa-verde:        #00ff66;
  --curto-circuito:         #ff2200;
  --papel-mimeografo:       #e8e6df;
  --tinta-carimbo:          #0a0a0b;
  --grafite-mono:           #8a909a;
}

body {
  margin: 0;
  background-color: var(--ferro-bate-pedra);
  color: var(--papel-mimeografo);
  font-family: 'JetBrains Mono', monospace, sans-serif;
}

.chassis-app {
  max-width: 900px;
  margin: 1.5rem auto;
  background: var(--ferro-painel);
  border: 3px solid var(--metal-cantoneira);
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.8);
}

.chassis-header {
  background: var(--ferro-bate-pedra);
  border-bottom: 3px solid var(--metal-cantoneira);
  padding: 0.75rem 1.25rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.85rem;
  font-weight: 700;
}

.badge-lambe {
  background: var(--fucsia-furacao);
  color: #000;
  padding: 0.2rem 0.5rem;
  font-weight: 800;
  display: inline-block;
  transform: rotate(-1.5deg);
  box-shadow: 2px 2px 0 #000;
}

.fita-isolante {
  height: 10px;
  background: repeating-linear-gradient(45deg, var(--veneno-eletrico), var(--veneno-eletrico) 10px, #000 10px, #000 20px);
  border-top: 1px solid #000;
  border-bottom: 1px solid #000;
}

.cockpit-body {
  padding: 1.5rem;
}

.station-panel {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  margin-bottom: 1.5rem;
  align-items: center;
}

.station-btn {
  background: var(--ferro-bate-pedra);
  color: var(--papel-mimeografo);
  border: 1px solid var(--metal-cantoneira);
  padding: 0.4rem 0.8rem;
  font-family: inherit;
  font-size: 0.8rem;
  cursor: pointer;
  transform: rotate(-0.5deg);
  transition: all 0.1s;
}

.station-btn.active {
  background: var(--veneno-eletrico);
  color: #000;
  font-weight: 800;
  border-color: #000;
  transform: rotate(1deg);
  box-shadow: 2px 2px 0 #000;
}

.metamero-main {
  background: var(--ferro-bate-pedra);
  border: 2px solid var(--metal-cantoneira);
  padding: 1.5rem;
  margin-bottom: 1.5rem;
  position: relative;
}

.track-display {
  margin-bottom: 1.5rem;
}

.track-title {
  font-size: 2.2rem;
  font-weight: 800;
  margin: 0;
  color: #fff;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  text-shadow: 2px 2px 0 var(--cetim-magenta);
}

.track-artist {
  font-size: 1.1rem;
  color: var(--veneno-eletrico);
  margin-top: 0.25rem;
  text-transform: uppercase;
  font-weight: 700;
}

.progress-strip {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
  font-size: 0.85rem;
}

.progress-track {
  flex: 1;
  height: 12px;
  background: #000;
  border: 1px solid var(--metal-cantoneira);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--fucsia-furacao);
}

.transport-controls {
  display: flex;
  justify-content: center;
  gap: 1.25rem;
  margin-bottom: 1.5rem;
}

.switch-btn {
  background: var(--ferro-painel);
  color: var(--papel-mimeografo);
  border: 2px solid var(--metal-cantoneira);
  padding: 0.75rem 1.5rem;
  font-family: inherit;
  font-size: 1.2rem;
  font-weight: 800;
  cursor: pointer;
  box-shadow: 3px 3px 0 #000;
  transition: all 0.1s;
}

.switch-btn:hover {
  border-color: var(--veneno-eletrico);
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 #000;
}

.switch-btn.play-toggle {
  background: var(--fucsia-furacao);
  color: #000;
  transform: rotate(-1.5deg);
  border-color: #000;
}

.switch-btn.play-toggle:hover {
  background: var(--veneno-eletrico);
}

.potentiometer-box {
  display: flex;
  align-items: center;
  gap: 1rem;
  background: #0d0e11;
  padding: 0.75rem 1.25rem;
  border: 1px solid var(--metal-cantoneira);
}

.potentiometer-box input[type="range"] {
  flex: 1;
  accent-color: var(--fucsia-furacao);
}

.gain-badge {
  background: var(--alta-voltagem);
  color: #000;
  padding: 0.2rem 0.5rem;
  font-weight: 800;
  font-size: 0.85rem;
  box-shadow: 2px 2px 0 #000;
}

.queue-section {
  background: var(--ferro-bate-pedra);
  border: 2px solid var(--metal-cantoneira);
  padding: 1.25rem;
}

.queue-header {
  font-size: 1.1rem;
  font-weight: 800;
  text-transform: uppercase;
  color: var(--veneno-eletrico);
  margin-bottom: 0.75rem;
}

.queue-item {
  display: flex;
  justify-content: space-between;
  padding: 0.6rem 0.75rem;
  border-bottom: 1px dashed var(--metal-cantoneira);
  cursor: pointer;
  font-size: 0.85rem;
}

.queue-item:hover {
  background: rgba(212, 255, 0, 0.08);
}

.queue-item.active {
  background: rgba(255, 0, 85, 0.15);
  border-left: 4px solid var(--fucsia-furacao);
  font-weight: 700;
}
"#;

#[component]
fn App() -> Element {
    let mut player_state = use_signal(PlayerState::default);
    let mut queue_controller = use_signal(QueueController::default);

    // Initial dummy queue items for cockpit testing if empty
    use_effect(move || {
        let mut qc = queue_controller.write();
        if qc.queue().is_empty() {
            qc.add_track(Track {
                id: "track-1".into(),
                title: "Construção".into(),
                artist: "Chico Buarque".into(),
                album: Some("Construção (1971)".into()),
                duration_ms: 258000,
                cover_url: None,
                stream_url: Some("/stream/track-1.opus".into()),
            });
            qc.add_track(Track {
                id: "track-2".into(),
                title: "Águas de Março".into(),
                artist: "Elis Regina & Tom Jobim".into(),
                album: Some("Elis & Tom (1974)".into()),
                duration_ms: 212000,
                cover_url: None,
                stream_url: Some("/stream/track-2.opus".into()),
            });
            qc.add_track(Track {
                id: "track-3".into(),
                title: "Chega de Saudade".into(),
                artist: "João Gilberto".into(),
                album: Some("Chega de Saudade (1959)".into()),
                duration_ms: 121000,
                cover_url: None,
                stream_url: Some("/stream/track-3.opus".into()),
            });

            if let Some(first) = qc.current_track() {
                let mut ps = player_state.write();
                ps.current_track = Some(first.clone());
                ps.duration_ms = first.duration_ms;
                ps.position_ms = 0;
            }
        }
    });

    let is_playing = matches!(player_state.read().playback_state, PlaybackState::Playing);
    let current_title = player_state
        .read()
        .current_track
        .as_ref()
        .map(|t| t.title.clone())
        .unwrap_or_else(|| "Nenhuma faixa em reprodução".to_string());
    let current_artist = player_state
        .read()
        .current_track
        .as_ref()
        .map(|t| t.artist.clone())
        .unwrap_or_else(|| "Selecione uma estação ou faixa".to_string());

    let pos_ms = player_state.read().position_ms;
    let dur_ms = player_state.read().duration_ms;
    let progress_pct = player_state.read().progress_pct();
    let current_vol = player_state.read().volume;
    let gain_11 = format_gain_11(current_vol);
    let formatted_pos = format_time(pos_ms);
    let formatted_dur = format_time(dur_ms);
    let play_icon = if is_playing { "⏸ PAUSAR" } else { "▶ TOCAR" };
    let active_station = player_state.read().active_station.clone();
    let available_stations = player_state.read().available_stations.clone();

    let toggle_playback = move |_| {
        let current = player_state.read().playback_state;
        let next = if current == PlaybackState::Playing {
            PlaybackState::Paused
        } else {
            PlaybackState::Playing
        };
        player_state.write().playback_state = next;
    };

    let next_track = move |_| {
        let mut qc = queue_controller.write();
        if let Some(next) = qc.next().cloned() {
            let mut ps = player_state.write();
            ps.duration_ms = next.duration_ms;
            ps.position_ms = 0;
            ps.current_track = Some(next);
            ps.playback_state = PlaybackState::Playing;
        }
    };

    let previous_track = move |_| {
        let mut qc = queue_controller.write();
        if let Some(prev) = qc.previous().cloned() {
            let mut ps = player_state.write();
            ps.duration_ms = prev.duration_ms;
            ps.position_ms = 0;
            ps.current_track = Some(prev);
            ps.playback_state = PlaybackState::Playing;
        }
    };

    let queue_tracks = queue_controller.read().queue().to_vec();
    let current_index = queue_controller.read().current_index();

    rsx! {
        style { "{BRUTALIST_CSS}" }

        div { class: "chassis-app",
            // Barra superior do chassi
            div { class: "chassis-header",
                div {
                    span { class: "badge-lambe", "LA CRAIA :: COCKPIT" }
                    span { style: "margin-left: 0.5rem; color: var(--grafite-mono);", "PAREDÃO & SIMBIÓTYKA" }
                }
                div { style: "display: flex; gap: 0.75rem; align-items: center;",
                    span { style: "color: var(--fio-massa-verde);", "● STATUS: OK" }
                    span { class: "gain-badge", "GANHO: {gain_11}/11" }
                }
            }

            div { class: "fita-isolante" }

            div { class: "cockpit-body",
                // Seletor de estações em fita crepe
                div { class: "station-panel",
                    span { style: "font-weight: 800; color: var(--veneno-eletrico); font-size: 0.85rem;", "[ ESTAÇÃO ]:" }
                    for station in available_stations {
                        button {
                            key: "{station}",
                            class: if station == active_station { "station-btn active" } else { "station-btn" },
                            onclick: {
                                let st = station.clone();
                                move |_| {
                                    player_state.write().select_station(st.clone());
                                }
                            },
                            "{station}"
                        }
                    }
                }

                // Metâmero Principal (Cabine de Reprodução)
                div { class: "metamero-main",
                    div { class: "track-display",
                        div { class: "track-title", "{current_title}" }
                        div { class: "track-artist", "{current_artist}" }
                    }

                    // Fita de progresso
                    div { class: "progress-strip",
                        span { "{formatted_pos}" }
                        div { class: "progress-track",
                            div {
                                class: "progress-fill",
                                style: "width: {progress_pct}%",
                            }
                        }
                        span { "{formatted_dur}" }
                    }

                    // Chaves mecânicas de controle
                    div { class: "transport-controls",
                        button {
                            class: "switch-btn",
                            style: "transform: rotate(-1.5deg);",
                            onclick: previous_track,
                            "⏮ ANTERIOR"
                        }
                        button {
                            class: "switch-btn play-toggle",
                            onclick: toggle_playback,
                            "{play_icon}"
                        }
                        button {
                            class: "switch-btn",
                            style: "transform: rotate(1deg);",
                            onclick: next_track,
                            "PRÓXIMA ⏭"
                        }
                    }

                    // Potenciômetro de ganho até 11
                    div { class: "potentiometer-box",
                        span { style: "font-weight: 800; color: var(--alta-voltagem); font-size: 0.8rem;", "POTENCIÔMETRO (0-11):" }
                        input {
                            r#type: "range",
                            min: "0",
                            max: "100",
                            value: "{(current_vol * 100.0) as u32}",
                            oninput: move |evt| {
                                if let Ok(val) = evt.value().parse::<f64>() {
                                    player_state.write().set_volume(val / 100.0);
                                }
                            },
                        }
                        span { class: "gain-badge",
                            if gain_11 >= 11 { "11 (PICO MÁXIMO!)" } else { "{gain_11}" }
                        }
                    }
                }

                // Metâmero de Fila
                div { class: "queue-section",
                    div { class: "queue-header", "[ METÂMEROS DA FILA ]" }
                    div {
                        for (idx, track) in queue_tracks.into_iter().enumerate() {
                            div {
                                key: "{track.id}",
                                class: if current_index == Some(idx) { "queue-item active" } else { "queue-item" },
                                onclick: move |_| {
                                    let mut qc = queue_controller.write();
                                    if let Some(selected) = qc.select_track(idx).cloned() {
                                        let mut ps = player_state.write();
                                        ps.current_track = Some(selected.clone());
                                        ps.duration_ms = selected.duration_ms;
                                        ps.position_ms = 0;
                                        ps.playback_state = PlaybackState::Playing;
                                    }
                                },
                                span { "[SEG.0{idx + 1}] {track.title}" }
                                span { style: "color: var(--veneno-eletrico);", "{track.artist}" }
                                span { style: "color: var(--grafite-mono);", "{format_time(track.duration_ms)}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
