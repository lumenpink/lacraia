use dioxus::prelude::*;
use lacraia_shared::{PlaybackState, Track};

fn main() {
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let current_track = use_signal(|| Option::<Track>::None);
    let mut playback_state = use_signal(|| PlaybackState::Stopped);

    let is_playing = matches!(*playback_state.read(), PlaybackState::Playing);

    let toggle_playback = move |_| {
        let current = *playback_state.read();
        let next = if current == PlaybackState::Playing {
            PlaybackState::Paused
        } else {
            PlaybackState::Playing
        };
        *playback_state.write() = next;
    };

    rsx! {
        div { class: "app-container",
            header { class: "player-header",
                h1 { "La Craia" }
                p { class: "tagline", "Áudio & HUD Veicular Descentralizado" }
            }

            main { class: "player-card",
                div { class: "track-info",
                    h2 { class: "track-title",
                        {
                            match &*current_track.read() {
                                Some(t) => t.title.clone(),
                                None => "Nenhuma faixa em reprodução".to_string(),
                            }
                        }
                    }
                    p { class: "track-artist",
                        {
                            match &*current_track.read() {
                                Some(t) => t.artist.clone(),
                                None => "Selecione uma estação ou artista".to_string(),
                            }
                        }
                    }
                }

                div { class: "controls",
                    button {
                        class: "control-btn",
                        onclick: move |_| {},
                        "⏮"
                    }
                    button {
                        class: "control-btn play-btn",
                        onclick: toggle_playback,
                        if is_playing { "⏸" } else { "▶" }
                    }
                    button {
                        class: "control-btn",
                        onclick: move |_| {},
                        "⏭"
                    }
                }
            }
        }
    }
}
