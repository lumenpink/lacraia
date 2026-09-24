pub mod audio;
pub mod ui;

use dioxus::prelude::*;
use lacraia_shared::{PlaybackState, Track};
use ui::{format_gain_11, format_time, PlayerState, QueueController, VisualizerState};

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

.visualizer-box {
  background: #0d0e11;
  border: 1px solid var(--metal-cantoneira);
  padding: 0.5rem 0.75rem;
  margin: 1rem 0;
}

.visualizer-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.4rem;
}

.btn-vis-toggle {
  background: var(--ferro-bate-pedra);
  border: 1px solid var(--metal-cantoneira);
  color: var(--papel-mimeografo);
  font-family: inherit;
  font-size: 0.7rem;
  font-weight: 800;
  padding: 0.2rem 0.5rem;
  cursor: pointer;
}

.btn-vis-toggle.active {
  border-color: var(--veneno-eletrico);
  color: var(--veneno-eletrico);
}

.visualizer-bars {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 48px;
  background: #050607;
  padding: 2px;
  border: 1px solid #1a1c21;
}

.vis-bar {
  flex: 1;
  min-height: 2px;
  transition: height 0.08s ease-out;
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

.door-toggle {
  background: var(--ferro-bate-pedra);
  border: 1px solid var(--metal-cantoneira);
  padding: 0.3rem 0.6rem;
  font-family: inherit;
  font-size: 0.75rem;
  font-weight: 800;
  cursor: pointer;
  box-shadow: 2px 2px 0 #000;
  transition: all 0.1s;
}

.door-toggle.unlocked {
  color: var(--fio-massa-verde);
  border-color: var(--fio-massa-verde);
}

.door-toggle.locked {
  color: var(--curto-circuito);
  border-color: var(--curto-circuito);
  background: rgba(255, 34, 0, 0.15);
}

.btn-embarque {
  background: var(--veneno-eletrico);
  color: #000;
  border: 1px solid #000;
  padding: 0.3rem 0.6rem;
  font-family: inherit;
  font-size: 0.75rem;
  font-weight: 800;
  cursor: pointer;
  box-shadow: 2px 2px 0 #000;
  margin-left: 0.5rem;
}

.qr-modal {
  background: #111215;
  border: 2px solid var(--veneno-eletrico);
  padding: 1.25rem;
  margin-bottom: 1.5rem;
  box-shadow: 0 0 15px rgba(212, 255, 0, 0.15);
  text-align: center;
}

.qr-code-svg {
  display: inline-block;
  margin: 0.75rem auto;
  border: 3px solid #000;
}

.badge-vivente {
  background: rgba(212, 255, 0, 0.15);
  border: 1px solid var(--veneno-eletrico);
  color: var(--veneno-eletrico);
  font-size: 0.7rem;
  padding: 0.1rem 0.4rem;
  font-weight: 700;
  margin-left: 0.5rem;
  display: inline-block;
}

.crew-section {
  background: var(--ferro-bate-pedra);
  border: 2px solid var(--metal-cantoneira);
  padding: 1.25rem;
  margin-top: 1.5rem;
}

.crew-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.6rem 0.75rem;
  border-bottom: 1px dashed var(--metal-cantoneira);
  font-size: 0.85rem;
}

.btn-eject {
  background: var(--curto-circuito);
  color: #fff;
  border: 1px solid #000;
  padding: 0.2rem 0.5rem;
  font-family: inherit;
  font-size: 0.75rem;
  font-weight: 800;
  cursor: pointer;
}

.btn-eject-all {
  background: var(--curto-circuito);
  color: #fff;
  border: 2px solid #000;
  padding: 0.6rem 1rem;
  font-family: inherit;
  font-size: 0.85rem;
  font-weight: 800;
  cursor: pointer;
  margin-top: 1rem;
  width: 100%;
  box-shadow: 3px 3px 0 #000;
}

.ejected-screen {
  padding: 3rem 1.5rem;
  text-align: center;
  background: #111215;
  border: 3px solid var(--curto-circuito);
  box-shadow: 0 0 25px rgba(255, 34, 0, 0.4);
}
"#;

#[component]
fn App() -> Element {
    let mut player_state = use_signal(PlayerState::default);
    let mut queue_controller = use_signal(QueueController::default);
    let mut visualizer_state = use_signal(VisualizerState::default);

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
                requested_by: Some("PILOTO".into()),
            });
            qc.add_track(Track {
                id: "track-2".into(),
                title: "Águas de Março".into(),
                artist: "Elis Regina & Tom Jobim".into(),
                album: Some("Elis & Tom (1974)".into()),
                duration_ms: 212000,
                cover_url: None,
                stream_url: Some("/stream/track-2.opus".into()),
                requested_by: Some("@Mari".into()),
            });
            qc.add_track(Track {
                id: "track-3".into(),
                title: "Chega de Saudade".into(),
                artist: "João Gilberto".into(),
                album: Some("Chega de Saudade (1959)".into()),
                duration_ms: 121000,
                cover_url: None,
                stream_url: Some("/stream/track-3.opus".into()),
                requested_by: Some("@Paulinho".into()),
            });

            if let Some(first) = qc.current_track() {
                let mut ps = player_state.write();
                ps.current_track = Some(first.clone());
                ps.duration_ms = first.duration_ms;
                ps.position_ms = 0;
            }
        }
    });

    let is_ejected = player_state.read().is_ejected;
    let ejection_reason = player_state.read().ejection_reason.clone();
    let is_locked = player_state.read().is_locked;
    let cabin_id = player_state.read().cabin_id.clone();
    let show_qr_modal = player_state.read().show_qr_modal;
    let crew = player_state.read().crew.clone();

    if is_ejected {
        let reason_text = ejection_reason.unwrap_or_else(|| "Aqui no paredão quem manda na agulha é o piloto. Comporte-se melhor na próxima curva.".into());
        return rsx! {
            style { "{BRUTALIST_CSS}" }
            div { class: "chassis-app",
                div { class: "ejected-screen",
                    div { style: "font-size: 3rem; margin-bottom: 1rem;", "🦂💥" }
                    h1 { style: "color: var(--curto-circuito); font-size: 1.8rem; text-transform: uppercase; margin: 0 0 1rem 0;", "VOCÊ FOI EJETADO DA CABINE!" }
                    p { style: "color: var(--papel-mimeografo); font-size: 1.05rem; max-width: 580px; margin: 1.5rem auto; line-height: 1.5;",
                        "{reason_text}"
                    }
                    div { style: "margin-top: 2rem; color: var(--alta-voltagem); font-weight: 700; font-size: 0.85rem;",
                        "[ SINAL DE ÁUDIO CORTADO • TRANSMISSÃO DE CABINE ENCERRADA ]"
                    }
                }
            }
        };
    }

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
    let is_vis_active = visualizer_state.read().is_active();
    let bar_items: Vec<(usize, u32, &'static str)> = (0..32)
        .map(|idx| {
            let height = if !is_vis_active {
                2
            } else if is_playing {
                6 + ((idx as u32 * 7) % 36)
            } else {
                3
            };
            let color = if idx % 2 == 0 {
                "var(--fucsia-furacao)"
            } else {
                "var(--veneno-eletrico)"
            };
            (idx, height, color)
        })
        .collect();

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
            // Barra superior do chassi com controles de cabine
            div { class: "chassis-header",
                div { style: "display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap;",
                    span { class: "badge-lambe", "LA CRAIA :: COCKPIT" }
                    span { style: "color: var(--veneno-eletrico); font-weight: 700; font-size: 0.8rem;", "[CABINE: {cabin_id}]" }
                }
                div { style: "display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap;",
                    button {
                        class: if is_locked { "door-toggle locked" } else { "door-toggle unlocked" },
                        onclick: move |_| {
                            let cur = player_state.read().is_locked;
                            player_state.write().is_locked = !cur;
                        },
                        if is_locked { "🔒 PORTAS TRANCADAS" } else { "🔓 PORTAS ABERTAS" }
                    }
                    button {
                        class: "btn-embarque",
                        onclick: move |_| {
                            let cur = player_state.read().show_qr_modal;
                            player_state.write().show_qr_modal = !cur;
                        },
                        if show_qr_modal { "[ ✕ FECHAR EMBARQUE ]" } else { "[ 📲 EMBARQUE / QR ]" }
                    }
                    span { class: "gain-badge", "GANHO: {gain_11}/11" }
                }
            }

            div { class: "fita-isolante" }

            div { class: "cockpit-body",
                // Modal de QR Code e Embarque de Passageiros (Toggleável)
                if show_qr_modal {
                    div { class: "qr-modal",
                        div { style: "color: var(--veneno-eletrico); font-weight: 800; font-size: 1.1rem; text-transform: uppercase;",
                            "📲 EMBARQUE DE PASSAGEIRO • CABINE: {cabin_id}"
                        }
                        p { style: "font-size: 0.85rem; color: var(--papel-mimeografo); margin: 0.5rem 0;",
                            "Aponte a câmera do celular para pedir cantigas na van ou copie o link fixo:"
                        }

                        // Representação vetorial SVG estilizada de QR Code
                        svg {
                            class: "qr-code-svg",
                            width: "160",
                            height: "160",
                            view_box: "0 0 160 160",
                            rect { x: "0", y: "0", width: "160", height: "160", fill: "#d4ff00" }
                            rect { x: "12", y: "12", width: "40", height: "40", fill: "#151618" }
                            rect { x: "20", y: "20", width: "24", height: "24", fill: "#d4ff00" }
                            rect { x: "108", y: "12", width: "40", height: "40", fill: "#151618" }
                            rect { x: "116", y: "20", width: "24", height: "24", fill: "#d4ff00" }
                            rect { x: "12", y: "108", width: "40", height: "40", fill: "#151618" }
                            rect { x: "20", y: "116", width: "24", height: "24", fill: "#d4ff00" }
                            // Dados centrais estilizados
                            rect { x: "64", y: "20", width: "12", height: "12", fill: "#151618" }
                            rect { x: "84", y: "20", width: "12", height: "24", fill: "#151618" }
                            rect { x: "64", y: "44", width: "32", height: "12", fill: "#151618" }
                            rect { x: "20", y: "64", width: "24", height: "12", fill: "#151618" }
                            rect { x: "56", y: "68", width: "48", height: "24", fill: "#151618" }
                            rect { x: "116", y: "64", width: "24", height: "12", fill: "#151618" }
                            rect { x: "64", y: "104", width: "12", height: "36", fill: "#151618" }
                            rect { x: "88", y: "104", width: "20", height: "12", fill: "#151618" }
                            rect { x: "120", y: "104", width: "20", height: "36", fill: "#151618" }
                            rect { x: "88", y: "128", width: "20", height: "20", fill: "#151618" }
                        }

                        div { style: "background: #000; padding: 0.5rem; border: 1px dashed var(--metal-cantoneira); font-size: 0.8rem; color: var(--veneno-eletrico); word-break: break-all;",
                            "https://lacraia.2lp.in/app?c={cabin_id}&join=1"
                        }

                        if is_locked {
                            div { style: "color: var(--curto-circuito); font-weight: 800; font-size: 0.85rem; margin-top: 0.75rem;",
                                "⚠️ ATENÇÃO: PORTAS TRANCADAS — Novos embarques serão recusados pelo piloto."
                            }
                        } else {
                            div { style: "color: var(--fio-massa-verde); font-weight: 800; font-size: 0.85rem; margin-top: 0.75rem;",
                                "✅ PORTAS ABERTAS — Passageiros autorizados a embarcar e pedir cantigas."
                            }
                        }
                    }
                }

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

                    // Visualizador de Espectro Metamérico (32 Patas do Bicho)
                    div { class: "visualizer-box",
                        div { class: "visualizer-meta",
                            span { style: "font-size: 0.75rem; font-weight: 800; color: var(--grafite-mono);", "ESPECTRO ACÚSTICO // 32 PATAS" }
                            button {
                                class: if is_vis_active { "btn-vis-toggle active" } else { "btn-vis-toggle" },
                                onclick: move |_| {
                                    visualizer_state.write().toggle();
                                },
                                if is_vis_active { "⚡ PATAS: ON" } else { "⚡ PATAS: OFF" }
                            }
                        }
                        div { class: "visualizer-bars",
                            for item in bar_items {
                                div {
                                    key: "{item.0}",
                                    class: "vis-bar",
                                    style: "height: {item.1}px; background: {item.2};",
                                }
                            }
                        }
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

                // Metâmero de Fila (com identificação do Vivente que pediu a cantiga)
                div { class: "queue-section",
                    div { class: "queue-header", "[ METÂMEROS DA FILA :: QUEM PEDIU A CANTIGA ]" }
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
                                div { style: "display: flex; align-items: center;",
                                    span { "[SEG.0{idx + 1}] {track.title}" }
                                    if let Some(ref vivente) = track.requested_by {
                                        span { class: "badge-vivente", "por: {vivente}" }
                                    }
                                }
                                span { style: "color: var(--veneno-eletrico);", "{track.artist}" }
                                span { style: "color: var(--grafite-mono);", "{format_time(track.duration_ms)}" }
                            }
                        }
                    }
                }

                // Metâmero de Tripulação & Alavanca de Ejeção de Passageiros
                div { class: "crew-section",
                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem; flex-wrap: wrap; gap: 0.5rem;",
                        div { class: "queue-header", style: "margin: 0;", "[ MANIFESTO DE BORDO :: TRIPULAÇÃO ({crew.len()}) ]" }
                        if !crew.is_empty() {
                            button {
                                class: "btn-eject",
                                onclick: move |_| {
                                    player_state.write().crew.clear();
                                },
                                "[ 💥 EJETAR GERAL ]"
                            }
                        }
                    }
                    if crew.is_empty() {
                        div { style: "color: var(--grafite-mono); font-size: 0.85rem; padding: 0.5rem 0;",
                            "Nenhum passageiro conectado. Clique em [ 📲 EMBARQUE / QR ] no painel superior para exibir o QR Code."
                        }
                    } else {
                        div {
                            for p in crew.iter() {
                                {
                                    let pid = p.id.clone();
                                    let pname = p.name.clone();
                                    let device_name = p.device.clone().unwrap_or_else(|| "Dispositivo".into());
                                    rsx! {
                                        div {
                                            key: "{pid}",
                                            class: "crew-item",
                                            span { "🟢 {pname} ({device_name})" }
                                            button {
                                                class: "btn-eject",
                                                onclick: move |_| {
                                                    player_state.write().crew.retain(|item| item.id != pid);
                                                },
                                                "[ 👢 CHUTAR ]"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
