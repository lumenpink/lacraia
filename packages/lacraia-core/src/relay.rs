use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
pub struct RelayWsQuery {
    pub room: String,
    pub role: Option<String>,
}

#[derive(Clone)]
pub struct Room {
    pub host_tx: Arc<Mutex<Option<mpsc::UnboundedSender<String>>>>,
    pub clients_broadcast_tx: broadcast::Sender<String>,
}

impl Default for Room {
    fn default() -> Self {
        let (broadcast_tx, _rx) = broadcast::channel(128);
        Self {
            host_tx: Arc::new(Mutex::new(None)),
            clients_broadcast_tx: broadcast_tx,
        }
    }
}

#[derive(Clone, Default)]
pub struct RelayHub {
    rooms: Arc<RwLock<HashMap<String, Arc<Room>>>>,
}

impl RelayHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_or_create_room(&self, room_id: &str) -> Arc<Room> {
        let mut rooms = self.rooms.write().await;
        rooms
            .entry(room_id.to_string())
            .or_insert_with(|| Arc::new(Room::default()))
            .clone()
    }

    pub async fn room_count(&self) -> usize {
        self.rooms.read().await.len()
    }
}

#[derive(Serialize)]
pub struct RelayHealthResponse {
    pub status: &'static str,
    #[serde(rename = "activeRooms")]
    pub active_rooms: usize,
}

pub async fn relay_health(State(hub): State<RelayHub>) -> Json<RelayHealthResponse> {
    let count = hub.room_count().await;
    Json(RelayHealthResponse {
        status: "ok",
        active_rooms: count,
    })
}

pub async fn relay_ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<RelayWsQuery>,
    State(hub): State<RelayHub>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_relay_socket(socket, params, hub))
}

async fn handle_relay_socket(socket: WebSocket, params: RelayWsQuery, hub: RelayHub) {
    let room = hub.get_or_create_room(&params.room).await;
    let is_host = params.role.as_deref().unwrap_or("client") == "host";
    let (mut ws_sender, mut ws_receiver) = socket.split();

    if is_host {
        info!("Host connected to relay room '{}'", params.room);
        let (host_mpsc_tx, mut host_mpsc_rx) = mpsc::unbounded_channel::<String>();
        {
            let mut current_host = room.host_tx.lock().await;
            *current_host = Some(host_mpsc_tx);
        }

        // Forward broadcast state to connected clients when host is online
        let _ = room.clients_broadcast_tx.send(
            serde_json::json!({
                "type": "RELAY_STATUS",
                "status": "HOST_ONLINE"
            })
            .to_string(),
        );

        let mut send_task = tokio::spawn(async move {
            while let Some(msg) = host_mpsc_rx.recv().await {
                if ws_sender.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        let room_clients = room.clients_broadcast_tx.clone();
        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_receiver.next().await {
                if let Message::Text(text) = msg {
                    // Broadcast host message (e.g. STATE_UPDATE) to all connected clients
                    let _ = room_clients.send(text);
                }
            }
        });

        tokio::select! {
            _ = (&mut send_task) => recv_task.abort(),
            _ = (&mut recv_task) => send_task.abort(),
        }

        info!("Host disconnected from relay room '{}'", params.room);
        {
            let mut current_host = room.host_tx.lock().await;
            *current_host = None;
        }

        let _ = room.clients_broadcast_tx.send(
            serde_json::json!({
                "type": "RELAY_STATUS",
                "status": "HOST_OFFLINE"
            })
            .to_string(),
        );
    } else {
        info!("Client connected to relay room '{}'", params.room);
        let host_online = room.host_tx.lock().await.is_some();
        if !host_online {
            let offline_notice = serde_json::json!({
                "type": "RELAY_STATUS",
                "status": "HOST_OFFLINE"
            })
            .to_string();
            let _ = ws_sender.send(Message::Text(offline_notice)).await;
        }

        let mut broadcast_rx = room.clients_broadcast_tx.subscribe();
        let mut send_task = tokio::spawn(async move {
            while let Ok(msg) = broadcast_rx.recv().await {
                if ws_sender.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        let host_tx_ref = room.host_tx.clone();
        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_receiver.next().await {
                if let Message::Text(text) = msg {
                    let host_opt = host_tx_ref.lock().await;
                    if let Some(host_sender) = host_opt.as_ref() {
                        let _ = host_sender.send(text);
                    } else {
                        warn!("Client message dropped: host is offline in room");
                    }
                }
            }
        });

        tokio::select! {
            _ = (&mut send_task) => recv_task.abort(),
            _ = (&mut recv_task) => send_task.abort(),
        }

        info!("Client disconnected from relay room '{}'", params.room);
    }
}

pub fn build_relay_app() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let hub = RelayHub::new();

    Router::new()
        .route("/relay/health", get(relay_health))
        .route("/relay/ws", get(relay_ws_handler))
        .layer(cors)
        .with_state(hub)
}

pub async fn start_relay_host_connector(
    relay_url: String,
    room_id: String,
    hub: Arc<crate::PlaybackHub>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let target_url = format!("{relay_url}?room={room_id}&role=host");
            info!("Conectando ao micro-relay em {}", target_url);
            match tokio_tungstenite::connect_async(&target_url).await {
                Ok((ws_stream, _)) => {
                    info!("Conectado com sucesso ao micro-relay na sala {}", room_id);
                    let (mut ws_write, mut ws_read) = ws_stream.split();
                    let mut rx = hub.tx.subscribe();

                    // Snapshot inicial de estado para a sala
                    let initial_state = hub.build_state_update_for_cabin(&room_id).await;
                    if let Ok(json) = serde_json::to_string(&initial_state) {
                        let _ = ws_write
                            .send(tokio_tungstenite::tungstenite::Message::Text(json))
                            .await;
                    }

                    let room_id_send = room_id.clone();
                    let mut send_task = tokio::spawn(async move {
                        while let Ok(msg) = rx.recv().await {
                            let should_send = match &msg {
                                lacraia_shared::WsMessage::StateUpdate(s) => {
                                    s.payload.cabin_id.as_deref() == Some(&room_id_send)
                                }
                                lacraia_shared::WsMessage::CrewUpdate(c) => {
                                    c.cabin_id == room_id_send
                                }
                                lacraia_shared::WsMessage::Ejected(e) => e.cabin_id == room_id_send,
                                lacraia_shared::WsMessage::CabinLocked(l) => {
                                    l.cabin_id == room_id_send
                                }
                                lacraia_shared::WsMessage::CacheAlert(a) => {
                                    a.cabin_id == room_id_send
                                }
                                _ => false,
                            };
                            if should_send {
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    if ws_write
                                        .send(tokio_tungstenite::tungstenite::Message::Text(json))
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }
                                }
                            }
                        }
                    });

                    let hub_recv = hub.clone();
                    let room_id_recv = room_id.clone();
                    let mut recv_task = tokio::spawn(async move {
                        while let Some(Ok(msg)) = ws_read.next().await {
                            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                                let action_opt = if let Ok(lacraia_shared::WsMessage::Command(
                                    cmd,
                                )) =
                                    serde_json::from_str::<lacraia_shared::WsMessage>(&text)
                                {
                                    Some(cmd.payload)
                                } else if let Ok(act) =
                                    serde_json::from_str::<lacraia_shared::CommandAction>(&text)
                                {
                                    Some(act)
                                } else if let Ok(val) =
                                    serde_json::from_str::<serde_json::Value>(&text)
                                {
                                    if let Some(action_str) =
                                        val.get("action").and_then(|a| a.as_str())
                                    {
                                        match action_str {
                                            "QUEUE_APPEND" => {
                                                let track_id = val
                                                    .get("trackId")
                                                    .or_else(|| val.get("track_id"))
                                                    .and_then(|t| t.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let req_by = val
                                                    .get("requestedBy")
                                                    .or_else(|| val.get("requested_by"))
                                                    .and_then(|u| u.as_str())
                                                    .map(|s| s.to_string());
                                                Some(lacraia_shared::CommandAction::QueueAppend {
                                                    track_id,
                                                    requested_by: req_by,
                                                })
                                            }
                                            _ => None,
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                                if let Some(action) = action_opt {
                                    let _ = hub_recv
                                        .dispatch_command_with_role(
                                            action,
                                            &room_id_recv,
                                            Some(lacraia_shared::Role::Passenger),
                                        )
                                        .await;
                                }
                            }
                        }
                    });

                    tokio::select! {
                        _ = (&mut send_task) => recv_task.abort(),
                        _ = (&mut recv_task) => send_task.abort(),
                    }
                }
                Err(e) => {
                    warn!(
                        "Falha ao conectar no micro-relay: {}. Reconectando em 3s...",
                        e
                    );
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    })
}
