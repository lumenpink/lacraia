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
    #[serde(rename = "nonStop", default)]
    pub non_stop: bool,
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

/// Starvation or cache absence alert classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CacheStarvationType {
    /// Track has no local file in /cache/ and requires remote cloud fetching.
    LocalCacheMissing,
    /// Remote network stream exhausted / buffer underrun during active playback.
    RemoteBufferStarved,
}

/// Alert notification payload describing buffer starvation or cache absence over the player deck.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheAlertInfo {
    #[serde(rename = "starvationType")]
    pub starvation_type: CacheStarvationType,
    pub message: String,
    #[serde(rename = "trackId", default)]
    pub track_id: Option<String>,
}

/// Physical inventory record for an audio file residing in the server cache.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheEntry {
    pub id: i64,
    #[serde(rename = "trackId")]
    pub track_id: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "fileSizeBytes")]
    pub file_size_bytes: u64,
    #[serde(rename = "durationSec")]
    pub duration_sec: u64,
    #[serde(rename = "audioCodec")]
    pub audio_codec: String,
    #[serde(rename = "hitCount")]
    pub hit_count: u64,
    #[serde(rename = "lastServedAt")]
    pub last_served_at: i64,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
}

/// Granular tier classification of audio resolution pipeline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionTier {
    Tier0Local,
    Tier1Direct,
    Tier2YtdlpWeb,
    Tier3YtdlpAndroid,
}

impl ResolutionTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tier0Local => "tier_0_local",
            Self::Tier1Direct => "tier_1_direct",
            Self::Tier2YtdlpWeb => "tier_2_ytdlp_web",
            Self::Tier3YtdlpAndroid => "tier_3_ytdlp_android",
        }
    }
}

/// Verification status of a resolved audio file artifact.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Valid,
    EmptyFile,
    Corrupted,
    NetworkError,
}

impl ValidationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::EmptyFile => "empty_file",
            Self::Corrupted => "corrupted",
            Self::NetworkError => "network_error",
        }
    }
}

/// Granular log of an individual audio resolution attempt across tiers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheResolutionAttempt {
    pub id: i64,
    #[serde(rename = "trackId")]
    pub track_id: String,
    #[serde(rename = "attemptNumber")]
    pub attempt_number: u32,
    pub tier: String,
    pub method: String,
    #[serde(rename = "commandLine")]
    pub command_line: Option<String>,
    pub success: bool,
    #[serde(rename = "exitCode")]
    pub exit_code: Option<i32>,
    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,
    #[serde(rename = "errorOutput")]
    pub error_output: Option<String>,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
    #[serde(rename = "bytesReceived")]
    pub bytes_received: u64,
    #[serde(rename = "validationStatus")]
    pub validation_status: String,
    #[serde(rename = "cabinId")]
    pub cabin_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
}

/// Summary statistics for resolution attempts reaching success.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttemptsSuccessDistribution {
    #[serde(rename = "attemptNumber")]
    pub attempt_number: u32,
    pub count: u64,
}

/// Summary statistics per resolution tier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TierSuccessRate {
    pub tier: String,
    #[serde(rename = "totalAttempts")]
    pub total_attempts: u64,
    pub successes: u64,
    pub failures: u64,
    #[serde(rename = "successRatePct")]
    pub success_rate_pct: f64,
    #[serde(rename = "avgDurationMs")]
    pub avg_duration_ms: f64,
}

/// Summary diagnostics for resolution errors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolutionErrorDiagnostic {
    pub tier: String,
    #[serde(rename = "errorCode")]
    pub error_code: String,
    pub count: u64,
    #[serde(rename = "sampleOutput")]
    pub sample_output: Option<String>,
}


