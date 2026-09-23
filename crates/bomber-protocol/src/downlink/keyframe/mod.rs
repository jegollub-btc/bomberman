//! Complete state, sent periodically.
//!
//! A keyframe is the only frame a client can trust unconditionally. It carries
//! the whole grid rather than a patch list, because two bits per cell is
//! bounded and a patch list is not -- it grows for the whole match.

mod snapshots;

pub use snapshots::{BombSnapshot, FlameCell, PlayerSnapshot, PowerupSnapshot};

use serde::{Deserialize, Serialize};

use bomber_domain::board::TileGrid;
use bomber_domain::game::GameState;

use crate::codec::{Reader, Result, Writer};
use crate::grid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keyframe {
    pub tick: u32,
    pub grid: TileGrid,
    pub players: Vec<PlayerSnapshot>,
    pub bombs: Vec<BombSnapshot>,
    pub flames: Vec<FlameCell>,
    pub powerups: Vec<PowerupSnapshot>,
    pub ticks_remaining: u32,
}

impl Keyframe {
    pub fn from_state(state: &GameState) -> Self {
        Keyframe {
            tick: state.tick,
            grid: state.board.grid.clone(),
            players: state.players.iter().map(PlayerSnapshot::from_player).collect(),
            bombs: state.bombs.iter().map(BombSnapshot::from_bomb).collect(),
            flames: state
                .flames
                .burning()
                .map(|(cell, ticks)| FlameCell {
                    x: cell.x,
                    y: cell.y,
                    ticks_remaining: ticks,
                })
                .collect(),
            powerups: state
                .powerups
                .iter()
                .map(PowerupSnapshot::from_powerup)
                .collect(),
            ticks_remaining: state.ticks_remaining(),
        }
    }

    pub fn encode(&self, w: &mut Writer) {
        w.u8(self.grid.width()).u8(self.grid.height());
        grid::encode(&self.grid, w);

        w.u8(self.players.len() as u8);
        for player in &self.players {
            player.encode(w);
        }
        // u16 counts, not u8: a large board mid-chain-reaction can easily hold
        // more than 255 burning cells.
        w.u16(self.bombs.len() as u16);
        for bomb in &self.bombs {
            bomb.encode(w);
        }
        w.u16(self.flames.len() as u16);
        for flame in &self.flames {
            flame.encode(w);
        }
        w.u16(self.powerups.len() as u16);
        for powerup in &self.powerups {
            powerup.encode(w);
        }
        w.u32(self.ticks_remaining);
    }

    pub fn decode(tick: u32, r: &mut Reader) -> Result<Self> {
        let width = r.u8()?;
        let height = r.u8()?;
        let grid = grid::decode(r, width, height)?;

        let player_count = r.u8()? as usize;
        let players = r.repeat(player_count, PlayerSnapshot::decode)?;
        let bomb_count = r.u16()? as usize;
        let bombs = r.repeat(bomb_count, BombSnapshot::decode)?;
        let flame_count = r.u16()? as usize;
        let flames = r.repeat(flame_count, FlameCell::decode)?;
        let powerup_count = r.u16()? as usize;
        let powerups = r.repeat(powerup_count, PowerupSnapshot::decode)?;

        Ok(Keyframe {
            tick,
            grid,
            players,
            bombs,
            flames,
            powerups,
            ticks_remaining: r.u32()?,
        })
    }
}
