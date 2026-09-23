//! The fixed geometry: border, pillars, spawns and their safe zones.
//!
//! Nothing here is random. It is the same for every match of a given size, and
//! it is what guarantees the board is connected without any maze carving.

use crate::board::{Tile, TileGrid};
use crate::shared::Cell;

/// Is this cell part of the indestructible skeleton?
pub fn is_structural(cell: Cell, width: u8, height: u8) -> bool {
    cell.x == 0
        || cell.y == 0
        || cell.x == width - 1
        || cell.y == height - 1
        || (cell.x.is_multiple_of(2) && cell.y.is_multiple_of(2))
}

/// The border ring plus the interior pillar lattice, everything else empty.
pub fn structural_grid(width: u8, height: u8) -> TileGrid {
    let mut grid = TileGrid::filled(width, height, Tile::Empty);
    for cell in grid.iter_cells().collect::<Vec<_>>() {
        if is_structural(cell, width, height) {
            grid.set(cell, Tile::Solid);
        }
    }
    grid
}

/// Spawn cells in player-id order: the four inner corners first, then edge
/// midpoints if more than four players are ever supported.
///
/// Every candidate lands on odd coordinates so it can never be a pillar.
pub fn spawn_cells(width: u8, height: u8, players: u8) -> Vec<Cell> {
    let mut cells = vec![
        Cell::new(1, 1),
        Cell::new(width - 2, 1),
        Cell::new(1, height - 2),
        Cell::new(width - 2, height - 2),
        Cell::new((width / 2) | 1, 1),
        Cell::new((width / 2) | 1, height - 2),
        Cell::new(1, (height / 2) | 1),
        Cell::new(width - 2, (height / 2) | 1),
    ];
    cells.truncate(players.max(1) as usize);
    cells
}

/// The spawn cell plus the two neighbours pointing at the board centre.
///
/// Keeping this L-shape clear is what stops a bot from spawning walled in, or
/// from being trapped by the first bomb anyone drops.
pub fn safe_zone(spawn: Cell, width: u8, height: u8) -> [Cell; 3] {
    let toward_centre = |v: u8, extent: u8| -> i32 {
        if (v as i32) < (extent as i32) / 2 {
            1
        } else {
            -1
        }
    };
    let dx = toward_centre(spawn.x, width);
    let dy = toward_centre(spawn.y, height);
    [
        spawn,
        Cell::new((spawn.x as i32 + dx) as u8, spawn.y),
        Cell::new(spawn.x, (spawn.y as i32 + dy) as u8),
    ]
}

/// Clear every spawn's safe zone and report which cells must stay clear.
///
/// The returned mask is consulted after mirroring, so symmetry can never fill a
/// safe zone back in.
pub fn clear_safe_zones(grid: &mut TileGrid, spawns: &[Cell]) -> Vec<bool> {
    let (w, h) = (grid.width(), grid.height());
    let mut protected = vec![false; w as usize * h as usize];
    for spawn in spawns {
        for cell in safe_zone(*spawn, w, h) {
            if grid.contains(cell) && !is_structural(cell, w, h) {
                protected[grid.index_of(cell)] = true;
                grid.set(cell, Tile::Empty);
            }
        }
    }
    protected
}
