use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};

use crate::runtime::Shared;

pub async fn handler(ws: WebSocketUpgrade, State(shared): State<Shared>) -> Response {
    ws.on_upgrade(|socket| watch(socket, shared))
}

/// Read-only: push the feed, ignore anything the client says.
async fn watch(socket: WebSocket, shared: Shared) {
    let (mut sender, mut receiver) = socket.split();
    let mut feed = shared.subscribe();

    // A spectator connecting mid-match must not stare at nothing until the next
    // one starts.
    for message in shared.welcome_messages() {
        if sender.send(Message::Text(message.to_json())).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            incoming = receiver.next() => match incoming {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                _ => {}
            },
            update = feed.recv() => match update {
                Ok(json) => {
                    if sender.send(Message::Text(json.to_string())).await.is_err() {
                        break;
                    }
                }
                // Lagged: this watcher fell behind. State is complete every
                // tick, so skipping ahead costs a frame of animation and
                // nothing else.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::debug!(skipped, "spectator fell behind");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
        }
    }
}
