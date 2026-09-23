//! Shared value types: what a tile, an action, a direction and a power-up are.
//!
//! These are the vocabulary of both the wire format and the simulation, so they
//! live here rather than in `bomber-sim` -- a bot links only against this crate.

use serde::{Deserialize, Serialize};

use crate::codec::{ProtoError, Reader, Result, Writer};

/// What occupies a cell of the board. Two bits on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Tile {
    /// Walkable.
    Empty = 0,
    /// Indestructible wall. Border ring and the interior pillar lattice.
    Solid = 1,
    /// Destructible block. Blocks movement and stops a blast; may drop a power-up.
    Soft = 2,
}

impl Tile {
    pub fn from_code(code: u8) -> Result<Self> {
        match code {
            0 => Ok(Tile::Empty),
            1 => Ok(Tile::Solid),
            2 => Ok(Tile::Soft),
            other => Err(ProtoError::InvalidValue {
                field: "tile",
                value: other as u32,
            }),
        }
    }

    /// Can a player walk into this tile? (Bombs are entities, not tiles.)
    pub fn is_walkable(self) -> bool {
        matches!(self, Tile::Empty)
    }

    /// Does a blast stop here? `Soft` stops the blast *and* is destroyed by it.
    pub fn blocks_blast(self) -> bool {
        !matches!(self, Tile::Empty)
    }
}

/// Facing. The order matches the `dirs` array in the visualizer's `assets.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Direction {
    Down = 0,
    Up = 1,
    Left = 2,
    Right = 3,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::Down,
        Direction::Up,
        Direction::Left,
        Direction::Right,
    ];

    pub fn from_code(code: u8) -> Result<Self> {
        match code {
            0 => Ok(Direction::Down),
            1 => Ok(Direction::Up),
            2 => Ok(Direction::Left),
            3 => Ok(Direction::Right),
            other => Err(ProtoError::InvalidValue {
                field: "direction",
                value: other as u32,
            }),
        }
    }

    /// Unit step in cell coordinates. Y grows downward.
    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::Down => (0, 1),
            Direction::Up => (0, -1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
}

/// What a power-up grants when walked over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum PowerupKind {
    /// +1 concurrent bomb.
    ExtraBomb = 0,
    /// +1 blast radius, capped at `max_flame`.
    Flame = 1,
    /// +1 speed level: fewer ticks to cross a cell, capped at `max_speed`.
    Speed = 2,
}

impl PowerupKind {
    pub const ALL: [PowerupKind; 3] = [
        PowerupKind::ExtraBomb,
        PowerupKind::Flame,
        PowerupKind::Speed,
    ];

    pub fn from_code(code: u8) -> Result<Self> {
        match code {
            0 => Ok(PowerupKind::ExtraBomb),
            1 => Ok(PowerupKind::Flame),
            2 => Ok(PowerupKind::Speed),
            other => Err(ProtoError::InvalidValue {
                field: "powerup_kind",
                value: other as u32,
            }),
        }
    }
}

/// The rules constants a bot needs in order to reason about timing.
///
/// Shipped verbatim in `MATCH_INIT` so a bot never has to hardcode them: the
/// moderator can retune the server and bots adapt without a rebuild.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    /// Ticks from placement to detonation.
    pub bomb_fuse_ticks: u16,
    /// Ticks a flame cell stays lethal.
    pub flame_duration_ticks: u16,
    /// Ticks to cross one cell at speed level 0.
    pub ticks_per_cell: u8,
    /// Ticks subtracted from `ticks_per_cell` per speed level.
    pub speed_step_ticks: u8,
    pub start_bombs: u8,
    pub start_flame: u8,
    pub max_flame: u8,
    pub max_speed: u8,
    /// Chance in percent that a destroyed soft block drops a power-up. The
    /// three kinds are then equally likely.
    pub powerup_chance_pct: u8,
    /// Total match length in ticks.
    pub round_time_ticks: u32,
    /// Tick at which the board starts closing in. `u32::MAX` disables it.
    pub sudden_death_tick: u32,
}

impl Rules {
    pub const ENCODED_LEN: usize = 2 + 2 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 4 + 4;

    /// Ticks to cross one cell at a given speed level, never below 1.
    pub fn ticks_per_cell_at(&self, speed: u8) -> u8 {
        let penalty = self.speed_step_ticks.saturating_mul(speed);
        self.ticks_per_cell.saturating_sub(penalty).max(1)
    }

    pub fn encode(&self, w: &mut Writer) {
        w.u16(self.bomb_fuse_ticks)
            .u16(self.flame_duration_ticks)
            .u8(self.ticks_per_cell)
            .u8(self.speed_step_ticks)
            .u8(self.start_bombs)
            .u8(self.start_flame)
            .u8(self.max_flame)
            .u8(self.max_speed)
            .u8(self.powerup_chance_pct)
            .u32(self.round_time_ticks)
            .u32(self.sudden_death_tick);
    }

    pub fn decode(r: &mut Reader) -> Result<Self> {
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
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            bomb_fuse_ticks: 120,
            flame_duration_ticks: 30,
            ticks_per_cell: 8,
            speed_step_ticks: 1,
            start_bombs: 1,
            start_flame: 1,
            max_flame: 6,
            max_speed: 3,
            powerup_chance_pct: 30,
            round_time_ticks: 180 * 60,
            sudden_death_tick: 120 * 60,
        }
    }
}

/// The board's static-ish geometry. Tiles change only when soft blocks are destroyed.
///
/// Packed two bits per cell on the wire, row-major, low bits first -- a 31x31
/// board is 241 bytes, which keeps a full-grid keyframe inside one datagram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileGrid {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<Tile>,
}

impl TileGrid {
    pub fn new(width: u8, height: u8) -> Self {
        Self {
            width,
            height,
            cells: vec![Tile::Empty; width as usize * height as usize],
        }
    }

    pub fn packed_len(width: u8, height: u8) -> usize {
        (width as usize * height as usize).div_ceil(4)
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32
    }

    fn index(&self, x: u8, y: u8) -> usize {
        y as usize * self.width as usize + x as usize
    }

    pub fn get(&self, x: u8, y: u8) -> Tile {
        self.cells[self.index(x, y)]
    }

    /// Out-of-bounds reads as `Solid`, so callers can probe past the border freely.
    pub fn get_or_solid(&self, x: i32, y: i32) -> Tile {
        if self.in_bounds(x, y) {
            self.get(x as u8, y as u8)
        } else {
            Tile::Solid
        }
    }

    pub fn set(&mut self, x: u8, y: u8, tile: Tile) {
        let i = self.index(x, y);
        self.cells[i] = tile;
    }

    pub fn encode_packed(&self, w: &mut Writer) {
        let mut packed = vec![0u8; Self::packed_len(self.width, self.height)];
        for (i, tile) in self.cells.iter().enumerate() {
            packed[i / 4] |= (*tile as u8) << ((i % 4) * 2);
        }
        w.bytes(&packed);
    }

    pub fn decode_packed(r: &mut Reader, width: u8, height: u8) -> Result<Self> {
        let count = width as usize * height as usize;
        let packed = r.bytes(Self::packed_len(width, height))?;
        let mut cells = Vec::with_capacity(count);
        for i in 0..count {
            let code = (packed[i / 4] >> ((i % 4) * 2)) & 0b11;
            cells.push(Tile::from_code(code)?);
        }
        Ok(TileGrid {
            width,
            height,
            cells,
        })
    }
}
