use serde::{Deserialize, Serialize};

use bomber_domain::lobby::LobbyState;

use crate::codec::{ProtoError, Reader, Result, Writer};

/// A heartbeat while waiting, so a bot can tell "nothing has started yet" from
/// "I have lost the server".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyStatus {
    pub tick: u32,
    pub state: LobbyState,
    pub players_connected: u8,
    pub max_players: u8,
    /// Bit `i` set means seat `i` is taken.
    pub slot_mask: u8,
    /// Ticks left in the start countdown; 0 outside `Countdown`.
    pub countdown_ticks: u16,
}

impl LobbyStatus {
    pub fn encode(&self, w: &mut Writer) {
        w.u8(self.state.code())
            .u8(self.players_connected)
            .u8(self.max_players)
            .u8(self.slot_mask)
            .u16(self.countdown_ticks);
    }

    pub fn decode(tick: u32, r: &mut Reader) -> Result<Self> {
        let raw = r.u8()?;
        Ok(LobbyStatus {
            tick,
            state: LobbyState::from_code(raw)
                .ok_or_else(|| ProtoError::invalid("lobby_state", raw))?,
            players_connected: r.u8()?,
            max_players: r.u8()?,
            slot_mask: r.u8()?,
            countdown_ticks: r.u16()?,
        })
    }
}
