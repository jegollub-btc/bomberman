//! How far a bomb reaches.
//!
//! Split out from [`super::detonation`] because "what does this blast cover" is
//! a question worth answering on its own -- bots reason about it, the
//! visualizer draws it, and it is the part most worth testing in isolation.

use crate::board::{Tile, TileGrid};
use crate::shared::{Cell, Direction};

/// Arm length in each direction, indexed like [`Direction::ALL`].
///
/// A solid wall stops the blast and is unharmed. A soft block stops it *and* is
/// destroyed, so it is included in the arm -- which is why the cell behind a
/// crate is shielded.
pub fn arms(grid: &TileGrid, centre: Cell, radius: u8) -> [u8; 4] {
    let mut arms = [0u8; 4];
    for (i, dir) in Direction::ALL.iter().enumerate() {
        for distance in 1..=radius {
            let Some(cell) = centre.step(*dir, distance) else {
                break;
            };
            match grid.get(cell) {
                Tile::Solid => break,
                Tile::Soft => {
                    arms[i] = distance;
                    break;
                }
                Tile::Empty => arms[i] = distance,
            }
        }
    }
    arms
}

/// Every cell the blast covers, centre first.
pub fn covered_cells(centre: Cell, arms: [u8; 4]) -> Vec<Cell> {
    let mut cells = vec![centre];
    for (i, dir) in Direction::ALL.iter().enumerate() {
        for distance in 1..=arms[i] {
            if let Some(cell) = centre.step(*dir, distance) {
                cells.push(cell);
            }
        }
    }
    cells
}
