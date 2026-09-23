use lacraia_shared::{PlaybackState, StateUpdate, Track, WsMessage};
use serde::{Deserialize, Serialize};

/// Formats millisecond duration into "mm:ss" display string
pub fn format_time(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

/// Clamps volume float between 0.0 and 1.0
pub fn clamp_volume(volume: f64) -> f64 {
    if volume < 0.0 {
        0.0
    } else if volume > 1.0 {
        1.0
    } else {
        volume
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueueController {
    queue: Vec<Track>,
    current_index: Option<usize>,
    station_seed: Option<String>,
}

impl QueueController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_station_seed(&mut self, seed: String) {
        self.station_seed = Some(seed);
    }

    pub fn station_seed(&self) -> Option<&str> {
        self.station_seed.as_deref()
    }

    pub fn queue(&self) -> &[Track] {
        &self.queue
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.current_index.and_then(|idx| self.queue.get(idx))
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    pub fn add_track(&mut self, track: Track) {
        self.queue.push(track);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    pub fn set_queue(&mut self, queue: Vec<Track>) {
        self.queue = queue;
        if !self.queue.is_empty() {
            self.current_index = Some(0);
        } else {
            self.current_index = None;
        }
    }

    pub fn select_track(&mut self, index: usize) -> Option<&Track> {
        if index < self.queue.len() {
            self.current_index = Some(index);
            self.current_track()
        } else {
            None
        }
    }

    pub fn next(&mut self) -> Option<&Track> {
        if let Some(idx) = self.current_index {
            if idx + 1 < self.queue.len() {
                self.current_index = Some(idx + 1);
                return self.current_track();
            }
        }
        None
    }

    pub fn previous(&mut self) -> Option<&Track> {
        if let Some(idx) = self.current_index {
            if idx > 0 {
                self.current_index = Some(idx - 1);
                return self.current_track();
            }
        }
        self.current_track()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.current_index = None;
    }
}

/// Reactive State Representation for the Automotive Web Player
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerState {
    pub current_track: Option<Track>,
    pub playback_state: PlaybackState,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f64,
    pub active_station: String,
    pub available_stations: Vec<String>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            current_track: None,
            playback_state: PlaybackState::Stopped,
            position_ms: 0,
            duration_ms: 0,
            volume: 0.8,
            active_station: "MPB & Samba".to_string(),
            available_stations: vec![
                "MPB & Samba".to_string(),
                "Synthwave Drive".to_string(),
                "Rock Clássico".to_string(),
                "Lo-Fi Road Trip".to_string(),
            ],
        }
    }
}

impl PlayerState {
    pub fn handle_ws_message(&mut self, msg: WsMessage) {
        match msg {
            WsMessage::StateUpdate(StateUpdate { payload }) => {
                if let Some(t) = payload.track {
                    self.duration_ms = t.duration_ms;
                    self.current_track = Some(t);
                }
                self.position_ms = payload.playback.position_ms;
                self.playback_state = payload.playback.state;
            }
            WsMessage::Command(_) => {}
        }
    }

    pub fn set_volume(&mut self, vol: f64) {
        self.volume = clamp_volume(vol);
    }

    pub fn progress_pct(&self) -> f64 {
        if self.duration_ms == 0 {
            0.0
        } else {
            (self.position_ms as f64 / self.duration_ms as f64).clamp(0.0, 1.0) * 100.0
        }
    }

    pub fn select_station(&mut self, station: String) {
        self.active_station = station;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lacraia_shared::{PlaybackDetails, StatePayload};

    #[test]
    fn test_format_time() {
        assert_eq!(format_time(0), "00:00");
        assert_eq!(format_time(9000), "00:09");
        assert_eq!(format_time(65000), "01:05");
        assert_eq!(format_time(274000), "04:34");
        assert_eq!(format_time(3600000), "60:00");
    }

    #[test]
    fn test_clamp_volume() {
        assert_eq!(clamp_volume(-0.5), 0.0);
        assert_eq!(clamp_volume(0.0), 0.0);
        assert_eq!(clamp_volume(0.65), 0.65);
        assert_eq!(clamp_volume(1.0), 1.0);
        assert_eq!(clamp_volume(1.5), 1.0);
    }

    #[test]
    fn test_queue_controller_lifecycle() {
        let mut qc = QueueController::new();
        assert!(qc.current_track().is_none());

        let t1 = Track {
            id: "1".into(),
            title: "Song 1".into(),
            artist: "Artist".into(),
            album: None,
            duration_ms: 180000,
            cover_url: None,
            stream_url: None,
        };
        let t2 = Track {
            id: "2".into(),
            title: "Song 2".into(),
            artist: "Artist".into(),
            album: None,
            duration_ms: 210000,
            cover_url: None,
            stream_url: None,
        };

        qc.add_track(t1);
        qc.add_track(t2);

        assert_eq!(qc.queue().len(), 2);
        assert_eq!(qc.current_index(), Some(0));
        assert_eq!(qc.current_track().unwrap().title, "Song 1");

        // Advance next
        let next = qc.next().unwrap();
        assert_eq!(next.title, "Song 2");
        assert_eq!(qc.current_index(), Some(1));

        // Advance beyond end
        assert!(qc.next().is_none());

        // Previous
        let prev = qc.previous().unwrap();
        assert_eq!(prev.title, "Song 1");
        assert_eq!(qc.current_index(), Some(0));

        // Direct select
        assert!(qc.select_track(1).is_some());
        assert_eq!(qc.current_index(), Some(1));
        assert!(qc.select_track(5).is_none());

        qc.clear();
        assert_eq!(qc.queue().len(), 0);
        assert!(qc.current_track().is_none());
    }

    #[test]
    fn test_player_state_and_telemetry() {
        let mut state = PlayerState::default();
        assert_eq!(state.volume, 0.8);
        assert_eq!(state.progress_pct(), 0.0);

        let t = Track {
            id: "test-id".into(),
            title: "Construção".into(),
            artist: "Chico Buarque".into(),
            album: Some("Construção".into()),
            duration_ms: 250000,
            cover_url: None,
            stream_url: None,
        };

        let msg = WsMessage::StateUpdate(StateUpdate {
            payload: StatePayload {
                track: Some(t.clone()),
                playback: PlaybackDetails {
                    state: PlaybackState::Playing,
                    position_ms: 125000,
                    updated_at: 1727047200000,
                },
                has_lrc: true,
            },
        });

        state.handle_ws_message(msg);
        assert_eq!(state.playback_state, PlaybackState::Playing);
        assert_eq!(state.position_ms, 125000);
        assert_eq!(state.duration_ms, 250000);
        assert_eq!(state.progress_pct(), 50.0);
        assert_eq!(state.current_track.as_ref().unwrap().title, "Construção");

        state.set_volume(1.2);
        assert_eq!(state.volume, 1.0);
        state.set_volume(-0.1);
        assert_eq!(state.volume, 0.0);

        state.select_station("Synthwave Drive".to_string());
        assert_eq!(state.active_station, "Synthwave Drive");
    }
}
