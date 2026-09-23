use crate::db::Database;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use lacraia_shared::{
    CommandAction, PlaybackDetails, PlaybackState, StatePayload, StateUpdate, Track, WsMessage,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tokio::time::sleep;
use tracing::{info, warn};

#[derive(Clone)]
pub struct PlaybackHub {
    pub db: Database,
    pub tx: broadcast::Sender<WsMessage>,
    pub current_track: Arc<Mutex<Option<Track>>>,
    pub playback_state: Arc<Mutex<PlaybackState>>,
    pub position_ms: Arc<Mutex<u64>>,
}

impl PlaybackHub {
    pub fn new(db: Database, tx: broadcast::Sender<WsMessage>) -> Self {
        Self {
            db,
            tx,
            current_track: Arc::new(Mutex::new(None)),
            playback_state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            position_ms: Arc::new(Mutex::new(0)),
        }
    }

    /// Builds a snapshot `STATE_UPDATE` message with the current playback status.
    pub async fn build_state_update(&self) -> WsMessage {
        let track = self.current_track.lock().await.clone();
        let state = *self.playback_state.lock().await;
        let pos = *self.position_ms.lock().await;

        WsMessage::StateUpdate(StateUpdate {
            payload: StatePayload {
                track,
                playback: PlaybackDetails {
                    state,
                    position_ms: pos,
                    updated_at: Utc::now().timestamp_millis(),
                },
                has_lrc: true,
            },
        })
    }

    /// Dispatches a command action, mutates internal state, queries the database if needed,
    /// and broadcasts the resulting state update.
    pub async fn dispatch_command(&self, action: CommandAction) -> Result<WsMessage, String> {
        match action {
            CommandAction::Play => {
                *self.playback_state.lock().await = PlaybackState::Playing;
            }
            CommandAction::Pause => {
                *self.playback_state.lock().await = PlaybackState::Paused;
            }
            CommandAction::Seek { position_ms } => {
                *self.position_ms.lock().await = position_ms;
            }
            CommandAction::Next => {
                *self.position_ms.lock().await = 0;
                match self.db.advance_queue() {
                    Ok(Some(entry)) => {
                        if let Ok(Some(record)) = self.db.get_track(&entry.track_id) {
                            let track = Track {
                                id: record.id.clone(),
                                title: record.title,
                                artist: record.artist,
                                album: None,
                                duration_ms: record.duration_sec * 1000,
                                cover_url: None,
                                stream_url: record.cached_path.map(|p| format!("/cache/{}", p)),
                            };
                            *self.current_track.lock().await = Some(track);
                            *self.playback_state.lock().await = PlaybackState::Playing;
                        }
                    }
                    Ok(None) => {
                        *self.current_track.lock().await = None;
                        *self.playback_state.lock().await = PlaybackState::Stopped;
                    }
                    Err(e) => return Err(format!("Database queue advance failed: {}", e)),
                }
            }
            CommandAction::Previous => {
                *self.position_ms.lock().await = 0;
            }
            CommandAction::QueueAppend { track_id } => {
                if let Err(e) = self.db.enqueue_track(&track_id) {
                    return Err(format!("Failed to enqueue track: {}", e));
                }
            }
        }

        let update = self.build_state_update().await;
        let _ = self.tx.send(update.clone());
        Ok(update)
    }

    /// Starts a background ticker emitting telemetries and advancing playhead when playing.
    pub fn start_telemetry_ticker(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_millis(1000)).await;

                let is_playing = {
                    let state = self.playback_state.lock().await;
                    *state == PlaybackState::Playing
                };

                if is_playing {
                    let mut pos = self.position_ms.lock().await;
                    *pos += 1000;

                    let duration_ms = {
                        let t = self.current_track.lock().await;
                        t.as_ref().map(|trk| trk.duration_ms).unwrap_or(0)
                    };

                    // Auto-advance track when current track finishes
                    if duration_ms > 0 && *pos >= duration_ms {
                        drop(pos);
                        info!("Track reached duration, advancing queue automatically");
                        let _ = self.dispatch_command(CommandAction::Next).await;
                        continue;
                    }
                }

                // Periodic telemetry heartbeat broadcast
                let update = self.build_state_update().await;
                let _ = self.tx.send(update);
            }
        })
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(hub): State<Arc<PlaybackHub>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, hub))
}

pub async fn handle_socket(socket: WebSocket, hub: Arc<PlaybackHub>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = hub.tx.subscribe();

    // Send initial snapshot state immediately upon client connection
    let initial_state = hub.build_state_update().await;
    if let Ok(json) = serde_json::to_string(&initial_state) {
        let _ = sender.send(Message::Text(json)).await;
    }

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    let hub_clone = hub.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            match serde_json::from_str::<WsMessage>(&text) {
                Ok(WsMessage::Command(cmd)) => {
                    info!("WebSocket Hub: dispatching command {:?}", cmd.payload);
                    let _ = hub_clone.dispatch_command(cmd.payload).await;
                }
                Ok(WsMessage::StateUpdate(_)) => {
                    warn!("Client sent unexpected StateUpdate, ignoring");
                }
                Err(e) => {
                    warn!("Failed to parse WebSocket message from client: {}", e);
                }
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::TrackRecord;

    #[tokio::test]
    async fn test_playback_hub_command_dispatch() {
        let db = Database::open_in_memory().unwrap();
        let (tx, mut rx) = broadcast::channel(10);
        let hub = Arc::new(PlaybackHub::new(db.clone(), tx));

        // Insert tracks into database
        let track = TrackRecord {
            id: "trk_alpha".into(),
            artist: "David Bowie".into(),
            title: "Heroes".into(),
            duration_sec: 360,
            cached_path: Some("/cache/heroes.opus".into()),
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&track).unwrap();
        db.enqueue_track("trk_alpha").unwrap();

        // 1. Play Command
        hub.dispatch_command(CommandAction::Play).await.unwrap();
        let received = rx.recv().await.unwrap();
        match received {
            WsMessage::StateUpdate(u) => {
                assert_eq!(u.payload.playback.state, PlaybackState::Playing);
            }
            _ => panic!("Expected StateUpdate"),
        }

        // 2. Seek Command
        hub.dispatch_command(CommandAction::Seek { position_ms: 45000 })
            .await
            .unwrap();
        let received2 = rx.recv().await.unwrap();
        match received2 {
            WsMessage::StateUpdate(u) => {
                assert_eq!(u.payload.playback.position_ms, 45000);
            }
            _ => panic!("Expected StateUpdate"),
        }

        // 3. Next Command (should advance queue and set track)
        hub.dispatch_command(CommandAction::Next).await.unwrap();
        let received3 = rx.recv().await.unwrap();
        match received3 {
            WsMessage::StateUpdate(u) => {
                assert_eq!(u.payload.playback.position_ms, 0);
                assert_eq!(u.payload.playback.state, PlaybackState::Playing);
                assert!(u.payload.track.is_some());
                assert_eq!(u.payload.track.unwrap().title, "Heroes");
            }
            _ => panic!("Expected StateUpdate"),
        }
    }
}
