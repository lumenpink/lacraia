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
    CabinLockedNotification, CacheAlertInfo, CacheAlertNotification, CacheStarvationType,
    CommandAction, CrewUpdate, EjectedNotification, PlaybackDetails, PlaybackState, Role,
    StatePayload, StateUpdate, Track, WsMessage,
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

pub fn clean_stream_url(path: &str) -> String {
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);
    format!("/cache/{}", filename)
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
        let queue_entries = self.db.get_queue_for_cabin(cabin_id).unwrap_or_default();
        let playing_track = queue_entries.iter().find(|(q, _)| q.state == crate::db::QueueState::Playing).map(|(q, t)| Track {
            id: t.id.clone(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: None,
            duration_ms: t.duration_sec * 1000,
            cover_url: None,
            stream_url: t.cached_path.as_deref().map(clean_stream_url),
            requested_by: q.requested_by.clone(),
        });

        let persisted_state = self.db.get_cabin_state(cabin_id).ok().flatten();

        let track = if playing_track.is_some() {
            playing_track
        } else if let Some(ref pers) = persisted_state {
            if let Some(ref tid) = pers.track_id {
                if let Ok(Some(record)) = self.db.get_track(tid) {
                    Some(Track {
                        id: record.id.clone(),
                        title: record.title,
                        artist: record.artist,
                        album: None,
                        duration_ms: record.duration_sec * 1000,
                        cover_url: None,
                        stream_url: record.cached_path.as_deref().map(clean_stream_url),
                        requested_by: Some("CABINE".into()),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        } else if cabin_id == "default" {
            self.current_track.lock().await.clone()
        } else {
            None
        };

        let state = if track.is_some() {
            if let Some(ref pers) = persisted_state {
                match pers.playback_state.as_str() {
                    "playing" => PlaybackState::Playing,
                    "paused" => PlaybackState::Paused,
                    _ => *self.playback_state.lock().await,
                }
            } else {
                *self.playback_state.lock().await
            }
        } else {
            PlaybackState::Stopped
        };

        let pos = if let Some(ref pers) = persisted_state {
            if *self.position_ms.lock().await == 0 && pers.position_ms > 0 {
                pers.position_ms
            } else {
                *self.position_ms.lock().await
            }
        } else {
            *self.position_ms.lock().await
        };
        let is_locked = self.db.is_cabin_locked(cabin_id).unwrap_or(false);

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

    pub async fn broadcast_state_for_cabin(&self, cabin_id: &str) {
        let update = self.build_state_update_for_cabin(cabin_id).await;
        let _ = self.tx.send(update);
    }

    pub async fn is_cabin_playing(&self, _cabin_id: &str) -> bool {
        let state = self.playback_state.lock().await;
        *state == PlaybackState::Playing
    }

    pub async fn get_current_track_for_cabin(&self, _cabin_id: &str) -> Option<Track> {
        self.current_track.lock().await.clone()
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
                    CommandAction::QueueAppend { .. } | CommandAction::QueueNext { .. } => { /* Allowed */ }
                    _ => return Err("FORBIDDEN: Apenas o piloto pode comandar a cabine".to_string()),
                }
            }
            None => {
                // Unauthenticated / Anonymous
                if self.db.is_cabin_locked(cabin_id).unwrap_or(false) {
                    return Err("FORBIDDEN: Cabine trancada pelo piloto".to_string());
                }
                match &action {
                    CommandAction::QueueAppend { .. } | CommandAction::QueueNext { .. } => { /* Allowed */ }
                    _ => return Err("UNAUTHORIZED: Autenticação de piloto necessária para comandar a cabine".to_string()),
                }
            }
        }

        let is_seek = matches!(action, CommandAction::Seek { .. });

        match action {
            CommandAction::Play => {
                *self.playback_state.lock().await = PlaybackState::Playing;
                if self.current_track.lock().await.is_none() {
                    if let Ok(queue) = self.db.get_queue_for_cabin(cabin_id) {
                        if let Some((entry, record)) = queue.into_iter().next() {
                            let _ = self.db.record_playback_history(cabin_id, &record.id, entry.requested_by.as_deref());
                            let track = Track {
                                id: record.id.clone(),
                                title: record.title,
                                artist: record.artist,
                                album: None,
                                duration_ms: record.duration_sec * 1000,
                                cover_url: None,
                                stream_url: record.cached_path.as_deref().map(clean_stream_url),
                                requested_by: entry.requested_by,
                            };
                            *self.current_track.lock().await = Some(track);
                        }
                    }
                }
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
                            let _ = self.db.record_playback_history(cabin_id, &record.id, entry.requested_by.as_deref());
                            let track = Track {
                                id: record.id.clone(),
                                title: record.title,
                                artist: record.artist,
                                album: None,
                                duration_ms: record.duration_sec * 1000,
                                cover_url: None,
                                stream_url: record.cached_path.as_deref().map(clean_stream_url),
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
            CommandAction::Jump { queue_id } => {
                *self.position_ms.lock().await = 0;
                match self.db.jump_queue_for_cabin(queue_id, cabin_id) {
                    Ok(Some(entry)) => {
                        if let Ok(Some(record)) = self.db.get_track(&entry.track_id) {
                            let _ = self.db.record_playback_history(cabin_id, &record.id, entry.requested_by.as_deref());
                            let track = Track {
                                id: record.id.clone(),
                                title: record.title,
                                artist: record.artist,
                                album: None,
                                duration_ms: record.duration_sec * 1000,
                                cover_url: None,
                                stream_url: record.cached_path.as_deref().map(clean_stream_url),
                                requested_by: entry.requested_by,
                            };
                            *self.current_track.lock().await = Some(track);
                            *self.playback_state.lock().await = PlaybackState::Playing;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => return Err(format!("Database queue jump failed: {}", e)),
                }
            }
            CommandAction::QueueAppend { track_id, requested_by } => {
                if let Err(e) = self.db.enqueue_track_with_meta(&track_id, requested_by.as_deref(), cabin_id) {
                    return Err(format!("Failed to enqueue track: {}", e));
                }
            }
            CommandAction::QueueNext { track_id, requested_by } => {
                if let Err(e) = self.db.enqueue_track_next_with_meta(&track_id, requested_by.as_deref(), cabin_id) {
                    return Err(format!("Failed to enqueue track next: {}", e));
                }
            }
            CommandAction::PlayNow { track_id, requested_by } => {
                *self.position_ms.lock().await = 0;
                match self.db.play_now_with_meta(&track_id, requested_by.as_deref(), cabin_id) {
                    Ok(entry) => {
                        if let Ok(Some(record)) = self.db.get_track(&entry.track_id) {
                            let _ = self.db.record_playback_history(cabin_id, &record.id, entry.requested_by.as_deref());
                            let track = Track {
                                id: record.id.clone(),
                                title: record.title,
                                artist: record.artist,
                                album: None,
                                duration_ms: record.duration_sec * 1000,
                                cover_url: None,
                                stream_url: record.cached_path.as_deref().map(clean_stream_url),
                                requested_by: entry.requested_by,
                            };
                            *self.current_track.lock().await = Some(track);
                            *self.playback_state.lock().await = PlaybackState::Playing;
                        }
                    }
                    Err(e) => return Err(format!("Failed to play track now: {}", e)),
                }
            }
            CommandAction::StartRadio { track_id, requested_by } => {
                *self.position_ms.lock().await = 0;
                match self.db.start_radio_with_meta(&track_id, requested_by.as_deref(), cabin_id) {
                    Ok(entry) => {
                        if let Ok(Some(record)) = self.db.get_track(&entry.track_id) {
                            let _ = self.db.record_playback_history(cabin_id, &record.id, entry.requested_by.as_deref());
                            let track = Track {
                                id: record.id.clone(),
                                title: record.title,
                                artist: record.artist,
                                album: None,
                                duration_ms: record.duration_sec * 1000,
                                cover_url: None,
                                stream_url: record.cached_path.as_deref().map(clean_stream_url),
                                requested_by: entry.requested_by,
                            };
                            *self.current_track.lock().await = Some(track);
                            *self.playback_state.lock().await = PlaybackState::Playing;
                        }
                    }
                    Err(e) => return Err(format!("Failed to start radio from track: {}", e)),
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
            CommandAction::EjectPassenger { ref passenger_id } => {
                let _ = self.db.eject_passenger(passenger_id);
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

        // Persist cabin playback state to SQLite
        {
            let cabin_playing_tid = self.db.get_queue_for_cabin(cabin_id).ok().and_then(|q| {
                q.into_iter()
                    .find(|(entry, _)| entry.state == crate::db::QueueState::Playing)
                    .map(|(entry, _)| entry.track_id)
            });

            let tid_opt = if cabin_playing_tid.is_some() {
                cabin_playing_tid
            } else if cabin_id == "default" {
                self.current_track.lock().await.as_ref().map(|t| t.id.clone())
            } else {
                None
            };

            let pos = *self.position_ms.lock().await;
            let st = match *self.playback_state.lock().await {
                PlaybackState::Playing => "playing",
                PlaybackState::Paused => "paused",
                PlaybackState::Buffering => "buffering",
                PlaybackState::Stopped => "stopped",
            };

            if let Some(ref tid) = tid_opt {
                let _ = self.db.save_cabin_state(cabin_id, Some(tid), pos, st);
            } else if is_seek {
                if let Ok(Some(existing)) = self.db.get_cabin_state(cabin_id) {
                    let _ = self.db.save_cabin_state(cabin_id, existing.track_id.as_deref(), pos, &existing.playback_state);
                }
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

                    if *pos % 5000 == 0 {
                        let cid = self.cabin_id.lock().await.clone();
                        let tid = self.current_track.lock().await.as_ref().map(|t| t.id.clone());
                        let _ = self.db.save_cabin_state(&cid, tid.as_deref(), *pos, "playing");
                    }
                }

                // Broadcast state update to active cabins and default
                let mut cabin_ids = vec!["default".to_string()];
                if let Ok(cabins) = self.db.list_cabins() {
                    for c in cabins {
                        if !cabin_ids.contains(&c.id) {
                            cabin_ids.push(c.id);
                        }
                    }
                }
                for cid in cabin_ids {
                    let update = self.build_state_update_for_cabin(&cid).await;
                    let _ = self.tx.send(update);
                }
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

    let cabin_id_for_send = cabin_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let should_send = match &msg {
                WsMessage::StateUpdate(s) => {
                    s.payload.cabin_id.as_deref() == Some(&cabin_id_for_send)
                }
                WsMessage::CrewUpdate(c) => c.cabin_id == cabin_id_for_send,
                WsMessage::Ejected(e) => e.cabin_id == cabin_id_for_send,
                WsMessage::CabinLocked(l) => l.cabin_id == cabin_id_for_send,
                WsMessage::CacheAlert(a) => a.cabin_id == cabin_id_for_send,
                WsMessage::Command(_) => false,
            };

            if should_send {
                if let Ok(text) = serde_json::to_string(&msg) {
                    if sender.send(Message::Text(text)).await.is_err() {
                        break;
                    }
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

        // 4. Jump Command
        let track2 = TrackRecord {
            id: "trk_beta".into(),
            artist: "Daft Punk".into(),
            title: "One More Time".into(),
            duration_sec: 320,
            cached_path: Some("/cache/daft_punk_one_more_time.opus".into()),
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&track2).unwrap();
        let q_id = db.enqueue_track("trk_beta").unwrap();
        hub.dispatch_command(CommandAction::Jump { queue_id: q_id }).await.unwrap();
        let received4 = rx.recv().await.unwrap();
        match received4 {
            WsMessage::StateUpdate(u) => {
                assert_eq!(u.payload.playback.state, PlaybackState::Playing);
                assert!(u.payload.track.is_some());
                assert_eq!(u.payload.track.unwrap().title, "One More Time");
            }
            _ => panic!("Expected StateUpdate for Jump"),
        }

        // 5. PlayNow Command
        let track3 = TrackRecord {
            id: "trk_now".into(),
            artist: "MC Serginho".into(),
            title: "Vai Lacraia".into(),
            duration_sec: 240,
            cached_path: Some("/cache/serginho_vai_lacraia.opus".into()),
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&track3).unwrap();
        hub.dispatch_command(CommandAction::PlayNow {
            track_id: "trk_now".into(),
            requested_by: Some("PILOTO".into()),
        }).await.unwrap();
        let received5 = rx.recv().await.unwrap();
        match received5 {
            WsMessage::StateUpdate(u) => {
                assert_eq!(u.payload.playback.state, PlaybackState::Playing);
                assert!(u.payload.track.is_some());
                assert_eq!(u.payload.track.unwrap().title, "Vai Lacraia");
            }
            _ => panic!("Expected StateUpdate for PlayNow"),
        }

        // 6. QueueNext Command
        let track4 = TrackRecord {
            id: "trk_next".into(),
            artist: "Legião Urbana".into(),
            title: "Tempo Perdido".into(),
            duration_sec: 300,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&track4).unwrap();
        hub.dispatch_command(CommandAction::QueueNext {
            track_id: "trk_next".into(),
            requested_by: Some("@Mari".into()),
        }).await.unwrap();
        let received6 = rx.recv().await.unwrap();
        match received6 {
            WsMessage::StateUpdate(u) => {
                let upcoming: Vec<_> = u.payload.queue.iter().filter(|q| q.state != "playing").collect();
                assert!(!upcoming.is_empty());
                assert_eq!(upcoming[0].title, "Tempo Perdido");
            }
            _ => panic!("Expected StateUpdate for QueueNext"),
        }

        // 7. StartRadio Command
        let track5 = TrackRecord {
            id: "trk_radio".into(),
            artist: "Chico Buarque".into(),
            title: "Construção".into(),
            duration_sec: 258,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&track5).unwrap();
        hub.dispatch_command(CommandAction::StartRadio {
            track_id: "trk_radio".into(),
            requested_by: Some("PILOTO".into()),
        }).await.unwrap();
        let received7 = rx.recv().await.unwrap();
        match received7 {
            WsMessage::StateUpdate(u) => {
                assert_eq!(u.payload.playback.state, PlaybackState::Playing);
                assert!(u.payload.track.is_some());
                assert_eq!(u.payload.track.unwrap().title, "Construção");
                let upcoming: Vec<_> = u.payload.queue.iter().filter(|q| q.state != "playing").collect();
                assert!(upcoming.is_empty());
            }
            _ => panic!("Expected StateUpdate for StartRadio"),
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

    #[tokio::test]
    async fn test_playback_hub_cabin_queue_isolation() {
        let db = Database::open_in_memory().unwrap();
        let (tx, _rx) = broadcast::channel(10);
        let hub = Arc::new(PlaybackHub::new(db.clone(), tx));

        // Enqueue to cabin A
        hub.dispatch_command_with_role(
            CommandAction::QueueAppend {
                track_id: "artist:song_a".into(),
                requested_by: Some("@Tripulante A".into()),
            },
            "cabin_alpha",
            Some(Role::Pilot),
        )
        .await
        .unwrap();

        // Enqueue to cabin B
        hub.dispatch_command_with_role(
            CommandAction::QueueAppend {
                track_id: "artist:song_b".into(),
                requested_by: Some("@Tripulante B".into()),
            },
            "cabin_beta",
            Some(Role::Pilot),
        )
        .await
        .unwrap();

        let state_alpha = hub.build_state_update_for_cabin("cabin_alpha").await;
        let state_beta = hub.build_state_update_for_cabin("cabin_beta").await;

        match state_alpha {
            WsMessage::StateUpdate(u) => {
                assert_eq!(u.payload.cabin_id, Some("cabin_alpha".into()));
                assert_eq!(u.payload.queue.len(), 1);
                assert_eq!(u.payload.queue[0].title, "song_a");
            }
            _ => panic!("Expected StateUpdate for cabin_alpha"),
        }

        match state_beta {
            WsMessage::StateUpdate(u) => {
                assert_eq!(u.payload.cabin_id, Some("cabin_beta".into()));
                assert_eq!(u.payload.queue.len(), 1);
                assert_eq!(u.payload.queue[0].title, "song_b");
            }
            _ => panic!("Expected StateUpdate for cabin_beta"),
        }
    }

    #[tokio::test]
    async fn test_playback_hub_state_persistence_and_resume() {
        let db = Database::open_in_memory().unwrap();
        let track = TrackRecord {
            id: "trk_persist".into(),
            artist: "MC Serginho".into(),
            title: "Vai Lacraia".into(),
            duration_sec: 240,
            cached_path: Some("/cache/vai_lacraia.opus".into()),
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&track).unwrap();

        let (tx, _rx) = broadcast::channel(100);
        let hub = Arc::new(PlaybackHub::new(db.clone(), tx));

        // 1. PlayNow on cabin 'cab_persisted'
        hub.dispatch_command_with_role(
            CommandAction::PlayNow {
                track_id: "trk_persist".into(),
                requested_by: Some("PILOTO".into()),
            },
            "cab_persisted",
            Some(Role::Pilot),
        )
        .await
        .unwrap();

        // 2. Seek to 25000 ms
        hub.dispatch_command_with_role(
            CommandAction::Seek { position_ms: 25000 },
            "cab_persisted",
            Some(Role::Pilot),
        )
        .await
        .unwrap();

        // Check SQLite persistence
        let persisted = db.get_cabin_state("cab_persisted").unwrap().expect("State must be persisted in SQLite");
        assert_eq!(persisted.track_id.as_deref(), Some("trk_persist"));
        assert_eq!(persisted.position_ms, 25000);

        // 3. New hub instance simulates daemon/process restart
        let (tx2, _rx2) = broadcast::channel(100);
        let hub2 = Arc::new(PlaybackHub::new(db.clone(), tx2));
        let state_resumed = hub2.build_state_update_for_cabin("cab_persisted").await;

        match state_resumed {
            WsMessage::StateUpdate(u) => {
                assert!(u.payload.track.is_some(), "Restored cabin state must include last track");
                assert_eq!(u.payload.track.unwrap().id, "trk_persist");
                assert_eq!(u.payload.playback.position_ms, 25000, "Restored cabin position must match SQLite");
            }
            _ => panic!("Expected StateUpdate on resumed cabin"),
        }
    }

    #[tokio::test]
    async fn test_playback_hub_records_history() {
        let db = Database::open_in_memory().unwrap();
        let track = TrackRecord {
            id: "trk_hist_1".into(),
            artist: "MC Serginho".into(),
            title: "Vai Lacraia".into(),
            duration_sec: 180,
            cached_path: Some("/cache/vai_lacraia.opus".into()),
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&track).unwrap();

        let (tx, _rx) = broadcast::channel(100);
        let hub = Arc::new(PlaybackHub::new(db.clone(), tx));

        // Pilot plays track
        hub.dispatch_command_with_role(
            CommandAction::PlayNow {
                track_id: "trk_hist_1".into(),
                requested_by: Some("PASSENGER_ALEX".into()),
            },
            "cab_history_test",
            Some(Role::Pilot),
        )
        .await
        .unwrap();

        let history = db.get_playback_history(Some("cab_history_test"), None, 10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].track_id, "trk_hist_1");
        assert_eq!(history[0].passenger_name.as_deref(), Some("PASSENGER_ALEX"));
        assert_eq!(history[0].cabin_id, "cab_history_test");
        assert_eq!(history[0].duration_sec, 180);
    }

    #[tokio::test]
    async fn test_playback_hub_cache_starvation_evaluation() {
        let db = Database::open_in_memory().unwrap();
        let (tx, mut rx) = broadcast::channel(100);
        let hub = Arc::new(PlaybackHub::new(db.clone(), tx));

        // 1. Uncached track
        let uncached = TrackRecord {
            id: "trk_cloud".into(),
            artist: "MC Serginho".into(),
            title: "Vai Lacraia".into(),
            duration_sec: 180,
            cached_path: None,
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&uncached).unwrap();

        let alert = hub.evaluate_cache_status("trk_cloud");
        assert!(alert.is_some(), "Uncached track must produce starvation alert");
        let alert = alert.unwrap();
        assert_eq!(alert.starvation_type, CacheStarvationType::LocalCacheMissing);
        assert!(alert.message.contains("CACHE LOCAL"));

        // 2. Cached track
        let cached = TrackRecord {
            id: "trk_local".into(),
            artist: "Kraftwerk".into(),
            title: "Autobahn".into(),
            duration_sec: 300,
            cached_path: Some("/cache/autobahn.opus".into()),
            genres: None,
            last_played_at: None,
        };
        db.upsert_track(&cached).unwrap();

        let alert_cached = hub.evaluate_cache_status("trk_local");
        assert!(alert_cached.is_none(), "Cached track must have no local cache missing alert");

        // 3. Emit remote buffer starvation alert
        hub.emit_cache_alert(
            "cab_starve",
            CacheStarvationType::RemoteBufferStarved,
            "BUFFER REMOTO ESGOTADO :: AGUARDANDO FLUXO",
            Some("trk_cloud".into()),
            true,
        )
        .await;

        let received = rx.recv().await.unwrap();
        match received {
            WsMessage::CacheAlert(a) => {
                assert_eq!(a.cabin_id, "cab_starve");
                assert_eq!(a.starvation_type, CacheStarvationType::RemoteBufferStarved);
                assert!(a.active);
            }
            _ => panic!("Expected WsMessage::CacheAlert"),
        }
    }
}

