//! Phase 7: the board closes in.
//!
//! From `sudden_death_tick`, one cell becomes a wall every
//! [`SUDDEN_DEATH_PERIOD`] ticks, spiralling inward from the outer ring. It
//! exists so a match between two bots that have both learned to hide still
//! terminates.

use crate::board::Tile;
use crate::game::{Event, GameState};

/// Ticks between one cell closing and the next.
pub const SUDDEN_DEATH_PERIOD: u32 = 6;

pub fn run(game: &mut GameState, events: &mut Vec<Event>) {
    if game.tick < game.rules.sudden_death_tick {
        return;
    }
    if !(game.tick - game.rules.sudden_death_tick).is_multiple_of(SUDDEN_DEATH_PERIOD) {
        return;
    }
    let Some(&cell) = game.closing_order.get(game.closed_count) else {
        return;
    };
    game.closed_count += 1;

    game.board.grid.set(cell, Tile::Solid);
    events.push(Event::TileChanged {
        cell,
        tile: Tile::Solid,
    });
    events.push(Event::WallClosed { cell });

    // Anything on the cell is gone, not displaced.
    game.bombs.retain(|b| b.cell != cell);
    game.powerups.retain(|p| p.cell != cell);

    if let Some(index) = game.players.iter().position(|p| p.alive && p.cell == cell) {
        game.kill(index, None, events);
    }
}

/// Interior cells, outermost ring first, clockwise.
pub fn closing_order(width: u8, height: u8) -> Vec<crate::shared::Cell> {
    use crate::shared::Cell;

    let (mut top, mut bottom) = (1i32, height as i32 - 2);
    let (mut left, mut right) = (1i32, width as i32 - 2);
    let mut order = Vec::new();
    while top <= bottom && left <= right {
        for x in left..=right {
            order.push(Cell::new(x as u8, top as u8));
        }
        for y in (top + 1)..=bottom {
            order.push(Cell::new(right as u8, y as u8));
        }
        if top < bottom {
            for x in (left..right).rev() {
                order.push(Cell::new(x as u8, bottom as u8));
            }
        }
        if left < right {
            for y in ((top + 1)..bottom).rev() {
                order.push(Cell::new(left as u8, y as u8));
            }
        }
        top += 1;
        bottom -= 1;
        left += 1;
        right -= 1;
    }
    order
}
