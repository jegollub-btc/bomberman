//! Shared fixtures for the domain tests.

#![allow(dead_code)]

use bomber_domain::board::{generate, GenerationConfig, Symmetry};
use bomber_domain::game::{GameState, Intent, Rules};
use bomber_domain::shared::{Cell, Direction, PlayerId};

/// An empty arena: pillar lattice only, no crates, so movement and blast tests
/// are not at the mercy of the generator.
pub fn arena(players: u8) -> GameState {
    arena_with(players, Rules::default())
}

pub fn arena_with(players: u8, rules: Rules) -> GameState {
    let config = GenerationConfig {
        width: 15,
        height: 13,
        soft_block_density: 0.0,
        symmetry: Symmetry::Quad,
    };
    GameState::new(generate(&config, 1, players), rules, 1, &seats(players))
}

/// Contiguous seats 0..players, the ordinary case.
pub fn seats(players: u8) -> Vec<PlayerId> {
    (0..players).map(PlayerId::new).collect()
}

pub fn idle(players: usize) -> Vec<Option<Intent>> {
    vec![None; players]
}

/// Intent slots with one player acting and everyone else silent.
pub fn only(players: usize, id: usize, intent: Intent) -> Vec<Option<Intent>> {
    let mut slots = vec![None; players];
    slots[id] = Some(intent);
    slots
}

pub fn walk(direction: Direction) -> Intent {
    Intent::moving(direction)
}

pub fn bomb() -> Intent {
    Intent::bomb()
}

pub fn at(state: &GameState, id: u8) -> Cell {
    state.player(PlayerId::new(id)).unwrap().cell
}

pub fn alive(state: &GameState, id: u8) -> bool {
    state.player(PlayerId::new(id)).unwrap().alive
}

/// Step until the predicate holds or `limit` ticks pass; returns the tick count.
pub fn step_until(
    state: &mut GameState,
    players: usize,
    limit: u32,
    mut done: impl FnMut(&GameState) -> bool,
) -> u32 {
    for elapsed in 0..limit {
        if done(state) {
            return elapsed;
        }
        state.step(&idle(players));
    }
    limit
}
