//! Scattering the destructible blocks -- the only random part of a board.

use rand::Rng;

use super::config::{GenerationConfig, Symmetry};
use super::layout::is_structural;
use crate::board::{Tile, TileGrid};
use crate::shared::Cell;

/// Fill eligible cells with soft blocks, honouring the configured symmetry.
///
/// The roll happens once per symmetry class and is then painted onto every
/// mirrored twin. Without that, one player can simply be dealt a more open
/// corner and the match stops being a comparison of bots -- which is the whole
/// point of an arena.
pub fn scatter(
    grid: &mut TileGrid,
    protected: &[bool],
    config: &GenerationConfig,
    rng: &mut impl Rng,
) {
    let (w, h) = (grid.width(), grid.height());
    let (source_w, source_h) = config.symmetry.source_region(w, h);
    let density = config.density();

    for y in 0..source_h {
        for x in 0..source_w {
            if !rng.gen_bool(density) {
                continue;
            }
            for cell in mirrors(Cell::new(x, y), w, h, config.symmetry) {
                if !is_structural(cell, w, h) && !protected[grid.index_of(cell)] {
                    grid.set(cell, Tile::Soft);
                }
            }
        }
    }
}

/// Every cell that must share this cell's roll.
fn mirrors(cell: Cell, width: u8, height: u8, symmetry: Symmetry) -> Vec<Cell> {
    let mx = width - 1 - cell.x;
    let my = height - 1 - cell.y;
    let mut out = match symmetry {
        Symmetry::None => vec![cell],
        Symmetry::MirrorX => vec![cell, Cell::new(mx, cell.y)],
        Symmetry::Quad => vec![
            cell,
            Cell::new(mx, cell.y),
            Cell::new(cell.x, my),
            Cell::new(mx, my),
        ],
    };
    out.sort_unstable();
    out.dedup();
    out
}
