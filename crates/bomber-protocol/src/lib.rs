//! The wire protocol.
//!
//! In hexagonal terms this is an adapter, not domain: it knows how to turn the
//! domain's vocabulary into bytes and back, and nothing else. The dependency
//! points inward -- protocol depends on [`bomber_domain`], never the reverse --
//! so the rules can be changed without touching a byte layout and a byte layout
//! can be changed without touching the rules.
//!
//! It is also the single source of truth for the layouts documented in
//! `BOT_GUIDE.md`; the constants there are the constants here.
//!
//! The shape of everything is dictated by one constraint: **the uplink is two
//! bytes.** See [`uplink`] for what that costs, and [`downlink`] for how the
//! server pays for it.

pub mod codec;
pub mod downlink;
pub mod grid;
pub mod uplink;

pub use codec::{ProtoError, Reader, Result, Writer};
pub use downlink::{
    frame_type, record_type, records_from_events, Assigned, BombSnapshot, Delta, DeltaRecord,
    FlameCell, Keyframe,
    LobbyStatus, MatchEnd, MatchInit, PlayerResultRecord, PlayerSnapshot, PowerupSnapshot,
    ServerFrame, RULES_ENCODED_LEN,
};
pub use uplink::{seq_is_newer, Action, ClientPacket, HELLO_PACKET, HELLO_PLAYER_ID};

/// Bumped whenever a layout changes incompatibly. Sent in `ASSIGNED` and
/// `MATCH_INIT` so a stale bot fails loudly instead of misreading bytes.
pub const PROTOCOL_VERSION: u8 = 1;

/// Default UDP port bots send their two bytes to.
pub const DEFAULT_UDP_PORT: u16 = 47800;

/// Default HTTP/WebSocket port for the visualizer and moderation UI.
pub const DEFAULT_WEB_PORT: u16 = 8080;

/// Simulation rate. Fixed rather than negotiated -- determinism depends on it.
pub const TICK_RATE: u8 = 60;

/// Hard ceiling on concurrent players, set by the 4-bit slot mask in
/// `LOBBY_STATUS` and by the number of corner spawns.
pub const MAX_PLAYERS: usize = 4;

/// Conservative payload ceiling: comfortably under the smallest path MTU worth
/// worrying about, so a frame is never fragmented or silently dropped.
pub const MAX_DATAGRAM: usize = 1200;

/// Sentinel for "no player" -- a draw, or a death with no kill credit.
pub const NO_PLAYER: u8 = 0xFF;

use bomber_domain::shared::PlayerId;

/// `None` is written as [`NO_PLAYER`].
pub fn encode_optional_player(id: Option<PlayerId>) -> u8 {
    id.map_or(NO_PLAYER, PlayerId::raw)
}

pub fn decode_optional_player(raw: u8) -> Option<PlayerId> {
    (raw != NO_PLAYER).then(|| PlayerId::new(raw))
}
