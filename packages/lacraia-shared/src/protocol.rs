use crate::models::{PlaybackState, Track};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "STATE_UPDATE")]
    StateUpdate(StateUpdate),
    #[serde(rename = "COMMAND")]
    Command(Command),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateUpdate {
    pub payload: StatePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatePayload {
    pub track: Option<Track>,
    pub playback: PlaybackDetails,
    #[serde(rename = "hasLrc")]
    pub has_lrc: bool,
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
                }),
                playback: PlaybackDetails {
                    state: PlaybackState::Playing,
                    position_ms: 15000,
                    updated_at: 1727047200000,
                },
                has_lrc: true,
            },
        });

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("STATE_UPDATE"));
        assert!(json.contains("trk_123"));
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
}
