use serde::{Deserialize, Serialize};

use bomber_domain::shared::PlayerId;

use crate::codec::{Reader, Result, Writer};

/// Reply to a hello: you are in, and this is who you are.
///
/// The id is bound to the source address of the hello. Everything the bot sends
/// afterwards is checked against that address, which is the only authentication
/// a two-byte uplink can support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assigned {
    pub tick: u32,
    pub protocol_version: u8,
    pub player_id: PlayerId,
    pub tick_rate: u8,
    pub max_players: u8,
}

impl Assigned {
    pub fn encode(&self, w: &mut Writer) {
        w.u8(self.protocol_version)
            .u8(self.player_id.raw())
            .u8(self.tick_rate)
            .u8(self.max_players);
    }

    pub fn decode(tick: u32, r: &mut Reader) -> Result<Self> {
        Ok(Assigned {
            tick,
            protocol_version: r.u8()?,
            player_id: PlayerId::new(r.u8()?),
            tick_rate: r.u8()?,
            max_players: r.u8()?,
        })
    }
}
