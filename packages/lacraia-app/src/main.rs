pub mod audio;
pub mod ui;

use dioxus::prelude::*;
use lacraia_shared::{PlaybackState, Track};
use ui::{format_time, PlayerState, QueueController};

fn main() {
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}

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
    let vol_pct = (current_vol * 100.0) as u32;
    let formatted_pos = format_time(pos_ms);
    let formatted_dur = format_time(dur_ms);
    let play_icon = if is_playing { "⏸" } else { "▶" };
    let play_label = if is_playing { "Pausar" } else { "Reproduzir" };
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
        div { class: "app-container automotive-theme",
            header { class: "player-header",
                h1 { "La Craia" }
                p { class: "tagline", "Áudio & HUD Veicular Descentralizado" }
            }

            // Station Selector
            div { class: "station-picker",
                span { class: "station-label", "Estação:" }
                div { class: "station-buttons",
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
            }

            main { class: "player-card",
                div { class: "track-info",
                    h2 { class: "track-title", "{current_title}" }
                    p { class: "track-artist", "{current_artist}" }
                }

                // Progress Bar
                div { class: "progress-container",
                    span { class: "time-label", "{formatted_pos}" }
                    div { class: "progress-bar-bg",
                        div {
                            class: "progress-bar-fill",
                            style: "width: {progress_pct}%",
                        }
                    }
                    span { class: "time-label", "{formatted_dur}" }
                }

                // Playback Controls
                div { class: "controls",
                    button {
                        class: "control-btn",
                        aria_label: "Faixa anterior",
                        onclick: previous_track,
                        "⏮"
                    }
                    button {
                        class: "control-btn play-btn",
                        aria_label: "{play_label}",
                        onclick: toggle_playback,
                        "{play_icon}"
                    }
                    button {
                        class: "control-btn",
                        aria_label: "Próxima faixa",
                        onclick: next_track,
                        "⏭"
                    }
                }

                // Volume slider
                div { class: "volume-container",
                    span { class: "volume-icon", "🔊" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "100",
                        value: "{vol_pct}",
                        oninput: move |evt| {
                            if let Ok(val) = evt.value().parse::<f64>() {
                                player_state.write().set_volume(val / 100.0);
                            }
                        },
                    }
                    span { class: "volume-value", "{vol_pct}%" }
                }
            }

            // Up Next Queue Section
            section { class: "queue-card",
                h3 { class: "queue-title", "Fila de Reprodução" }
                ul { class: "queue-list",
                    for (idx, track) in queue_tracks.into_iter().enumerate() {
                        li {
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
                            span { class: "queue-item-title", "{track.title}" }
                            span { class: "queue-item-artist", "{track.artist}" }
                            span { class: "queue-item-duration", "{format_time(track.duration_ms)}" }
                        }
                    }
                }
            }
        }
    }
}
