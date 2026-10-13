use lacraia_shared::{PassengerInfo, PlaybackState, Role, StateUpdate, Track, WsMessage};
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

/// Formats volume float (0.0 to 1.0) into the legendary 0 to 11 gain scale
pub fn format_gain_11(volume: f64) -> u32 {
    let clamped = clamp_volume(volume);
    (clamped * 11.0).round() as u32
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
    pub role: Role,
    pub is_locked: bool,
    pub cabin_id: String,
    pub crew: Vec<PassengerInfo>,
    pub is_ejected: bool,
    pub ejection_reason: Option<String>,
    pub show_qr_modal: bool,
    pub queue: Vec<lacraia_shared::QueueItem>,
    pub cache_alert: Option<lacraia_shared::CacheAlertInfo>,
    pub non_stop: bool,
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
            role: Role::Pilot,
            is_locked: false,
            cabin_id: "default".to_string(),
            crew: vec![],
            is_ejected: false,
            ejection_reason: None,
            show_qr_modal: false,
            queue: vec![],
            cache_alert: None,
            non_stop: true,
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
                if let Some(cid) = payload.cabin_id {
                    self.cabin_id = cid;
                }
                if let Some(locked) = payload.is_locked {
                    self.is_locked = locked;
                }
                if let Some(ns) = payload.non_stop {
                    self.non_stop = ns;
                }
                self.queue = payload.queue;
            }
            WsMessage::CrewUpdate(crew_update) => {
                self.cabin_id = crew_update.cabin_id;
                self.is_locked = crew_update.is_locked;
                self.crew = crew_update.passengers;
            }
            WsMessage::Ejected(ejected) => {
                self.is_ejected = true;
                self.ejection_reason = Some(ejected.reason);
            }
            WsMessage::CabinLocked(_) => {
                self.is_locked = true;
            }
            WsMessage::CacheAlert(alert) => {
                if alert.active {
                    self.cache_alert = Some(lacraia_shared::CacheAlertInfo {
                        starvation_type: alert.starvation_type,
                        message: alert.message,
                        track_id: alert.track_id,
                    });
                } else {
                    self.cache_alert = None;
                }
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

/// Real-time WebAudio 32-segment spectrum analyzer state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualizerState {
    pub enabled: bool,
    pub bands_count: usize,
    pub peak_decay_rate: f32,
    pub amplitudes: Vec<f32>,
}

impl Default for VisualizerState {
    fn default() -> Self {
        Self {
            enabled: true,
            bands_count: 32,
            peak_decay_rate: 0.88,
            amplitudes: vec![0.0; 32],
        }
    }
}

impl VisualizerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle(&mut self) -> bool {
        self.enabled = !self.enabled;
        if !self.enabled {
            self.reset();
        }
        self.enabled
    }

    pub fn is_active(&self) -> bool {
        self.enabled
    }

    pub fn reset(&mut self) {
        for amp in &mut self.amplitudes {
            *amp = 0.0;
        }
    }

    pub fn update_amplitudes(&mut self, new_raw_bands: &[f32]) {
        if !self.enabled {
            self.reset();
            return;
        }

        for (i, amp) in self.amplitudes.iter_mut().enumerate() {
            let target = new_raw_bands.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            if target > *amp {
                *amp = target;
            } else {
                *amp = (*amp * self.peak_decay_rate + target * (1.0 - self.peak_decay_rate)).clamp(0.0, 1.0);
            }
        }
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
            requested_by: Some("@Mari".into()),
        };
        let t2 = Track {
            id: "2".into(),
            title: "Song 2".into(),
            artist: "Artist".into(),
            album: None,
            duration_ms: 210000,
            cover_url: None,
            stream_url: None,
            requested_by: None,
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
            requested_by: Some("PILOTO".into()),
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
                cabin_id: Some("van-furacao".into()),
                is_locked: Some(false),
                non_stop: Some(true),
                queue: vec![],
            },
        });

        state.handle_ws_message(msg);
        assert_eq!(state.playback_state, PlaybackState::Playing);
        assert_eq!(state.position_ms, 125000);
        assert_eq!(state.duration_ms, 250000);
        assert_eq!(state.progress_pct(), 50.0);
        assert_eq!(state.current_track.as_ref().unwrap().title, "Construção");
        assert_eq!(state.cabin_id, "van-furacao");
        assert!(!state.is_locked);
        assert!(state.non_stop);

        // Test CrewUpdate message
        let crew_msg = WsMessage::CrewUpdate(lacraia_shared::CrewUpdate {
            cabin_id: "van-furacao".into(),
            is_locked: true,
            passengers: vec![PassengerInfo {
                id: "psg_1".into(),
                name: "Mari".into(),
                device: Some("iPhone 14".into()),
                joined_at: 1727048000000,
            }],
        });
        state.handle_ws_message(crew_msg);
        assert!(state.is_locked);
        assert_eq!(state.crew.len(), 1);
        assert_eq!(state.crew[0].name, "Mari");

        // Test Ejected message
        let eject_msg = WsMessage::Ejected(lacraia_shared::EjectedNotification {
            cabin_id: "van-furacao".into(),
            reason: "Chute da cabine pelo piloto".into(),
        });
        state.handle_ws_message(eject_msg);
        assert!(state.is_ejected);
        assert_eq!(state.ejection_reason.as_deref(), Some("Chute da cabine pelo piloto"));

        // Test CacheAlert message
        let alert_msg = WsMessage::CacheAlert(lacraia_shared::CacheAlertNotification {
            cabin_id: "van-furacao".into(),
            starvation_type: lacraia_shared::CacheStarvationType::LocalCacheMissing,
            message: "CACHE LOCAL EM FALTA :: CARREGANDO DA NUVEM".into(),
            track_id: Some("trk_cloud".into()),
            active: true,
        });
        state.handle_ws_message(alert_msg);
        assert!(state.cache_alert.is_some());
        assert_eq!(state.cache_alert.as_ref().unwrap().starvation_type, lacraia_shared::CacheStarvationType::LocalCacheMissing);
        assert_eq!(state.cache_alert.as_ref().unwrap().message, "CACHE LOCAL EM FALTA :: CARREGANDO DA NUVEM");

        // Test clear CacheAlert message
        let alert_clear = WsMessage::CacheAlert(lacraia_shared::CacheAlertNotification {
            cabin_id: "van-furacao".into(),
            starvation_type: lacraia_shared::CacheStarvationType::LocalCacheMissing,
            message: "".into(),
            track_id: None,
            active: false,
        });
        state.handle_ws_message(alert_clear);
        assert!(state.cache_alert.is_none());

        state.set_volume(1.2);
        assert_eq!(state.volume, 1.0);
        state.set_volume(-0.1);
        assert_eq!(state.volume, 0.0);

        state.select_station("Synthwave Drive".to_string());
        assert_eq!(state.active_station, "Synthwave Drive");
    }

    #[test]
    fn test_format_gain_11() {
        assert_eq!(format_gain_11(0.0), 0);
        assert_eq!(format_gain_11(1.0), 11);
        assert_eq!(format_gain_11(0.5), 6);
        assert_eq!(format_gain_11(1.5), 11);
        assert_eq!(format_gain_11(-0.5), 0);
    }

    #[test]
    fn test_visualizer_state_lifecycle() {
        let mut vis = VisualizerState::new();
        assert!(vis.is_active());
        assert_eq!(vis.bands_count, 32);
        assert_eq!(vis.amplitudes.len(), 32);
        assert!(vis.amplitudes.iter().all(|&a| a == 0.0));

        let toggled = vis.toggle();
        assert!(!toggled);
        assert!(!vis.is_active());

        // Feeding amplitudes when disabled does not populate
        vis.update_amplitudes(&[0.9; 32]);
        assert!(vis.amplitudes.iter().all(|&a| a == 0.0));

        vis.toggle();
        assert!(vis.is_active());
    }

    #[test]
    fn test_visualizer_inertia_falloff() {
        let mut vis = VisualizerState::new();
        let impulse = vec![1.0; 32];
        vis.update_amplitudes(&impulse);

        for amp in &vis.amplitudes {
            assert_eq!(*amp, 1.0);
        }

        // Drop signal to zero: verify smooth decay
        let silence = vec![0.0; 32];
        vis.update_amplitudes(&silence);

        for amp in &vis.amplitudes {
            assert!(*amp < 1.0 && *amp > 0.8, "Amplitude should decay smoothly: {}", amp);
        }

        // Reset clears all
        vis.reset();
        for amp in &vis.amplitudes {
            assert_eq!(*amp, 0.0);
        }
    }
}

