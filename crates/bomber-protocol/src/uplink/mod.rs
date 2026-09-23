//! The client -> server uplink: exactly two bytes, forever.
//!
//! ```text
//! byte 0:  player_id            0..=3, or 0xFF to say hello
//! byte 1:  (seq << 4) | action_code
//! ```
//!
//! Two bytes is the entire budget, and it rules out acknowledgements, tokens
//! and resync requests. Everything unusual about this protocol follows from
//! that single fact -- the keyframe cadence in [`crate::downlink`], and
//! identity being bound to a source address rather than carried in the packet.
//!
//! The only spare room is the high nibble of byte 1. Spending it on a wrapping
//! sequence counter is what lets the server discard duplicated and reordered
//! datagrams and measure per-bot loss; a bare action byte could do neither.

mod action;
mod hello;
mod packet;
mod sequence;

pub use action::Action;
pub use hello::Hello;
pub use packet::{ClientPacket, HELLO_PACKET, HELLO_PLAYER_ID};
pub use sequence::{seq_is_newer, SEQ_MASK, SEQ_SPACE};
