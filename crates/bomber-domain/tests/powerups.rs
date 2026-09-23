//! Pickups, caps, and what fire does to a dropped item.

mod common;

use bomber_domain::board::Tile;
use bomber_domain::game::{Event, Powerup, PowerupKind, Rules};
use bomber_domain::shared::{Cell, Direction, PlayerId};
use common::*;

#[test]
fn power_ups_apply_on_pickup_and_respect_their_caps() {
    let mut state = arena(1);
    state.rules.max_flame = 2;
    state.rules.max_speed = 1;
    let total = state.rules.ticks_per_cell_at(0);

    for (id, kind, x) in [
        (1u16, PowerupKind::ExtraBomb, 2u8),
        (2, PowerupKind::Flame, 3),
        (3, PowerupKind::Speed, 4),
    ] {
        state.powerups.push(Powerup {
            id,
            cell: Cell::new(x, 1),
            kind,
        });
    }

    for _ in 0..3 {
        state.step(&only(1, 0, walk(Direction::Right)));
        for _ in 1..=total {
            state.step(&idle(1));
        }
    }

    assert_eq!(state.players[0].bombs_max, 2);
    assert_eq!(state.players[0].flame, 2, "capped at max_flame");
    assert_eq!(state.players[0].speed, 1, "capped at max_speed");
    assert!(state.powerups.is_empty());
}

/// Blasting open a corridor can cost you the item you were aiming for.
#[test]
fn a_power_up_caught_in_a_blast_is_destroyed_not_banked() {
    let mut state = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            start_flame: 2,
            ..Rules::default()
        },
    );
    state.powerups.push(Powerup {
        id: 1,
        cell: Cell::new(3, 1),
        kind: PowerupKind::Flame,
    });

    state.step(&only(1, 0, bomb()));
    state.step(&idle(1));

    assert!(state.powerups.is_empty());
    assert_eq!(state.players[0].flame, 2, "not collected on the way out");
}

/// Collecting one has to be announced, not just applied: the visualizer plays a
/// pickup effect off this event, and a bot credits the item from it.
#[test]
fn collecting_a_power_up_reports_who_took_it() {
    let mut state = arena(1);
    let me = PlayerId::new(0);
    let target = Cell::new(2, 1);
    state.powerups.push(Powerup {
        id: 7,
        cell: target,
        kind: PowerupKind::ExtraBomb,
    });

    let outcome = state.step(&only(1, 0, walk(Direction::Right)));

    assert!(
        outcome.events.iter().any(|event| matches!(
            event,
            Event::PowerupRemoved { id: 7, taken_by: Some(p), .. } if *p == me
        )),
        "expected a PowerupRemoved crediting {me}, got {:?}",
        outcome.events
    );
    assert_eq!(state.players[0].bombs_max, 2);
}

/// The bug this pins: dropping the power-up before sweeping the cell destroyed
/// every power-up in the game on the tick it appeared. Nothing errored -- the
/// drop and the destruction both looked entirely normal -- and matches simply
/// had no items in them.
#[test]
fn a_power_up_survives_the_blast_that_dropped_it() {
    let mut state = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            powerup_chance_pct: 100,
            ..Rules::default()
        },
    );
    let crate_cell = Cell::new(2, 1);
    state.board.grid.set(crate_cell, Tile::Soft);

    state.step(&only(1, 0, bomb()));
    let outcome = state.step(&idle(1));

    assert_eq!(
        state.powerups.len(),
        1,
        "the crate broke, so its item is on the board: {:?}",
        outcome.events
    );
    assert_eq!(state.powerups[0].cell, crate_cell);
    assert!(
        outcome
            .events
            .iter()
            .any(|e| matches!(e, Event::PowerupSpawned { cell, .. } if *cell == crate_cell)),
        "and its arrival was announced"
    );
    assert!(
        !outcome.events.iter().any(|e| matches!(
            e,
            Event::PowerupRemoved { taken_by: None, cell, .. } if *cell == crate_cell
        )),
        "it must not be swept away by the blast that produced it"
    );
}

/// The other half of the rule, so a fix to one cannot quietly break the other.
#[test]
fn a_power_up_already_lying_there_is_destroyed_by_a_blast() {
    let mut state = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            start_flame: 2,
            powerup_chance_pct: 0,
            ..Rules::default()
        },
    );
    state.powerups.push(Powerup {
        id: 42,
        cell: Cell::new(3, 1),
        kind: PowerupKind::Flame,
    });

    state.step(&only(1, 0, bomb()));
    let outcome = state.step(&idle(1));

    assert!(state.powerups.is_empty());
    assert!(outcome.events.iter().any(|e| matches!(
        e,
        Event::PowerupRemoved { id: 42, taken_by: None, .. }
    )));
}
