use serde::{Deserialize, Serialize};

use bomber_domain::board::TileGrid;
use bomber_domain::game::Rules;
use bomber_domain::shared::{Cell, PlayerId};

use crate::codec::{Reader, Result, Writer};
use crate::grid;

/// Everything static about a match, sent before the first tick.
///
/// Repeated on five consecutive ticks at match start: with no acknowledgement
/// channel, repetition is the only delivery guarantee available. A bot that
/// still misses it can send hello again to have it re-sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchInit {
    pub tick: u32,
    pub protocol_version: u8,
    pub match_id: u32,
    /// The seed the whole match derives from. Recording it makes the match
    /// exactly reproducible from the seed plus the input log.
    pub seed: u64,
    pub your_player_id: PlayerId,
    pub grid: TileGrid,
    /// Indexed by player id.
    pub spawns: Vec<Cell>,
    pub rules: Rules,
}

impl MatchInit {
    pub fn encode(&self, w: &mut Writer) {
        w.u8(self.protocol_version)
            .u32(self.match_id)
            .u64(self.seed)
            .u8(self.your_player_id.raw())
            .u8(self.spawns.len() as u8)
            .u8(self.grid.width())
            .u8(self.grid.height());
        grid::encode(&self.grid, w);
        for spawn in &self.spawns {
            w.u8(spawn.x).u8(spawn.y);
        }
        encode_rules(&self.rules, w);
    }

    pub fn decode(tick: u32, r: &mut Reader) -> Result<Self> {
        let protocol_version = r.u8()?;
        let match_id = r.u32()?;
        let seed = r.u64()?;
        let your_player_id = PlayerId::new(r.u8()?);
        let player_count = r.u8()? as usize;
        let width = r.u8()?;
        let height = r.u8()?;
        let grid = grid::decode(r, width, height)?;
        let spawns = r.repeat(player_count, |r| Ok(Cell::new(r.u8()?, r.u8()?)))?;
        Ok(MatchInit {
            tick,
            protocol_version,
            match_id,
            seed,
            your_player_id,
            grid,
            spawns,
            rules: decode_rules(r)?,
        })
    }
}

/// The rules block, in bytes: `u16 u16 u8 u8 u8 u8 u8 u8 u8 u32 u32`.
///
/// Bots are expected to read the block rather than hardcode the defaults --
/// that is what lets a moderator retune the server without anyone rebuilding a
/// bot. A test asserts this matches what [`encode_rules`] actually writes,
/// because a wrong number here is a wrong number in `BOT_GUIDE.md`, and a bot
/// author who trusts it mis-parses everything after the block.
pub const RULES_ENCODED_LEN: usize = 2 + 2 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 4 + 4;

pub fn encode_rules(rules: &Rules, w: &mut Writer) {
    w.u16(rules.bomb_fuse_ticks)
        .u16(rules.flame_duration_ticks)
        .u8(rules.ticks_per_cell)
        .u8(rules.speed_step_ticks)
        .u8(rules.start_bombs)
        .u8(rules.start_flame)
        .u8(rules.max_flame)
        .u8(rules.max_speed)
        .u8(rules.powerup_chance_pct)
        .u32(rules.round_time_ticks)
        .u32(rules.sudden_death_tick);
}

pub fn decode_rules(r: &mut Reader) -> Result<Rules> {
    Ok(Rules {
        bomb_fuse_ticks: r.u16()?,
        flame_duration_ticks: r.u16()?,
        ticks_per_cell: r.u8()?,
        speed_step_ticks: r.u8()?,
        start_bombs: r.u8()?,
        start_flame: r.u8()?,
        max_flame: r.u8()?,
        max_speed: r.u8()?,
        powerup_chance_pct: r.u8()?,
        round_time_ticks: r.u32()?,
        sudden_death_tick: r.u32()?,
    })
}
