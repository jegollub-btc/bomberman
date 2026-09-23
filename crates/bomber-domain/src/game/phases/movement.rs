//! Phase 4: players move.
//!
//! A step **commits the instant it starts**: the player's cell becomes the
//! destination immediately and cannot be changed until the step completes. This
//! is the single most important rule for a bot author to internalise, and it is
//! what makes collision resolvable in one pass -- a cell is either claimed or
//! it is not, with no half-occupied states.

use crate::board::Tile;
use crate::game::{Event, GameState, Intent};
use crate::shared::{Cell, Direction};

pub fn run(game: &mut GameState, intents: &[Option<Intent>], events: &mut Vec<Event>) {
    // Resolved in player-id order, which is what makes the lower id win a
    // contested cell.
    for index in 0..game.players.len() {
        let intent = intents
            .get(game.players[index].id.index())
            .copied()
            .flatten()
            .unwrap_or(Intent::IDLE);
        advance(game, index, intent.movement, events);
    }
}

fn advance(
    game: &mut GameState,
    index: usize,
    intent: Option<Direction>,
    events: &mut Vec<Event>,
) {
    if !game.players[index].alive {
        return;
    }

    if game.players[index].is_moving() {
        let player = &mut game.players[index];
        player.move_progress += 1;
        if player.move_progress >= player.move_total {
            player.move_progress = 0;
            player.move_total = 0;
        }
        events.push(game.players[index].state_event());
        return;
    }

    let Some(direction) = intent else { return };

    // Turning in place always succeeds, even into a wall, so a bot can aim
    // without first having to find an open cell.
    game.players[index].facing = direction;

    if let Some(target) = game.players[index].cell.neighbour(direction) {
        if can_enter(game, index, target) {
            let total = game.rules.ticks_per_cell_at(game.players[index].speed);
            let player = &mut game.players[index];
            player.cell = target;
            player.move_total = total;
            player.move_progress = 1;
        }
    }
    events.push(game.players[index].state_event());
}

fn can_enter(game: &GameState, index: usize, target: Cell) -> bool {
    if game.board.grid.get(target) != Tile::Empty {
        return false;
    }

    // Bombs block everyone. The only way to be standing on one is to have just
    // placed it, so this single check *is* the classic "walk off your own bomb,
    // never back onto it" rule -- with no extra state to track.
    if game.bomb_at(target).is_some() {
        return false;
    }

    // One player per cell.
    !game
        .players
        .iter()
        .enumerate()
        .any(|(other, p)| other != index && p.alive && p.cell == target)
}
