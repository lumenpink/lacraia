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

