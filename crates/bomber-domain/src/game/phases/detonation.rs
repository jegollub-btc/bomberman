//! Phase 2: fuses run down, and everything they set off resolves.
//!
//! Chain reactions complete **inside this one tick**. A bomb caught by another
//! bomb's fire is appended to the same work queue, so a ten-bomb chain happens
//! at once rather than stuttering across ten ticks. That is the behaviour
//! players expect, and it is also the only version that is unambiguous to
//! simulate deterministically.

use std::collections::VecDeque;

use rand::Rng;

use crate::board::Tile;
use crate::game::{scoring, Bomb, BombId, Event, GameState, Powerup, PowerupKind};
use crate::shared::{Cell, PlayerId};

use super::blast;

pub fn run(game: &mut GameState, events: &mut Vec<Event>) {
    for bomb in &mut game.bombs {
        bomb.tick_fuse();
    }

    let mut pending: VecDeque<BombId> = game
        .bombs
        .iter()
        .filter(|b| b.is_due())
        .map(|b| b.id)
        .collect();
    if pending.is_empty() {
        return;
    }

    let mut detonated: Vec<BombId> = Vec::new();
    while let Some(id) = pending.pop_front() {
        if detonated.contains(&id) {
            continue;
        }
        let Some(bomb) = game.bombs.iter().find(|b| b.id == id).cloned() else {
            continue;
        };
        detonated.push(id);
        detonate(game, &bomb, events, &mut pending);
    }

    game.bombs.retain(|b| !detonated.contains(&b.id));
}

fn detonate(
    game: &mut GameState,
    bomb: &Bomb,
    events: &mut Vec<Event>,
    pending: &mut VecDeque<BombId>,
) {
    let arms = blast::arms(&game.board.grid, bomb.cell, bomb.flame);
    events.push(Event::Exploded {
        bomb: bomb.id,
        centre: bomb.cell,
        arms,
    });
    events.push(Event::BombRemoved { id: bomb.id });

    for cell in blast::covered_cells(bomb.cell, arms) {
        ignite(game, cell, bomb.owner, events, pending);
    }

    if let Some(owner) = game.player_mut(bomb.owner) {
        owner.bombs_active = owner.bombs_active.saturating_sub(1);
    }
}

/// Set one cell alight: destroy what is there, then chain into any bomb on it.
fn ignite(
    game: &mut GameState,
    cell: Cell,
    owner: PlayerId,
    events: &mut Vec<Event>,
    pending: &mut VecDeque<BombId>,
) {
    if game.board.grid.get(cell) == Tile::Soft {
        game.board.grid.set(cell, Tile::Empty);
        events.push(Event::TileChanged {
            cell,
            tile: Tile::Empty,
        });
        game.award(owner, scoring::SCORE_BLOCK, events);
        maybe_drop_powerup(game, cell, events);
    }

    // Fire destroys a power-up rather than banking it, so blasting open a
    // corridor can cost you the item you were aiming for.
    if let Some(index) = game.powerups.iter().position(|p| p.cell == cell) {
        let removed = game.powerups.remove(index);
        events.push(Event::PowerupRemoved {
            id: removed.id,
            cell: removed.cell,
            kind: removed.kind,
            taken_by: None,
        });
    }

    let ticks = game.rules.flame_duration_ticks.min(u8::MAX as u16) as u8;
    game.flames.kindle(cell, ticks);
    events.push(Event::FlameKindled { cell, ticks });

    if let Some(caught) = game.bombs.iter().find(|b| b.cell == cell) {
        pending.push_back(caught.id);
    }
}

fn maybe_drop_powerup(game: &mut GameState, cell: Cell, events: &mut Vec<Event>) {
    let chance = game.rules.powerup_chance_pct.min(100) as f64 / 100.0;
    if !game.rng.gen_bool(chance) {
        return;
    }
    let kind: PowerupKind = PowerupKind::ALL[game.rng.gen_range(0..PowerupKind::ALL.len())];
    let id = game.next_powerup_id();
    game.powerups.push(Powerup { id, cell, kind });
    events.push(Event::PowerupSpawned { id, cell, kind });
}
