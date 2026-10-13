use crate::models::{PassengerInfo, PlaybackState, Track};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "STATE_UPDATE")]
    StateUpdate(StateUpdate),
    #[serde(rename = "COMMAND")]
    Command(Command),
    #[serde(rename = "CREW_UPDATE")]
    CrewUpdate(CrewUpdate),
    #[serde(rename = "EJECTED")]
    Ejected(EjectedNotification),
    #[serde(rename = "CABIN_LOCKED")]
    CabinLocked(CabinLockedNotification),
    #[serde(rename = "CACHE_ALERT")]
    CacheAlert(CacheAlertNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheAlertNotification {
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    #[serde(rename = "starvationType")]
    pub starvation_type: crate::models::CacheStarvationType,
    pub message: String,
    #[serde(rename = "trackId", default)]
    pub track_id: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrewUpdate {
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    #[serde(rename = "isLocked")]
    pub is_locked: bool,
    pub passengers: Vec<PassengerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EjectedNotification {
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CabinLockedNotification {
    #[serde(rename = "cabinId")]
    pub cabin_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateUpdate {
    pub payload: StatePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueItem {
    pub id: i64,
    #[serde(rename = "trackId")]
    pub track_id: String,
    pub title: String,
    pub artist: String,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
    pub position: i64,
    pub state: String,
    #[serde(rename = "requestedBy", default)]
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatePayload {
    pub track: Option<Track>,
    pub playback: PlaybackDetails,
    #[serde(rename = "hasLrc")]
    pub has_lrc: bool,
    #[serde(rename = "cabinId", default)]
    pub cabin_id: Option<String>,
    #[serde(rename = "isLocked", default)]
    pub is_locked: Option<bool>,
    #[serde(rename = "nonStop", default)]
    pub non_stop: Option<bool>,
    #[serde(default)]
    pub queue: Vec<QueueItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaybackDetails {
    pub state: PlaybackState,
    #[serde(rename = "positionMs")]
    pub position_ms: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Command {
    pub payload: CommandAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandAction {
    Play,
    Pause,
    Next,
    Previous,
    Seek {
        #[serde(rename = "positionMs")]
        position_ms: u64,
    },
    QueueAppend {
        #[serde(rename = "trackId")]
        track_id: String,
        #[serde(rename = "requestedBy", default)]
        requested_by: Option<String>,
    },
    QueueNext {
        #[serde(rename = "trackId")]
        track_id: String,
        #[serde(rename = "requestedBy", default)]
        requested_by: Option<String>,
    },
    PlayNow {
        #[serde(rename = "trackId")]
        track_id: String,
        #[serde(rename = "requestedBy", default)]
        requested_by: Option<String>,
    },
    StartRadio {
        #[serde(rename = "trackId")]
        track_id: String,
        #[serde(rename = "requestedBy", default)]
        requested_by: Option<String>,
    },
    QueueRemove {
        #[serde(rename = "queueId")]
        queue_id: i64,
    },
    Jump {
        #[serde(rename = "queueId")]
        queue_id: i64,
    },
    LockCabin {
        locked: bool,
    },
    EjectPassenger {
        #[serde(rename = "passengerId")]
        passenger_id: String,
    },
    EjectAll,
    ToggleNonStop,
    SetNonStop {
        enabled: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_state_update() {
        let update = WsMessage::StateUpdate(StateUpdate {
            payload: StatePayload {
                track: Some(Track {
                    id: "trk_123".into(),
                    title: "Song".into(),
                    artist: "Artist".into(),
                    album: None,
                    duration_ms: 180000,
                    cover_url: None,
                    stream_url: None,
                    requested_by: Some("@Mari".into()),
                }),
                playback: PlaybackDetails {
                    state: PlaybackState::Playing,
                    position_ms: 15000,
                    updated_at: 1727047200000,
                },
                has_lrc: true,
                cabin_id: Some("van-furacao".into()),
                is_locked: Some(false),
                non_stop: None,
                queue: vec![],
            },
        });

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("STATE_UPDATE"));
        assert!(json.contains("trk_123"));
        assert!(json.contains("@Mari"));
        assert!(json.contains("van-furacao"));
    }

    #[test]
    fn test_deserialize_command() {
        let json = r#"{"type":"COMMAND","payload":{"action":"NEXT"}}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        match msg {
            WsMessage::Command(cmd) => assert_eq!(cmd.payload, CommandAction::Next),
            _ => panic!("Expected Command"),
        }
    }

    #[test]
    fn test_deserialize_cabin_commands() {
        let json_lock = r#"{"type":"COMMAND","payload":{"action":"LOCK_CABIN","locked":true}}"#;
        let msg_lock: WsMessage = serde_json::from_str(json_lock).unwrap();
        match msg_lock {
            WsMessage::Command(cmd) => assert_eq!(cmd.payload, CommandAction::LockCabin { locked: true }),
            _ => panic!("Expected LockCabin"),
        }

        let json_eject = r#"{"type":"COMMAND","payload":{"action":"EJECT_PASSENGER","passengerId":"psg_42"}}"#;
        let msg_eject: WsMessage = serde_json::from_str(json_eject).unwrap();
        match msg_eject {
            WsMessage::Command(cmd) => assert_eq!(
                cmd.payload,
                CommandAction::EjectPassenger {
                    passenger_id: "psg_42".into()
                }
            ),
            _ => panic!("Expected EjectPassenger"),
        }

        let json_queue = r#"{"type":"COMMAND","payload":{"action":"QUEUE_APPEND","trackId":"trk_99","requestedBy":"@Mari"}}"#;
        let msg_queue: WsMessage = serde_json::from_str(json_queue).unwrap();
        match msg_queue {
            WsMessage::Command(cmd) => assert_eq!(
                cmd.payload,
                CommandAction::QueueAppend {
                    track_id: "trk_99".into(),
                    requested_by: Some("@Mari".into()),
                }
            ),
            _ => panic!("Expected QueueAppend"),
        }

        let json_remove = r#"{"type":"COMMAND","payload":{"action":"QUEUE_REMOVE","queueId":42}}"#;
        let msg_remove: WsMessage = serde_json::from_str(json_remove).unwrap();
        match msg_remove {
            WsMessage::Command(cmd) => assert_eq!(
                cmd.payload,
                CommandAction::QueueRemove { queue_id: 42 }
            ),
            _ => panic!("Expected QueueRemove"),
        }

        let json_next = r#"{"type":"COMMAND","payload":{"action":"QUEUE_NEXT","trackId":"trk_next","requestedBy":"PILOTO"}}"#;
        let msg_next: WsMessage = serde_json::from_str(json_next).unwrap();
        match msg_next {
            WsMessage::Command(cmd) => assert_eq!(
                cmd.payload,
                CommandAction::QueueNext {
                    track_id: "trk_next".into(),
                    requested_by: Some("PILOTO".into()),
                }
            ),
            _ => panic!("Expected QueueNext"),
        }

        let json_now = r#"{"type":"COMMAND","payload":{"action":"PLAY_NOW","trackId":"trk_now","requestedBy":"PILOTO"}}"#;
        let msg_now: WsMessage = serde_json::from_str(json_now).unwrap();
        match msg_now {
            WsMessage::Command(cmd) => assert_eq!(
                cmd.payload,
                CommandAction::PlayNow {
                    track_id: "trk_now".into(),
                    requested_by: Some("PILOTO".into()),
                }
            ),
            _ => panic!("Expected PlayNow"),
        }

        let json_radio = r#"{"type":"COMMAND","payload":{"action":"START_RADIO","trackId":"trk_radio","requestedBy":"PILOTO"}}"#;
        let msg_radio: WsMessage = serde_json::from_str(json_radio).unwrap();
        match msg_radio {
            WsMessage::Command(cmd) => assert_eq!(
                cmd.payload,
                CommandAction::StartRadio {
                    track_id: "trk_radio".into(),
                    requested_by: Some("PILOTO".into()),
                }
            ),
            _ => panic!("Expected StartRadio"),
        }
    }

    #[test]
    fn test_serialize_crew_and_ejected_notifications() {
        let crew_msg = WsMessage::CrewUpdate(CrewUpdate {
            cabin_id: "van-furacao".into(),
            is_locked: true,
            passengers: vec![PassengerInfo {
                id: "psg_1".into(),
                name: "Mari".into(),
                device: Some("iPhone 14".into()),
                joined_at: 1727048000000,
            }],
        });
        let json_crew = serde_json::to_string(&crew_msg).unwrap();
        assert!(json_crew.contains("CREW_UPDATE"));
        assert!(json_crew.contains("van-furacao"));
        assert!(json_crew.contains("Mari"));
        assert!(json_crew.contains("iPhone 14"));

        let ejected_msg = WsMessage::Ejected(EjectedNotification {
            cabin_id: "van-furacao".into(),
            reason: "Chute da cabine pelo motorista".into(),
        });
        let json_ejected = serde_json::to_string(&ejected_msg).unwrap();
        assert!(json_ejected.contains("EJECTED"));
        assert!(json_ejected.contains("Chute da cabine"));
    }

    #[test]
    fn test_serialize_cache_alert_notification() {
        let alert = WsMessage::CacheAlert(CacheAlertNotification {
            cabin_id: "cabin_test".into(),
            starvation_type: crate::models::CacheStarvationType::LocalCacheMissing,
            message: "CACHE LOCAL EM FALTA :: CARREGANDO DA NUVEM".into(),
            track_id: Some("trk_starved_1".into()),
            active: true,
        });
        let json_alert = serde_json::to_string(&alert).unwrap();
        assert!(json_alert.contains("CACHE_ALERT"));
        assert!(json_alert.contains("LOCAL_CACHE_MISSING"));
        assert!(json_alert.contains("trk_starved_1"));

        let deserialized: WsMessage = serde_json::from_str(&json_alert).unwrap();
        assert_eq!(deserialized, alert);
    }

    #[test]
    fn test_non_stop_commands_and_state_serialization() {
        // 1. ToggleNonStop command
        let toggle_cmd = Command {
            payload: CommandAction::ToggleNonStop,
        };
        let json_toggle = serde_json::to_string(&toggle_cmd).unwrap();
        assert!(json_toggle.contains("TOGGLE_NON_STOP"));
        let de_toggle: Command = serde_json::from_str(&json_toggle).unwrap();
        assert_eq!(de_toggle.payload, CommandAction::ToggleNonStop);

        // 2. SetNonStop command
        let set_cmd = Command {
            payload: CommandAction::SetNonStop { enabled: true },
        };
        let json_set = serde_json::to_string(&set_cmd).unwrap();
        assert!(json_set.contains("SET_NON_STOP"));
        assert!(json_set.contains("enabled"));
        let de_set: Command = serde_json::from_str(&json_set).unwrap();
        assert_eq!(de_set.payload, CommandAction::SetNonStop { enabled: true });

        // 3. StatePayload with non_stop: true
        let state = StateUpdate {
            payload: StatePayload {
                track: None,
                playback: PlaybackDetails {
                    state: PlaybackState::Playing,
                    position_ms: 0,
                    updated_at: 0,
                },
                has_lrc: false,
                cabin_id: Some("cab_nonstop".into()),
                is_locked: Some(false),
                non_stop: Some(true),
                queue: vec![],
            },
        };
        let json_state = serde_json::to_string(&state).unwrap();
        assert!(json_state.contains("\"nonStop\":true"));
        let de_state: StateUpdate = serde_json::from_str(&json_state).unwrap();
        assert_eq!(de_state.payload.non_stop, Some(true));
    }

    #[test]
    fn test_serialize_cache_inventory_and_attempts() {
        use crate::models::{CacheEntry, CacheResolutionAttempt, ResolutionTier, ValidationStatus};

        let entry = CacheEntry {
            id: 1,
            track_id: "mc:vai_lacraia".into(),
            file_path: "/cache/vai_lacraia.opus".into(),
            file_size_bytes: 4194304,
            duration_sec: 240,
            audio_codec: "opus".into(),
            hit_count: 5,
            last_served_at: 1700000000,
            created_at: 1699990000,
        };
        let json_entry = serde_json::to_string(&entry).unwrap();
        assert!(json_entry.contains("\"trackId\":\"mc:vai_lacraia\""));
        assert!(json_entry.contains("\"fileSizeBytes\":4194304"));
        assert!(json_entry.contains("\"hitCount\":5"));

        let attempt = CacheResolutionAttempt {
            id: 10,
            track_id: "mc:vai_lacraia".into(),
            attempt_number: 1,
            tier: ResolutionTier::Tier1Direct.as_str().into(),
            method: "rustypipe_stream".into(),
            command_line: None,
            success: false,
            exit_code: Some(403),
            error_code: Some("HTTP_403".into()),
            error_output: Some("Forbidden signature error".into()),
            duration_ms: 350,
            bytes_received: 0,
            validation_status: ValidationStatus::NetworkError.as_str().into(),
            cabin_id: Some("cab_default".into()),
            created_at: 1700000000,
        };
        let json_attempt = serde_json::to_string(&attempt).unwrap();
        assert!(json_attempt.contains("\"tier\":\"tier_1_direct\""));
        assert!(json_attempt.contains("\"exitCode\":403"));
        assert!(json_attempt.contains("\"errorCode\":\"HTTP_403\""));
    }
}

