use bomber_domain::board::Board;
use bomber_domain::lobby::LobbyState;
use bomber_domain::shared::PlayerId;

use super::{MapSettings, SeatPresence};

/// One seat, as the moderation UI sees it.
///
/// The transport address is not here: the session deals in seats and player
/// ids, and who is on the other end of a socket is the adapter's business.
#[derive(Debug, Clone, PartialEq)]
pub struct SeatView {
    pub id: PlayerId,
    pub name: String,
    pub connected: bool,
    pub presence: SeatPresence,
    /// Ticks since this seat last sent anything. `None` if it never has.
    pub stale_ticks: Option<u32>,
    /// Whether `stale_ticks` has passed the configured threshold.
    pub stale: bool,
}

/// Everything a UI needs to render the lobby, in one read.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSnapshot {
    pub state: LobbyState,
    pub paused: bool,
    /// The server's own answer to "would Start succeed right now", so a UI can
    /// drive its button from the same rule the server enforces instead of
    /// re-deriving it and drifting.
    pub can_start: bool,
    pub min_players: u8,
    pub max_players: u8,
    pub countdown_ticks: u16,
    pub map: MapSettings,
    pub seats: Vec<SeatView>,
    pub session_tick: u32,
    /// The board of the match in progress, or the last one played.
    pub board: Option<Board>,
}
