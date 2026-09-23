//! Phase 6: anyone standing in fire dies.
//!
//! Running this after movement is what makes "I walked into an existing flame"
//! and "a bomb went off under me" the same rule, checked once.

use crate::game::{Event, GameState};

pub fn run(game: &mut GameState, events: &mut Vec<Event>) {
    let victims: Vec<usize> = game
        .players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.alive && game.flames.is_lethal(p.cell))
        .map(|(index, _)| index)
        .collect();

    for index in victims {
        // No kill credit: once chain reactions are involved, "whose bomb was
        // that" has no honest answer, so nobody is blamed rather than blaming
        // the wrong player.
        game.kill(index, None, events);
    }
}
