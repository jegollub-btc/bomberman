use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use bomber_domain::shared::PlayerId;
use bomber_protocol::{ClientPacket, ServerFrame, MAX_DATAGRAM};
use tokio::net::UdpSocket;

use crate::runtime::Shared;

/// The bot-facing socket.
#[derive(Clone)]
pub struct Endpoint {
    socket: Arc<UdpSocket>,
}

impl Endpoint {
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        tracing::info!(%addr, "listening for bots");
        Ok(Endpoint {
            socket: Arc::new(socket),
        })
    }

    pub async fn send(&self, addr: SocketAddr, frame: &ServerFrame) {
        let bytes = frame.encode();
        if bytes.len() > MAX_DATAGRAM {
            // Better a loud log than a datagram that silently never arrives.
            tracing::warn!(
                frame = frame.frame_type(),
                size = bytes.len(),
                "frame exceeds the datagram budget and may be dropped in flight"
            );
        }
        if let Err(error) = self.socket.send_to(&bytes, addr).await {
            tracing::debug!(%addr, %error, "send failed");
        }
    }

    pub async fn send_to_player(&self, shared: &Shared, player: PlayerId, frame: &ServerFrame) {
        let addr = shared.registry.lock().unwrap().addr_of(player);
        if let Some(addr) = addr {
            self.send(addr, frame).await;
        }
    }
}

/// Receive datagrams until the socket dies.
pub async fn listen(endpoint: Endpoint, shared: Shared) {
    let mut buf = [0u8; 64];
    loop {
        let (len, addr) = match endpoint.socket.recv_from(&mut buf).await {
            Ok(received) => received,
            Err(error) => {
                tracing::error!(%error, "udp receive failed");
                continue;
            }
        };

        // Anything can arrive on a UDP port. A packet that does not decode is
        // not worth a log line at info level -- it is the normal background
        // noise of an open port.
        let Ok(packet) = ClientPacket::decode(&buf[..len]) else {
            continue;
        };

        if packet.is_hello() {
            handle_hello(&endpoint, &shared, addr).await;
        } else {
            handle_action(&shared, addr, packet);
        }
    }
}

async fn handle_hello(endpoint: &Endpoint, shared: &Shared, addr: SocketAddr) {
    let existing = shared.registry.lock().unwrap().player_of(&addr);

    let (player, frames) = {
        let mut session = shared.session.lock().unwrap();
        let player = match existing {
            Some(player) => player,
            None => match session.admit() {
                Ok(player) => {
                    shared.registry.lock().unwrap().bind(addr, player);
                    tracing::info!(%addr, %player, "seated");
                    player
                }
                Err(reason) => {
                    // The bot is expected to retry, so this is routine.
                    tracing::debug!(%addr, ?reason, "refused");
                    return;
                }
            },
        };

        // Re-sending the match frame is the documented recovery path for a bot
        // that missed it: without acknowledgements, asking again is all it has.
        let mut frames = vec![session.assigned_frame(player)];
        frames.extend(session.match_init_frame_for(player));
        frames.push(session.lobby_status_frame());
        (player, frames)
    };

    for frame in &frames {
        endpoint.send(addr, frame).await;
    }
    shared.notify_lobby_changed();
    let _ = player;
}

fn handle_action(shared: &Shared, addr: SocketAddr, packet: ClientPacket) {
    let Some(bound) = shared.registry.lock().unwrap().player_of(&addr) else {
        return;
    };
    // Byte 0 is a claim, not a credential. Believe it only from the address
    // that owns the seat.
    if packet.claimed_player() != Some(bound) {
        tracing::debug!(%addr, claimed = packet.player_id, %bound, "id does not match source");
        return;
    }

    let mut session = shared.session.lock().unwrap();
    let _ = session.submit(bound, packet.action, packet.seq);
}
