use serde::{Deserialize, Serialize};

use super::Direction;

/// A position on the board, in cells. Never pixels.
///
/// The origin is top-left and `y` grows downward, matching both the wire
/// protocol and the way a canvas is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Cell {
    pub x: u8,
    pub y: u8,
}

impl Cell {
    pub const fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    /// The neighbouring cell in `dir`, or `None` if that would leave the `u8`
    /// grid. Callers still have to check the board's own bounds.
    pub fn neighbour(self, dir: Direction) -> Option<Cell> {
        let (dx, dy) = dir.delta();
        let x = (self.x as i32) + dx;
        let y = (self.y as i32) + dy;
        (x >= 0 && y >= 0 && x <= u8::MAX as i32 && y <= u8::MAX as i32)
            .then(|| Cell::new(x as u8, y as u8))
    }

    /// The cell `distance` steps away in `dir`, without bounds checking beyond
    /// the `u8` range.
    pub fn step(self, dir: Direction, distance: u8) -> Option<Cell> {
        let (dx, dy) = dir.delta();
        let x = (self.x as i32) + dx * distance as i32;
        let y = (self.y as i32) + dy * distance as i32;
        (x >= 0 && y >= 0 && x <= u8::MAX as i32 && y <= u8::MAX as i32)
            .then(|| Cell::new(x as u8, y as u8))
    }
}

impl From<(u8, u8)> for Cell {
    fn from((x, y): (u8, u8)) -> Self {
        Cell::new(x, y)
    }
}

impl From<Cell> for (u8, u8) {
    fn from(c: Cell) -> Self {
        (c.x, c.y)
    }
}
