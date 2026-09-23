//! The server -> client downlink.
//!
//! Every frame opens with `[frame_type: u8][tick: u32 LE]`.
//!
//! This is where the two-byte uplink gets paid for. A client can never
//! acknowledge anything, so the server cannot delta against a "last acked
//! tick". Instead it interleaves:
//!
//! * [`Keyframe`] -- complete state, every `keyframe_interval` ticks, and
//! * [`Delta`]    -- changes since an explicitly named `base_tick`.
//!
//! A client that misses a datagram discards deltas whose `base_tick` it does
//! not hold and waits for the next keyframe. Recovery is therefore
//! time-bounded rather than ack-bounded, which is the only option available.

mod assigned;
pub mod delta;
mod frame;
pub mod keyframe;
mod lobby_status;
mod match_end;
mod match_init;

pub use assigned::Assigned;
pub use delta::{record_type, records_from_events, Delta, DeltaRecord};
pub use frame::{frame_type, ServerFrame};
pub use keyframe::{BombSnapshot, FlameCell, Keyframe, PlayerSnapshot, PowerupSnapshot};
pub use lobby_status::LobbyStatus;
pub use match_end::{MatchEnd, PlayerResultRecord};
pub use match_init::{decode_rules, encode_rules, MatchInit, RULES_ENCODED_LEN};
