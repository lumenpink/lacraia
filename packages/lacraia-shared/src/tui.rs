use crate::models::{PlaybackState, Track};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TuiTab {
    #[default]
    NowPlaying,
    Queue,
    Telemetry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    TogglePlay,
    NextTrack,
    PrevTrack,
    VolumeUp,
    VolumeDown,
    SwitchTab(TuiTab),
    NextTab,
    Quit,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiState {
    pub current_track: Option<Track>,
    pub playback_state: PlaybackState,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: u8,
    pub active_cabin: String,
    pub ws_connected: bool,
    pub queue: Vec<Track>,
    pub active_tab: TuiTab,
    pub station: String,
    pub spectrum_bands: [u8; 16],
    pub status_message: Option<String>,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            current_track: None,
            playback_state: PlaybackState::Stopped,
            position_ms: 0,
            duration_ms: 0,
            volume: 8,
            active_cabin: "default".to_string(),
            ws_connected: false,
            queue: Vec::new(),
            active_tab: TuiTab::NowPlaying,
            station: "MPB & Samba".to_string(),
            spectrum_bands: [0; 16],
            status_message: None,
        }
    }
}

impl TuiState {
    pub fn handle_key(&mut self, key: char) -> TuiAction {
        match key {
            ' ' => {
                self.playback_state = match self.playback_state {
                    PlaybackState::Playing => PlaybackState::Paused,
                    PlaybackState::Paused | PlaybackState::Stopped | PlaybackState::Buffering => {
                        PlaybackState::Playing
                    }
                };
                TuiAction::TogglePlay
            }
            'n' | 'N' => TuiAction::NextTrack,
            'p' | 'P' => TuiAction::PrevTrack,
            '+' | '=' | 'k' | 'K' => {
                if self.volume < 11 {
                    self.volume += 1;
                }
                TuiAction::VolumeUp
            }
            '-' | '_' | 'j' | 'J' => {
                if self.volume > 0 {
                    self.volume -= 1;
                }
                TuiAction::VolumeDown
            }
            '1' => {
                self.active_tab = TuiTab::NowPlaying;
                TuiAction::SwitchTab(TuiTab::NowPlaying)
            }
            '2' => {
                self.active_tab = TuiTab::Queue;
                TuiAction::SwitchTab(TuiTab::Queue)
            }
            '3' => {
                self.active_tab = TuiTab::Telemetry;
                TuiAction::SwitchTab(TuiTab::Telemetry)
            }
            '\t' => {
                self.active_tab = match self.active_tab {
                    TuiTab::NowPlaying => TuiTab::Queue,
                    TuiTab::Queue => TuiTab::Telemetry,
                    TuiTab::Telemetry => TuiTab::NowPlaying,
                };
                TuiAction::NextTab
            }
            'q' | 'Q' => TuiAction::Quit,
            _ => TuiAction::None,
        }
    }
}

pub fn is_tui_requested(args: &[String]) -> bool {
    args.iter().any(|arg| {
        let trimmed = arg.trim();
        trimmed == "tui" || trimmed == "--tui" || trimmed.starts_with("--tui=")
    })
}

pub fn format_time_ms(ms: u64) -> String {
    let total_sec = ms / 1000;
    let min = total_sec / 60;
    let sec = total_sec % 60;
    format!("{:02}:{:02}", min, sec)
}

pub fn format_progress_bar(current_ms: u64, total_ms: u64, width: usize) -> String {
    if total_ms == 0 || width == 0 {
        return "-".repeat(width);
    }
    let ratio = (current_ms as f64 / total_ms as f64).clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("{}{}", "=".repeat(filled), "-".repeat(empty))
}

pub struct TuiRenderer;

impl TuiRenderer {
    pub fn render_to_string(state: &TuiState, width: usize, _height: usize) -> String {
        let w = width.max(60);
        let mut out = String::new();

        // 1. Cabeçalho de Identidade
        out.push_str(&format!("┌{}┐\n", "─".repeat(w - 2)));
        let title_line = format!(
            "│  LA CRAIA :: COCKPIT DE ÁUDIO NO TERMINAL   [CABINE: {}]",
            state.active_cabin
        );
        out.push_str(&Self::pad_line(&title_line, w));
        out.push('\n');

        let status_str = if state.ws_connected {
            "WS :: CONECTADO"
        } else {
            "WS :: MODO AUTÔNOMO LOCAL"
        };
        let status_line = format!(
            "│  STATUS: {:<27}  GANHO: VOL: {}/11",
            status_str, state.volume
        );
        out.push_str(&Self::pad_line(&status_line, w));
        out.push('\n');
        out.push_str(&format!("├{}┤\n", "─".repeat(w - 2)));

        // 2. Barra de Abas
        let t1 = if state.active_tab == TuiTab::NowPlaying {
            "[1: NOW PLAYING *]"
        } else {
            "[1: NOW PLAYING]"
        };
        let t2 = if state.active_tab == TuiTab::Queue {
            format!("[2: FILA ({}) *]", state.queue.len())
        } else {
            format!("[2: FILA ({})]", state.queue.len())
        };
        let t3 = if state.active_tab == TuiTab::Telemetry {
            "[3: TELEMETRIA *]"
        } else {
            "[3: TELEMETRIA]"
        };
        let tabs_line = format!("│  {}   {}   {}", t1, t2, t3);
        out.push_str(&Self::pad_line(&tabs_line, w));
        out.push('\n');
        out.push_str(&format!("├{}┤\n", "─".repeat(w - 2)));

        // 3. Conteúdo da Aba Ativa
        match state.active_tab {
            TuiTab::NowPlaying => {
                let track_title = state
                    .current_track
                    .as_ref()
                    .map(|t| t.title.as_str())
                    .unwrap_or("Nenhuma faixa em reprodução");
                let track_artist = state
                    .current_track
                    .as_ref()
                    .map(|t| t.artist.as_str())
                    .unwrap_or("--");
                let state_label = match state.playback_state {
                    PlaybackState::Playing => "TOCANDO ▶",
                    PlaybackState::Paused => "PAUSADO ⏸",
                    PlaybackState::Stopped => "PARADO ■",
                    PlaybackState::Buffering => "CARREGANDO ⏳",
                };

                out.push_str(&Self::pad_line(&format!("│  FAIXA   : {}", track_title), w));
                out.push('\n');
                out.push_str(&Self::pad_line(
                    &format!("│  ARTISTA : {}", track_artist),
                    w,
                ));
                out.push('\n');
                out.push_str(&Self::pad_line(
                    &format!(
                        "│  ESTADO  : {:<12} ESTAÇÃO: {}",
                        state_label, state.station
                    ),
                    w,
                ));
                out.push('\n');

                let bar_width = 30;
                let prog_bar = format_progress_bar(state.position_ms, state.duration_ms, bar_width);
                let time_str = format!(
                    "{} / {}",
                    format_time_ms(state.position_ms),
                    format_time_ms(state.duration_ms)
                );
                out.push_str(&Self::pad_line(
                    &format!("│  TEMPO   : {} [{}]", time_str, prog_bar),
                    w,
                ));
                out.push('\n');

                // Visualizador de Espectro ANSI (16 bandas)
                let block_chars = [' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
                let visualizer: String = state
                    .spectrum_bands
                    .iter()
                    .map(|&b| {
                        let idx = (b as usize).min(block_chars.len() - 1);
                        block_chars[idx]
                    })
                    .collect();
                out.push_str(&Self::pad_line(
                    &format!("│  ESPECTRO: {} 32-BAR CORE", visualizer),
                    w,
                ));
                out.push('\n');
            }
            TuiTab::Queue => {
                out.push_str(&Self::pad_line("│  PRÓXIMAS FAIXAS NA FILA:", w));
                out.push('\n');
                if state.queue.is_empty() {
                    out.push_str(&Self::pad_line(
                        "│    (fila vazia - piloto automático atuando)",
                        w,
                    ));
                    out.push('\n');
                } else {
                    for (i, t) in state.queue.iter().take(5).enumerate() {
                        let req = t.requested_by.as_deref().unwrap_or("SISTEMA");
                        let dur = format_time_ms(t.duration_ms);
                        let item_line = format!(
                            "│    {:02}. {} - {} ({}) [{}]",
                            i + 1,
                            t.title,
                            t.artist,
                            dur,
                            req
                        );
                        out.push_str(&Self::pad_line(&item_line, w));
                        out.push('\n');
                    }
                }
            }
            TuiTab::Telemetry => {
                out.push_str(&Self::pad_line("│  MÉTRICAS & DIAGNÓSTICO DO CORE:", w));
                out.push('\n');
                out.push_str(&Self::pad_line(
                    &format!("│    Cabine Ativa        : {}", state.active_cabin),
                    w,
                ));
                out.push('\n');
                out.push_str(&Self::pad_line(
                    &format!(
                        "│    Canal Telemetria    : {}",
                        if state.ws_connected {
                            "Ativo (WebSocket)"
                        } else {
                            "Offline (Cache Local)"
                        }
                    ),
                    w,
                ));
                out.push('\n');
                out.push_str(&Self::pad_line(
                    &format!("│    Volume Calibrado    : {}/11", state.volume),
                    w,
                ));
                out.push('\n');
            }
        }

        // 4. Rodapé com Atalhos
        out.push_str(&format!("├{}┤\n", "─".repeat(w - 2)));
        let footer =
            "│  [Espaço] Play/Pause | [n] Próxima | [p] Anterior | [+/-] Vol | [1-3] Abas | [q] Sair";
        out.push_str(&Self::pad_line(footer, w));
        out.push('\n');
        out.push_str(&format!("└{}┘\n", "─".repeat(w - 2)));

        out
    }

    fn pad_line(line: &str, target_width: usize) -> String {
        // Contagem real de caracteres visuais (evitando desvios com unicode simples)
        let count = line.chars().count();
        if count >= target_width {
            let mut truncated: String = line.chars().take(target_width - 1).collect();
            truncated.push('│');
            truncated
        } else {
            let padding = target_width - count - 1;
            format!("{}{}{}", line, " ".repeat(padding), "│")
        }
    }
}

pub fn run_interactive_tui(mut state: TuiState) {
    use std::io::{stdout, IsTerminal, Write};
    let w = 80;
    let h = 24;
    print!("\x1b[2J\x1b[H");
    print!("{}", TuiRenderer::render_to_string(&state, w, h));
    let _ = stdout().flush();

    if !std::io::stdin().is_terminal() {
        return;
    }

    let stdin = std::io::stdin();
    let mut input = String::new();
    loop {
        input.clear();
        if stdin.read_line(&mut input).is_err() {
            break;
        }
        let mut should_quit = false;
        for ch in input.chars() {
            let action = state.handle_key(ch);
            if action == TuiAction::Quit {
                should_quit = true;
                break;
            }
        }
        if should_quit {
            println!("Saindo da interface TUI...");
            break;
        }
        print!("\x1b[2J\x1b[H");
        print!("{}", TuiRenderer::render_to_string(&state, w, h));
        let _ = stdout().flush();
    }
}
