use lacraia_shared::{PlaybackState, Track};

#[derive(Debug, Clone, PartialEq)]
pub struct AudioMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover_url: Option<String>,
}

impl From<&Track> for AudioMetadata {
    fn from(t: &Track) -> Self {
        Self {
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone().unwrap_or_else(|| "La Craia Radio".to_string()),
            cover_url: t.cover_url.clone(),
        }
    }
}

pub struct AudioEngine {
    current_track: Option<Track>,
    playback_state: PlaybackState,
    volume: f64,
    position_ms: u64,
    duration_ms: u64,
    #[cfg(target_arch = "wasm32")]
    audio_elem: Option<web_sys::HtmlAudioElement>,
}

impl AudioEngine {
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        let audio_elem = web_sys::HtmlAudioElement::new().ok();

        Self {
            current_track: None,
            playback_state: PlaybackState::Stopped,
            volume: 1.0,
            position_ms: 0,
            duration_ms: 0,
            #[cfg(target_arch = "wasm32")]
            audio_elem,
        }
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.current_track.as_ref()
    }

    pub fn playback_state(&self) -> PlaybackState {
        self.playback_state
    }

    pub fn position_ms(&self) -> u64 {
        self.position_ms
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    pub fn volume(&self) -> f64 {
        self.volume
    }

    pub fn set_volume(&mut self, vol: f64) {
        self.volume = vol.clamp(0.0, 1.0);
        #[cfg(target_arch = "wasm32")]
        if let Some(ref elem) = self.audio_elem {
            elem.set_volume(self.volume);
        }
    }

    pub fn load_track(&mut self, track: Track) {
        self.duration_ms = track.duration_ms;
        self.position_ms = 0;

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(ref elem) = self.audio_elem {
                if let Some(ref url) = track.stream_url {
                    elem.set_src(url);
                }
            }
            self.sync_media_session(&track);
        }

        self.current_track = Some(track);
    }

    pub fn play(&mut self) {
        if self.current_track.is_some() {
            self.playback_state = PlaybackState::Playing;
            #[cfg(target_arch = "wasm32")]
            if let Some(ref elem) = self.audio_elem {
                let _ = elem.play();
            }
        }
    }

    pub fn pause(&mut self) {
        self.playback_state = PlaybackState::Paused;
        #[cfg(target_arch = "wasm32")]
        if let Some(ref elem) = self.audio_elem {
            let _ = elem.pause();
        }
    }

    pub fn seek_to(&mut self, pos_ms: u64) {
        self.position_ms = pos_ms.min(self.duration_ms);
        #[cfg(target_arch = "wasm32")]
        if let Some(ref elem) = self.audio_elem {
            elem.set_current_time((self.position_ms as f64) / 1000.0);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn sync_media_session(&self, track: &Track) {
        if let Some(window) = web_sys::window() {
            let nav = window.navigator();
            let media_session = nav.media_session();

            if let Ok(meta) = web_sys::MediaMetadata::new() {
                meta.set_title(&track.title);
                meta.set_artist(&track.artist);
                meta.set_album(&track.album.clone().unwrap_or_else(|| "La Craia".to_string()));
                media_session.set_metadata(Some(&meta));
            }
        }
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_engine_lifecycle() {
        let mut engine = AudioEngine::new();
        assert_eq!(engine.playback_state(), PlaybackState::Stopped);

        let track = Track {
            id: "trk_1".into(),
            title: "Rebel Rebel".into(),
            artist: "David Bowie".into(),
            album: Some("Diamond Dogs".into()),
            duration_ms: 274000,
            cover_url: None,
            stream_url: Some("/cache/rebel.opus".into()),
        };

        engine.load_track(track.clone());
        assert_eq!(engine.duration_ms(), 274000);
        assert_eq!(engine.position_ms(), 0);

        engine.play();
        assert_eq!(engine.playback_state(), PlaybackState::Playing);

        engine.seek_to(45000);
        assert_eq!(engine.position_ms(), 45000);

        engine.pause();
        assert_eq!(engine.playback_state(), PlaybackState::Paused);

        // Clamping volume
        engine.set_volume(1.5);
        assert_eq!(engine.volume(), 1.0);
        engine.set_volume(-0.5);
        assert_eq!(engine.volume(), 0.0);
        engine.set_volume(0.8);
        assert_eq!(engine.volume(), 0.8);
    }

    #[test]
    fn test_metadata_conversion() {
        let track = Track {
            id: "trk_2".into(),
            title: "Heroes".into(),
            artist: "David Bowie".into(),
            album: None,
            duration_ms: 360000,
            cover_url: Some("https://example.com/cover.jpg".into()),
            stream_url: None,
        };

        let meta = AudioMetadata::from(&track);
        assert_eq!(meta.title, "Heroes");
        assert_eq!(meta.artist, "David Bowie");
        assert_eq!(meta.album, "La Craia Radio");
        assert_eq!(meta.cover_url.as_deref(), Some("https://example.com/cover.jpg"));
    }
}
