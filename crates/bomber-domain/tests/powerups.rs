//! Pickups, caps, and what fire does to a dropped item.

mod common;

use bomber_domain::game::{Powerup, PowerupKind, Rules};
use bomber_domain::shared::{Cell, Direction};
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
