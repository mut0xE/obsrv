use crate::state::AppState;
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};

pub async fn handle(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.ws_tx.subscribe();

    loop {
        tokio::select! {
            // event from broadcast → send to browser
            Ok(event) = rx.recv() => {
                let msg = serde_json::to_string(&event)
                    .unwrap_or_default();

                if socket
                    .send(Message::Text(msg.into()))
                    .await
                    .is_err()
                {
                    break; // client disconnected
                }
            }

            // message from browser
            Some(Ok(msg)) = socket.recv() => {
                match msg {
                    Message::Text(text) if text == "ping" => {
                        let _ = socket
                            .send(Message::Text("pong".into()))
                            .await;
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }

            else => break,
        }
    }

    tracing::info!("WebSocket client disconnected");
}
