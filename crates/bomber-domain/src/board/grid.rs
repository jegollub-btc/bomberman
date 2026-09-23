use serde::{Deserialize, Serialize};

use super::Tile;
use crate::shared::Cell;

/// A rectangular grid of tiles.
///
/// Reads outside the grid answer [`Tile::Solid`] rather than panicking, so
/// blast and movement code can probe past the border without a bounds check at
/// every call site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileGrid {
    width: u8,
    height: u8,
    cells: Vec<Tile>,
}

impl TileGrid {
    pub fn filled(width: u8, height: u8, tile: Tile) -> Self {
        Self {
            width,
            height,
            cells: vec![tile; width as usize * height as usize],
        }
    }

    /// Rebuild from a flat row-major list. Returns `None` if the length does
    /// not match the dimensions.
    pub fn from_cells(width: u8, height: u8, cells: Vec<Tile>) -> Option<Self> {
        (cells.len() == width as usize * height as usize).then_some(Self {
            width,
            height,
            cells,
        })
    }

    pub fn width(&self) -> u8 {
        self.width
    }

    pub fn height(&self) -> u8 {
        self.height
    }

    /// Row-major, `y * width + x`.
    pub fn cells(&self) -> &[Tile] {
        &self.cells
    }

    pub fn contains(&self, cell: Cell) -> bool {
        cell.x < self.width && cell.y < self.height
    }

    pub fn index_of(&self, cell: Cell) -> usize {
        cell.y as usize * self.width as usize + cell.x as usize
    }

    /// Out-of-bounds reads as [`Tile::Solid`].
    pub fn get(&self, cell: Cell) -> Tile {
        if self.contains(cell) {
            self.cells[self.index_of(cell)]
        } else {
            Tile::Solid
        }
    }

    pub fn set(&mut self, cell: Cell, tile: Tile) {
        if self.contains(cell) {
            let i = self.index_of(cell);
            self.cells[i] = tile;
        }
    }

    pub fn iter_cells(&self) -> impl Iterator<Item = Cell> + '_ {
        let (w, h) = (self.width, self.height);
        (0..h).flat_map(move |y| (0..w).map(move |x| Cell::new(x, y)))
    }
}
