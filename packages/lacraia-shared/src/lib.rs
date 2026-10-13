pub mod fuzzy;
pub mod lrc;
pub mod models;
pub mod protocol;

pub use fuzzy::{is_title_clean, is_title_matching, is_valid_duration_delta, strip_accents};
pub use lrc::{parse_lrc, LyricLine};
pub use models::{
    AttemptsSuccessDistribution, CabinInfo, CacheAlertInfo, CacheEntry, CacheResolutionAttempt,
    CacheStarvationType, PassengerInfo, PlaybackHistoryEntry, PlaybackState, ResolutionErrorDiagnostic,
    ResolutionTier, Role, TierSuccessRate, Track, ValidationStatus,
};
pub use protocol::{
    CabinLockedNotification, CacheAlertNotification, Command, CommandAction, CrewUpdate,
    EjectedNotification, PlaybackDetails, QueueItem, StatePayload, StateUpdate, WsMessage,
};
