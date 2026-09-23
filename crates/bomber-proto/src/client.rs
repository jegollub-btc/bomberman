//! The client -> server uplink: exactly two bytes, forever.
//!
//! ```text
//! byte 0: player_id   0..=3, or 0xFF to say hello
//! byte 1: (seq << 4) | action_code
//! ```
//!
//! Two bytes is the whole budget, which rules out acknowledgements, tokens and
//! resync requests. Everything that looks unusual about this protocol -- the
//! keyframe cadence, address-bound identity -- follows from that.
//!
//! The one piece of spare room is the high nibble of byte 1. Spending it on a
//! wrapping sequence counter is what lets the server drop duplicated and
//! reordered datagrams and measure per-bot loss; a bare action byte could do
//! neither.

use serde::{Deserialize, Serialize};

use crate::codec::{ProtoError, Result};

/// Sent in byte 0 to request a slot. Not a real player id.
pub const HELLO_PLAYER_ID: u8 = 0xFF;

/// The complete hello datagram: `[0xFF, 0xFF]`.
///
/// Byte 1 decodes as seq 15 / action `Hello`, so the format stays uniform.
pub const HELLO_PACKET: [u8; 2] = [HELLO_PLAYER_ID, 0xFF];

/// What a player wants to do on a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Action {
    Noop = 0,
    Up = 1,
    Down = 2,
    Left = 3,
    Right = 4,
    Bomb = 5,
    UpBomb = 6,
    DownBomb = 7,
    LeftBomb = 8,
    RightBomb = 9,
    /// Request a slot, or ask for `MATCH_INIT` to be re-sent.
    Hello = 15,
}

use crate::types::Direction;

impl Action {
    pub fn from_code(code: u8) -> Result<Self> {
        Ok(match code {
            0 => Action::Noop,
            1 => Action::Up,
            2 => Action::Down,
            3 => Action::Left,
            4 => Action::Right,
            5 => Action::Bomb,
            6 => Action::UpBomb,
            7 => Action::DownBomb,
            8 => Action::LeftBomb,
            9 => Action::RightBomb,
            15 => Action::Hello,
            other => {
                return Err(ProtoError::InvalidValue {
                    field: "action",
                    value: other as u32,
                })
            }
        })
    }

    pub fn code(self) -> u8 {
        self as u8
    }

    /// The movement component, if any.
    pub fn movement(self) -> Option<Direction> {
        match self {
            Action::Up | Action::UpBomb => Some(Direction::Up),
            Action::Down | Action::DownBomb => Some(Direction::Down),
            Action::Left | Action::LeftBomb => Some(Direction::Left),
            Action::Right | Action::RightBomb => Some(Direction::Right),
            _ => None,
        }
    }

    /// Whether this action also drops a bomb on the player's current cell.
    pub fn places_bomb(self) -> bool {
        matches!(
            self,
            Action::Bomb
                | Action::UpBomb
                | Action::DownBomb
                | Action::LeftBomb
                | Action::RightBomb
        )
    }

    /// Combine a direction with an optional bomb drop into a single action.
    pub fn moving(dir: Direction, bomb: bool) -> Action {
        match (dir, bomb) {
            (Direction::Up, false) => Action::Up,
            (Direction::Down, false) => Action::Down,
            (Direction::Left, false) => Action::Left,
            (Direction::Right, false) => Action::Right,
            (Direction::Up, true) => Action::UpBomb,
            (Direction::Down, true) => Action::DownBomb,
            (Direction::Left, true) => Action::LeftBomb,
            (Direction::Right, true) => Action::RightBomb,
        }
    }
}

/// A decoded uplink datagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientPacket {
    pub player_id: u8,
    /// Wrapping 0..=15 counter, incremented by the bot on every send.
    pub seq: u8,
    pub action: Action,
}

impl ClientPacket {
    pub const LEN: usize = 2;

    pub fn new(player_id: u8, seq: u8, action: Action) -> Self {
        Self {
            player_id,
            seq: seq & 0x0F,
            action,
        }
    }

    pub fn hello() -> Self {
        Self::new(HELLO_PLAYER_ID, 0x0F, Action::Hello)
    }

    pub fn encode(&self) -> [u8; 2] {
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
}

/// Is `seq` newer than `previous` in wrapping 4-bit space?
///
/// Half the 16-value space counts as "ahead". A bot sending one packet per tick
/// wraps every 16 ticks, so anything more than 8 ticks stale is indistinguishable
/// from fresh -- which is fine, since the server only ever cares about the newest
/// packet inside the current tick window.
pub fn seq_is_newer(seq: u8, previous: u8) -> bool {
    let diff = (seq.wrapping_sub(previous)) & 0x0F;
    diff != 0 && diff < 8
}
