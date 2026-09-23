//! Fuses, blast shape, chains and who dies.

mod common;

use bomber_domain::board::Tile;
use bomber_domain::game::Rules;
use bomber_domain::shared::{Cell, Direction};
use common::*;

#[test]
fn a_bomb_detonates_exactly_on_its_fuse() {
    let mut state = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 10,
            ..Rules::default()
        },
    );
    state.step(&only(1, 0, bomb()));
    assert_eq!(state.bombs.len(), 1);

    for remaining in (1..10).rev() {
        state.step(&idle(1));
        assert_eq!(state.bombs.len(), 1, "{remaining} ticks of fuse left");
    }
    state.step(&idle(1));
    assert!(state.bombs.is_empty());
    assert!(state.flames.is_lethal(Cell::new(1, 1)));
}

#[test]
fn you_can_walk_off_your_own_bomb_but_never_back_onto_it() {
    let mut state = arena(1);
    let total = state.rules.ticks_per_cell_at(0);

    // Drop and step away in one intent.
    state.step(&only(1, 0, walk(Direction::Right).with_bomb()));
    assert_eq!(state.bombs.len(), 1);
    assert_eq!(at(&state, 0), Cell::new(2, 1));
    for _ in 1..=total {
        state.step(&idle(1));
    }

    state.step(&only(1, 0, walk(Direction::Left)));
    assert_eq!(at(&state, 0), Cell::new(2, 1), "the bomb blocks the way back");
}

#[test]
fn a_blast_stops_at_the_first_soft_block_and_destroys_it() {
    let mut state = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            start_flame: 4,
            ..Rules::default()
        },
    );
    state.board.grid.set(Cell::new(3, 1), Tile::Soft);
    state.board.grid.set(Cell::new(4, 1), Tile::Soft);

    state.step(&only(1, 0, bomb()));
    state.step(&idle(1));

    assert_eq!(state.board.grid.get(Cell::new(3, 1)), Tile::Empty);
    assert_eq!(
        state.board.grid.get(Cell::new(4, 1)),
        Tile::Soft,
        "the block behind it is shielded"
    );
    assert!(!state.flames.is_lethal(Cell::new(4, 1)));
}

#[test]
fn a_blast_does_not_pass_through_a_pillar() {
    let mut state = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            start_flame: 6,
            ..Rules::default()
        },
    );
    state.players[0].cell = Cell::new(1, 2);
    assert_eq!(state.board.grid.get(Cell::new(2, 2)), Tile::Solid);

    state.step(&only(1, 0, bomb()));
    state.step(&idle(1));

    assert!(state.flames.is_lethal(Cell::new(1, 2)));
    assert!(!state.flames.is_lethal(Cell::new(3, 2)));
}

#[test]
fn a_chain_reaction_resolves_within_a_single_tick() {
    let mut state = arena_with(
        2,
        Rules {
            bomb_fuse_ticks: 30,
            start_flame: 2,
            ..Rules::default()
        },
    );
    // Player 1 parks a long-fused bomb two cells to the right of player 0.
    state.players[1].cell = Cell::new(3, 1);
    state.step(&only(2, 1, bomb()));

    // Player 0 drops a short-fused bomb whose blast will reach it.
    state.rules.bomb_fuse_ticks = 1;
    state.step(&only(2, 0, bomb()));
    assert_eq!(state.bombs.len(), 2);

    state.step(&idle(2));

    assert!(
        state.bombs.is_empty(),
        "the caught bomb must go off now, not on its own fuse ~28 ticks later"
    );
    assert!(state.flames.is_lethal(Cell::new(1, 1)));
    assert!(state.flames.is_lethal(Cell::new(3, 1)));
    assert!(
        state.flames.is_lethal(Cell::new(5, 1)),
        "the chained bomb's own blast propagates too"
    );
}

#[test]
fn a_player_standing_in_flame_dies() {
    let mut state = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            ..Rules::default()
        },
    );
    state.step(&only(1, 0, bomb()));
    assert!(alive(&state, 0));
    state.step(&idle(1));
    assert!(!alive(&state, 0));
}

#[test]
fn walking_into_an_existing_flame_also_kills() {
    // Three players: the bomber dies to its own blast, and a bystander keeps
    // the match running so the walk-in is actually simulated.
    let mut state = arena_with(
        3,
        Rules {
            bomb_fuse_ticks: 1,
            flame_duration_ticks: 60,
            start_flame: 1,
            ..Rules::default()
        },
    );
    state.players[1].cell = Cell::new(3, 1);

    state.step(&only(3, 0, bomb()));
    state.step(&idle(3));
    assert!(state.flames.is_lethal(Cell::new(2, 1)));
    assert!(alive(&state, 1));
    assert!(!state.finished, "a third player keeps the match alive");

    state.step(&only(3, 1, walk(Direction::Left)));
    assert!(!alive(&state, 1), "stepped into a live flame");
}

#[test]
fn a_bomb_is_refunded_to_its_owner_only_once_it_detonates() {
    let mut state = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 2,
            start_bombs: 1,
            ..Rules::default()
        },
    );
    state.step(&only(1, 0, bomb()));
    assert_eq!(state.players[0].bombs_active, 1);

    state.step(&only(1, 0, bomb()));
    assert_eq!(state.bombs.len(), 1, "a second bomb is refused while one is live");

    state.step(&idle(1));
    assert_eq!(state.players[0].bombs_active, 0);
}
