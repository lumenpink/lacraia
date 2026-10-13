use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
    #[serde(rename = "coverUrl", default)]
    pub cover_url: Option<String>,
    #[serde(rename = "streamUrl", default)]
    pub stream_url: Option<String>,
    #[serde(rename = "requestedBy", default)]
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Pilot,
    Passenger,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PassengerInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(rename = "joinedAt")]
    pub joined_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CabinInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "isLocked")]
    pub is_locked: bool,
    #[serde(default)]
    pub passengers: Vec<PassengerInfo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
    Buffering,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self::Stopped
    }
}

/// Entry representing a song playback record for telemetry, history, and quick replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaybackHistoryEntry {
    /// Unique auto-incrementing ID in SQLite.
    pub id: i64,
    /// Cabin identifier where playback occurred.
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    /// Canonical track identifier.
    #[serde(rename = "trackId")]
    pub track_id: String,
    /// Song title.
    pub title: String,
    /// Artist or performer name.
    pub artist: String,
    /// Passenger name who requested the playback, if any.
    #[serde(rename = "passengerName")]
    pub passenger_name: Option<String>,
    /// UTC unix timestamp of when playback started.
    #[serde(rename = "playedAt")]
    pub played_at: i64,
    /// Track duration in seconds.
    #[serde(rename = "durationSec")]
    pub duration_sec: u64,
}

