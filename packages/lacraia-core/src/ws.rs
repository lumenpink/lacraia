use crate::db::Database;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use lacraia_shared::{
    CabinLockedNotification, CommandAction, CrewUpdate, EjectedNotification, PlaybackDetails,
    PlaybackState, Role, StatePayload, StateUpdate, Track, WsMessage,
};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tokio::time::sleep;
use tracing::{info, warn};

#[derive(Debug, Deserialize, Default)]
pub struct WsParams {
    pub cabin: Option<String>,
    pub token: Option<String>,
}

#[derive(Clone)]
pub struct PlaybackHub {
    pub db: Database,
    pub tx: broadcast::Sender<WsMessage>,
    pub current_track: Arc<Mutex<Option<Track>>>,
    pub playback_state: Arc<Mutex<PlaybackState>>,
    pub position_ms: Arc<Mutex<u64>>,
    pub cabin_id: Arc<Mutex<String>>,
}

impl PlaybackHub {
    pub fn new(db: Database, tx: broadcast::Sender<WsMessage>) -> Self {
        Self {
            db,
            tx,
            current_track: Arc::new(Mutex::new(None)),
            playback_state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            position_ms: Arc::new(Mutex::new(0)),
            cabin_id: Arc::new(Mutex::new("default".into())),
        }
    }

    /// Builds a snapshot `STATE_UPDATE` message with the current playback status for a specific cabin.
    pub async fn build_state_update_for_cabin(&self, cabin_id: &str) -> WsMessage {
        let track = self.current_track.lock().await.clone();
        let state = *self.playback_state.lock().await;
        let pos = *self.position_ms.lock().await;
        let is_locked = self.db.is_cabin_locked(cabin_id).unwrap_or(false);

        let queue_entries = self.db.get_queue_for_cabin(cabin_id).unwrap_or_default();
        let queue: Vec<lacraia_shared::QueueItem> = queue_entries
            .into_iter()
            .map(|(q, t)| lacraia_shared::QueueItem {
                id: q.id,
                track_id: q.track_id,
                title: t.title,
                artist: t.artist,
                duration_ms: t.duration_sec * 1000,
                position: q.position,
                state: q.state.as_str().to_string(),
                requested_by: q.requested_by,
            })
            .collect();

        WsMessage::StateUpdate(StateUpdate {
            payload: StatePayload {
                track,
                playback: PlaybackDetails {
                    state,
                    position_ms: pos,
                    updated_at: Utc::now().timestamp_millis(),
                },
                has_lrc: true,
                cabin_id: Some(cabin_id.to_string()),
                is_locked: Some(is_locked),
                queue,
            },
        })
    }

    /// Builds a snapshot `STATE_UPDATE` message with default cabin status.
    pub async fn build_state_update(&self) -> WsMessage {
        let cabin_id = self.cabin_id.lock().await.clone();
        self.build_state_update_for_cabin(&cabin_id).await
    }

    pub async fn dispatch_command(&self, action: CommandAction) -> Result<WsMessage, String> {
        let cabin_id = self.cabin_id.lock().await.clone();
        self.dispatch_command_with_role(action, &cabin_id, Some(Role::Pilot)).await
    }

    /// Dispatches a command action, mutates internal state, queries the database if needed,
    /// checks permissions according to caller role, and broadcasts the resulting state update.
    pub async fn dispatch_command_with_role(
        &self,
        action: CommandAction,
        cabin_id: &str,
        caller_role: Option<Role>,
    ) -> Result<WsMessage, String> {
        // Enforce strict role-based access control
        match caller_role {
            Some(Role::Pilot) => {
                // Pilot has full command authority
            }
            Some(Role::Passenger) => {
                match &action {
                    CommandAction::QueueAppend { .. } => { /* Allowed */ }
                    _ => return Err("FORBIDDEN: Apenas o piloto pode comandar a cabine".to_string()),
                }
            }
            None => {
                // Unauthenticated / Anonymous
                if self.db.is_cabin_locked(cabin_id).unwrap_or(false) {
                    return Err("FORBIDDEN: Cabine trancada pelo piloto".to_string());
                }
                match &action {
                    CommandAction::QueueAppend { .. } => { /* Allowed */ }
                    _ => return Err("UNAUTHORIZED: Autenticação de piloto necessária para comandar a cabine".to_string()),
                }
            }
        }

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
                match self.db.advance_queue_for_cabin(cabin_id) {
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
                                requested_by: entry.requested_by,
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
            CommandAction::QueueAppend { track_id, requested_by } => {
                if let Err(e) = self.db.enqueue_track_with_meta(&track_id, requested_by.as_deref(), cabin_id) {
                    return Err(format!("Failed to enqueue track: {}", e));
                }
            }
            CommandAction::QueueRemove { queue_id } => {
                if let Err(e) = self.db.remove_queue_entry(queue_id, cabin_id) {
                    return Err(format!("Failed to remove track from queue: {}", e));
                }
            }
            CommandAction::LockCabin { locked } => {
                let _ = self.db.set_cabin_locked(cabin_id, locked);
                let passengers = self.db.get_active_passengers(cabin_id).unwrap_or_default();
                let _ = self.tx.send(WsMessage::CrewUpdate(CrewUpdate {
                    cabin_id: cabin_id.to_string(),
                    is_locked: locked,
                    passengers,
                }));
            }
            CommandAction::EjectPassenger { passenger_id } => {
                let _ = self.db.eject_passenger(&passenger_id);
                let _ = self.tx.send(WsMessage::Ejected(EjectedNotification {
                    cabin_id: cabin_id.to_string(),
                    reason: "Chute da cabine pelo piloto".into(),
                }));
                let passengers = self.db.get_active_passengers(cabin_id).unwrap_or_default();
                let is_locked = self.db.is_cabin_locked(cabin_id).unwrap_or(false);
                let _ = self.tx.send(WsMessage::CrewUpdate(CrewUpdate {
                    cabin_id: cabin_id.to_string(),
                    is_locked,
                    passengers,
                }));
            }
            CommandAction::EjectAll => {
                let _ = self.db.eject_all_passengers(cabin_id);
                let _ = self.tx.send(WsMessage::Ejected(EjectedNotification {
                    cabin_id: cabin_id.to_string(),
                    reason: "Todos os passageiros foram ejetados pelo piloto".into(),
                }));
                let is_locked = self.db.is_cabin_locked(cabin_id).unwrap_or(false);
                let _ = self.tx.send(WsMessage::CrewUpdate(CrewUpdate {
                    cabin_id: cabin_id.to_string(),
                    is_locked,
                    passengers: vec![],
                }));
            }
        }

        let update = self.build_state_update_for_cabin(cabin_id).await;
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
    Query(params): Query<WsParams>,
    State(hub): State<Arc<PlaybackHub>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, hub, params))
}

pub async fn handle_socket(socket: WebSocket, hub: Arc<PlaybackHub>, params: WsParams) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = hub.tx.subscribe();

    let cabin_id = params.cabin.unwrap_or_else(|| "default".to_string());
    let master_key = std::env::var("LACRAIA_MASTER_KEY").ok();

    // Check role from token if provided (strictly no automatic pilot fallback)
    let role = if let Some(ref tok) = params.token {
        hub.db.validate_token(&cabin_id, tok, master_key.as_deref()).unwrap_or(None)
    } else {
        None
    };

    // If cabin is locked and user has no role, notify and abort connection
    if hub.db.is_cabin_locked(&cabin_id).unwrap_or(false) && role.is_none() {
        let locked_msg = WsMessage::CabinLocked(CabinLockedNotification {
            cabin_id: cabin_id.clone(),
            message: "Portas trancadas pelo piloto".into(),
        });
        if let Ok(json) = serde_json::to_string(&locked_msg) {
            let _ = sender.send(Message::Text(json)).await;
        }
        return;
    }

    // Send initial snapshot state immediately upon client connection
    let initial_state = hub.build_state_update_for_cabin(&cabin_id).await;
    if let Ok(json) = serde_json::to_string(&initial_state) {
        let _ = sender.send(Message::Text(json)).await;
    }

    // Send initial crew update
    let passengers = hub.db.get_active_passengers(&cabin_id).unwrap_or_default();
    let is_locked = hub.db.is_cabin_locked(&cabin_id).unwrap_or(false);
    let crew_msg = WsMessage::CrewUpdate(CrewUpdate {
        cabin_id: cabin_id.clone(),
        is_locked,
        passengers,
    });
    if let Ok(json) = serde_json::to_string(&crew_msg) {
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
    let cabin_id_clone = cabin_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            match serde_json::from_str::<WsMessage>(&text) {
                Ok(WsMessage::Command(cmd)) => {
                    info!("WebSocket Hub: dispatching command {:?} on cabin {} with role {:?}", cmd.payload, cabin_id_clone, role);
                    let _ = hub_clone.dispatch_command_with_role(cmd.payload, &cabin_id_clone, role).await;
                }
                Ok(WsMessage::StateUpdate(_)) => {
                    warn!("Client sent unexpected StateUpdate, ignoring");
                }
                Ok(WsMessage::CrewUpdate(_)) | Ok(WsMessage::Ejected(_)) | Ok(WsMessage::CabinLocked(_)) => {
                    // Outgoing notifications ignored from client input
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

    #[tokio::test]
    async fn test_playback_hub_role_authorization_and_cabin_controls() {
        let db = Database::open_in_memory().unwrap();
        let (tx, mut rx) = broadcast::channel(10);
        let hub = Arc::new(PlaybackHub::new(db.clone(), tx));

        // 0. Anonymous/Guest cannot Pause/Stop (unauthorized)
        let unauth_res = hub
            .dispatch_command_with_role(CommandAction::Pause, "default", None)
            .await;
        assert!(unauth_res.is_err());
        assert!(unauth_res.unwrap_err().contains("UNAUTHORIZED"));

        // 1. Passenger cannot Pause/Stop (forbidden)
        let res = hub
            .dispatch_command_with_role(CommandAction::Pause, "default", Some(Role::Passenger))
            .await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("FORBIDDEN"));

        // 2. Passenger CAN append to queue
        let track = TrackRecord {
            id: "trk_party".into(),
            artist: "Daft Punk".into(),
            title: "One More Time".into(),
            duration_sec: 320,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&track).unwrap();

        let queue_res = hub
            .dispatch_command_with_role(
                CommandAction::QueueAppend {
                    track_id: "trk_party".into(),
                    requested_by: Some("@Mari".into()),
                },
                "default",
                Some(Role::Passenger),
            )
            .await;
        assert!(queue_res.is_ok());

        // Drain state update
        let _ = rx.recv().await.unwrap();

        // 3. Pilot locks cabin
        hub.dispatch_command(CommandAction::LockCabin { locked: true })
            .await
            .unwrap();
        let crew_msg = rx.recv().await.unwrap();
        match crew_msg {
            WsMessage::CrewUpdate(c) => {
                assert!(c.is_locked);
            }
            _ => panic!("Expected CrewUpdate with is_locked=true"),
        }

        // Drain subsequent state update from dispatch_command
        let _ = rx.recv().await.unwrap();

        // 3.1. When cabin is locked, anonymous guest cannot append to queue
        let locked_append_res = hub
            .dispatch_command_with_role(
                CommandAction::QueueAppend {
                    track_id: "trk_party".into(),
                    requested_by: Some("@Hacker".into()),
                },
                "default",
                None,
            )
            .await;
        assert!(locked_append_res.is_err());
        assert!(locked_append_res.unwrap_err().contains("FORBIDDEN"));

        // 4. Pilot ejects passenger
        hub.dispatch_command(CommandAction::EjectPassenger {
            passenger_id: "psg_test".into(),
        })
        .await
        .unwrap();

        let eject_msg = rx.recv().await.unwrap();
        match eject_msg {
            WsMessage::Ejected(e) => {
                assert!(e.reason.contains("piloto"));
            }
            _ => panic!("Expected Ejected message"),
        }
    }
}
