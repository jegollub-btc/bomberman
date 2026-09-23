use bomber_domain::shared::PlayerId;

use crate::codec::{ProtoError, Result};

use super::{Action, SEQ_MASK};

/// Sent in byte 0 to request a slot. Not a real player id.
pub const HELLO_PLAYER_ID: u8 = 0xFF;

/// The complete hello datagram.
///
/// Byte 1 happens to decode as seq 15 / [`Action::Hello`], so the format stays
/// uniform rather than needing a special case.
pub const HELLO_PACKET: [u8; 2] = [HELLO_PLAYER_ID, 0xFF];

/// A decoded uplink datagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientPacket {
    /// Raw, because a hello carries `0xFF` rather than a real player.
    pub player_id: u8,
    /// Wrapping 0..=15 counter, incremented by the bot on every send.
    pub seq: u8,
    pub action: Action,
}

impl ClientPacket {
    pub const LEN: usize = 2;

    pub fn new(player: PlayerId, seq: u8, action: Action) -> Self {
        Self {
            player_id: player.raw(),
            seq: seq & SEQ_MASK,
            action,
        }
    }

    pub fn hello() -> Self {
        Self {
            player_id: HELLO_PLAYER_ID,
            seq: SEQ_MASK,
            action: Action::Hello,
        }
    }

    pub fn encode(&self) -> [u8; Self::LEN] {
        [self.player_id, (self.seq << 4) | self.action.code()]
    }

    pub fn decode(buf: &[u8]) -> Result<Self> {
        if buf.len() < Self::LEN {
            return Err(ProtoError::Truncated {
                at: buf.len(),
                need: Self::LEN - buf.len(),
            });
        }
        Ok(ClientPacket {
            player_id: buf[0],
            seq: buf[1] >> 4,
            action: Action::from_code(buf[1] & 0x0F)?,
        })
    }

    pub fn is_hello(&self) -> bool {
        self.player_id == HELLO_PLAYER_ID || self.action == Action::Hello
    }

    /// The claimed player, or `None` for a hello.
    ///
    /// A claim is exactly that: the server still has to check it against the
    /// address the datagram arrived from before acting on it.
    pub fn claimed_player(&self) -> Option<PlayerId> {
        (!self.is_hello()).then(|| PlayerId::new(self.player_id))
    }
}
