use serde::{Deserialize, Serialize};

/// What occupies a cell.
///
/// Two bits on the wire, so the discriminants are load-bearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Tile {
    /// Walkable.
    Empty = 0,
    /// Indestructible: the border ring and the interior pillar lattice.
    Solid = 1,
    /// Destructible. Blocks movement, stops a blast, and may drop a power-up.
    Soft = 2,
}

impl Tile {
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Tile::Empty),
            1 => Some(Tile::Solid),
            2 => Some(Tile::Soft),
            _ => None,
        }
    }

    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Can a player walk in? Bombs are entities rather than tiles, so they are
    /// checked separately.
    pub const fn is_walkable(self) -> bool {
        matches!(self, Tile::Empty)
    }

    /// Does a blast stop here? `Soft` stops it *and* is destroyed by it, which
    /// is why this is not the same question as [`Tile::is_walkable`].
    pub const fn blocks_blast(self) -> bool {
        !matches!(self, Tile::Empty)
    }
}
