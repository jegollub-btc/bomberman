use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use bomber_application::{CommandOutcome, ModeratorCommand};
use futures_util::{SinkExt, StreamExt};

use crate::runtime::Shared;

use super::dto::{board_dto, Inbound, Outbound};

pub async fn handler(ws: WebSocketUpgrade, State(shared): State<Shared>) -> Response {
    ws.on_upgrade(|socket| moderate(socket, shared))
}

async fn moderate(socket: WebSocket, shared: Shared) {
    let (mut sender, mut receiver) = socket.split();
    let mut feed = shared.subscribe();

    for message in shared.welcome_messages() {
        if sender.send(Message::Text(message.to_json())).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            incoming = receiver.next() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    let reply = handle(&shared, &text);
                    if sender.send(Message::Text(reply.to_json())).await.is_err() {
                        break;
                    }
                }
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                _ => {}
            },
            update = feed.recv() => match update {
                Ok(json) => {
                    if sender.send(Message::Text(json.to_string())).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
        }
    }
}

/// Every command gets exactly one reply.
fn handle(shared: &Shared, text: &str) -> Outbound {
    let inbound: Inbound = match serde_json::from_str(text) {
        Ok(inbound) => inbound,
        Err(error) => {
            return Outbound::Error {
                cmd: String::new(),
                message: format!("could not parse command: {error}"),
            }
        }
    };

    let command: ModeratorCommand = inbound.into();
    let name = command.name().to_string();

    let outcome = shared.session.lock().unwrap().execute(command);

    let reply = match outcome {
        CommandOutcome::Accepted => Outbound::Ack { cmd: name },
        CommandOutcome::AcceptedAndReleased(player) => {
            // The session freed the seat; the transport binding is ours to drop,
            // otherwise the kicked bot would keep driving it.
            shared.registry.lock().unwrap().release(player);
            Outbound::Ack { cmd: name }
        }
        CommandOutcome::Preview { board, seed } => {
            let rules = shared.session.lock().unwrap().config().rules;
            Outbound::MapPreview(board_dto(&board, 0, seed, rules, Vec::new()))
        }
        CommandOutcome::Rejected(message) => Outbound::Error { cmd: name, message },
    };

    // Any accepted command may have changed what the lobby looks like, and
    // every watcher -- not just the one that sent it -- needs to see that.
    if !matches!(reply, Outbound::Error { .. }) {
        shared.notify_lobby_changed();
    }
    reply
}
