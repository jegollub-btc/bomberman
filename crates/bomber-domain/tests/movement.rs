//! How a player gets from one cell to the next.

mod common;

use bomber_domain::shared::{Cell, Direction, PlayerId};
use common::*;

#[test]
fn a_step_takes_ticks_per_cell_and_commits_immediately() {
    let mut state = arena(1);
    let total = state.rules.ticks_per_cell_at(0);
    assert_eq!(at(&state, 0), Cell::new(1, 1));

    state.step(&only(1, 0, walk(Direction::Right)));
    // The destination belongs to the player from the very first tick.
    assert_eq!(at(&state, 0), Cell::new(2, 1));
    assert!(state.player(PlayerId::new(0)).unwrap().is_moving());

    for progress in 2..total {
        state.step(&idle(1));
        assert!(
            state.player(PlayerId::new(0)).unwrap().is_moving(),
            "still mid-step at progress {progress}"
        );
    }
    state.step(&idle(1));
    assert!(!state.player(PlayerId::new(0)).unwrap().is_moving());
    assert_eq!(at(&state, 0), Cell::new(2, 1));
}

#[test]
fn a_step_cannot_be_redirected_once_started() {
    let mut state = arena(1);
    state.step(&only(1, 0, walk(Direction::Right)));
    state.step(&only(1, 0, walk(Direction::Down)));
    assert_eq!(at(&state, 0), Cell::new(2, 1));
    assert_eq!(state.player(PlayerId::new(0)).unwrap().facing, Direction::Right);
}

#[test]
fn walking_into_a_wall_still_turns_the_player() {
    let mut state = arena(1);
    // (1,1) is the top-left spawn; up is the solid border.
    state.step(&only(1, 0, walk(Direction::Up)));
    assert_eq!(at(&state, 0), Cell::new(1, 1));
    assert!(!state.player(PlayerId::new(0)).unwrap().is_moving());
    assert_eq!(state.player(PlayerId::new(0)).unwrap().facing, Direction::Up);
}

#[test]
fn speed_power_ups_shorten_the_step() {
    let mut state = arena(1);
    state.players[0].speed = 2;
    let expected = state.rules.ticks_per_cell_at(2);
    assert!(expected < state.rules.ticks_per_cell_at(0));

    state.step(&only(1, 0, walk(Direction::Right)));
    assert_eq!(state.players[0].move_total, expected);
}

#[test]
fn a_cell_holds_one_player_and_the_lower_id_wins_a_tie() {
    let mut state = arena(2);
    state.players[0].cell = Cell::new(2, 1);
    state.players[1].cell = Cell::new(4, 1);

    state.step(&[
        Some(walk(Direction::Right)),
        Some(walk(Direction::Left)),
    ]);

    assert_eq!(at(&state, 0), Cell::new(3, 1), "lower id claims the cell");
    assert_eq!(at(&state, 1), Cell::new(4, 1), "higher id is blocked");
}

#[test]
fn a_moving_player_reports_the_cell_it_came_from() {
    let mut state = arena(1);
    state.step(&only(1, 0, walk(Direction::Right)));
    let player = state.player(PlayerId::new(0)).unwrap();
    assert_eq!(player.origin_cell(), Some(Cell::new(1, 1)));
}
