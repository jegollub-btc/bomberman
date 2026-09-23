//! The playing field.
//!
//! A board is the static geometry of a match plus the destructible blocks laid
//! over it. It changes only when a blast destroys a soft block or sudden death
//! walls a cell off, so everything else in the domain can treat it as stable.

pub mod generation;

mod grid;
mod tile;

pub use generation::{generate, validate, GenerationConfig, MapError, Symmetry};
pub use grid::TileGrid;
pub use tile::Tile;

use crate::shared::Cell;

/// A generated board: the grid plus where each player starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pub grid: TileGrid,
    /// Indexed by player id.
    pub spawns: Vec<Cell>,
}

impl Board {
    pub fn width(&self) -> u8 {
        self.grid.width()
    }

    pub fn height(&self) -> u8 {
        self.grid.height()
    }

    pub fn spawn_of(&self, player: crate::shared::PlayerId) -> Option<Cell> {
        self.spawns.get(player.index()).copied()
    }
}
