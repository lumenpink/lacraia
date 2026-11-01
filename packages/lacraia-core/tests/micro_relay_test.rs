use futures::{SinkExt, StreamExt};
use lacraia_core::relay::build_relay_app;
use serde_json::Value;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

async fn spawn_test_relay() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = build_relay_app();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, handle)
}

#[tokio::test]
async fn test_micro_relay_room_lifecycle_and_message_forwarding() {
    let (addr, _server_handle) = spawn_test_relay().await;

    // 1. Conectar o som da van como "host" na sala da van
    let host_url = format!("ws://{}/relay/ws?room=van-furacao&role=host", addr);
    let (mut host_ws, _) = connect_async(&host_url).await.expect("host connect");

    // 2. Conectar o celular do passageiro (via 5G) como "client" na mesma sala
    let client_url = format!("ws://{}/relay/ws?room=van-furacao&role=client", addr);
    let (mut client_ws, _) = connect_async(&client_url).await.expect("client connect");

    // 3. Passageiro envia comando de enfileiramento em JSON
    let cmd_payload = serde_json::json!({
        "action": "QUEUE_APPEND",
        "trackId": "online:track123",
        "requestedBy": "Passageiro no 5G"
    });
    client_ws
        .send(Message::Text(cmd_payload.to_string()))
        .await
        .unwrap();

    // 4. O som da van (host) deve receber o comando imediatamente
    let host_msg = tokio::time::timeout(Duration::from_secs(2), host_ws.next())
        .await
        .expect("timeout aguardando mensagem no host")
        .expect("stream finalizada")
        .expect("erro no websocket");

    if let Message::Text(text) = host_msg {
        let parsed: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["action"], "QUEUE_APPEND");
        assert_eq!(parsed["trackId"], "online:track123");
        assert_eq!(parsed["requestedBy"], "Passageiro no 5G");
    } else {
        panic!("Esperava mensagem de texto JSON no host");
    }

    // 5. O som da van emite STATE_UPDATE
    let state_payload = serde_json::json!({
        "type": "STATE_UPDATE",
        "payload": {
            "track": {
                "title": "Vai Lacraia",
                "artist": "MC Serginho"
            }
        }
    });
    host_ws
        .send(Message::Text(state_payload.to_string()))
        .await
        .unwrap();

    // 6. O celular do passageiro deve receber a atualização de estado
    let client_msg = tokio::time::timeout(Duration::from_secs(2), client_ws.next())
        .await
        .expect("timeout aguardando mensagem no client")
        .expect("stream finalizada")
        .expect("erro no websocket");

    if let Message::Text(text) = client_msg {
        let parsed: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "STATE_UPDATE");
        assert_eq!(parsed["payload"]["track"]["title"], "Vai Lacraia");
    } else {
        panic!("Esperava mensagem de texto JSON no cliente");
    }
}

#[tokio::test]
async fn test_micro_relay_room_isolation() {
    let (addr, _server_handle) = spawn_test_relay().await;

    // Host e Cliente 1 na sala "van-furacao"
    let host_url = format!("ws://{}/relay/ws?room=van-furacao&role=host", addr);
    let (mut host_ws, _) = connect_async(&host_url).await.unwrap();

    let client1_url = format!("ws://{}/relay/ws?room=van-furacao&role=client", addr);
    let (mut client1_ws, _) = connect_async(&client1_url).await.unwrap();

    // Cliente 2 em outra sala "outra-van"
    let client2_url = format!("ws://{}/relay/ws?room=outra-van&role=client", addr);
    let (mut client2_ws, _) = connect_async(&client2_url).await.unwrap();

    // Host da van-furacao envia mensagem
    let msg = serde_json::json!({
        "type": "STATE_UPDATE",
        "room": "van-furacao"
    });
    host_ws.send(Message::Text(msg.to_string())).await.unwrap();

    // Cliente 1 recebe
    let c1_msg = tokio::time::timeout(Duration::from_secs(1), client1_ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(c1_msg.is_text());

    // Cliente 2 não deve receber nada (timeout)
    let c2_res = tokio::time::timeout(Duration::from_millis(300), client2_ws.next()).await;
    assert!(
        c2_res.is_err(),
        "Cliente em sala diferente não deve receber mensagens de outra van"
    );
}

#[tokio::test]
async fn test_micro_relay_host_offline_notification() {
    let (addr, _server_handle) = spawn_test_relay().await;

    // Cliente conecta numa sala sem nenhum host online
    let client_url = format!("ws://{}/relay/ws?room=van-fantasma&role=client", addr);
    let (mut client_ws, _) = connect_async(&client_url).await.unwrap();

    // Deve receber notificação de HOST_OFFLINE
    let msg = tokio::time::timeout(Duration::from_secs(1), client_ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    if let Message::Text(text) = msg {
        let parsed: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "RELAY_STATUS");
        assert_eq!(parsed["status"], "HOST_OFFLINE");
    } else {
        panic!("Esperava notificação de status em JSON");
    }
}
