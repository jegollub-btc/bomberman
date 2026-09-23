//! Phase 3: bombs are placed.
//!
//! This runs as its own phase, before anyone moves, so that "drop a bomb and
//! step away" is reliable for every player on every tick. If placement and
//! movement were interleaved per player, whether you escaped your own bomb
//! would depend on your player id.

use crate::game::{Bomb, Event, GameState, Intent};

pub fn run(game: &mut GameState, intents: &[Option<Intent>], events: &mut Vec<Event>) {
    for index in 0..game.players.len() {
        let wants_bomb = intents
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(Intent::IDLE)
            .place_bomb;
        if wants_bomb {
            place(game, index, events);
        }
    }
}

fn place(game: &mut GameState, index: usize, events: &mut Vec<Event>) {
    let player = &game.players[index];
    if !player.can_place_bomb() {
        return;
    }
    let (cell, owner, flame) = (player.cell, player.id, player.flame);
    if game.bomb_at(cell).is_some() {
        return;
    }

    let id = game.next_bomb_id();
    let fuse = game.rules.bomb_fuse_ticks;
    game.bombs.push(Bomb {
        id,
        owner,
        cell,
        fuse,
        flame,
    });
    game.players[index].bombs_active += 1;
    events.push(Event::BombPlaced {
        id,
        owner,
        cell,
        fuse,
    });
}
