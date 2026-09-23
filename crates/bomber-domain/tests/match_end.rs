//! How a match finishes, and who gets credit.

mod common;

use bomber_domain::board::Tile;
use bomber_domain::game::{EndReason, Rules, SCORE_BLOCK};
use bomber_domain::shared::{Cell, PlayerId};
use common::*;

#[test]
fn the_match_ends_when_one_player_is_left() {
    let mut state = arena_with(
        2,
        Rules {
            bomb_fuse_ticks: 2,
            ..Rules::default()
        },
    );
    state.players[1].cell = Cell::new(1, 3);

    let mut ended = None;
    for _ in 0..10 {
        if let Some(outcome) = state.step(&only(2, 0, bomb())).ended {
            ended = Some(outcome);
            break;
        }
    }

    let outcome = ended.expect("the match should have ended");
    assert_eq!(outcome.reason, EndReason::LastStanding);
    assert_eq!(outcome.winner, Some(PlayerId::new(1)));
    assert_eq!(
        outcome
            .results
            .iter()
            .find(|r| r.id == PlayerId::new(1))
            .unwrap()
            .placement,
        1
    );
}

#[test]
fn a_mutual_kill_is_a_draw() {
    let mut state = arena_with(
        2,
        Rules {
            bomb_fuse_ticks: 2,
            start_flame: 1,
            ..Rules::default()
        },
    );
    state.players[0].cell = Cell::new(1, 1);
    state.players[1].cell = Cell::new(1, 2);

    let mut ended = None;
    for _ in 0..10 {
        if let Some(outcome) = state.step(&only(2, 0, bomb())).ended {
            ended = Some(outcome);
            break;
        }
    }

    let outcome = ended.expect("the match should have ended");
    assert_eq!(outcome.winner, None, "nobody survives, so nobody wins");
}

#[test]
fn the_match_ends_on_the_round_timer() {
    let mut state = arena_with(
        2,
        Rules {
            round_time_ticks: 30,
            sudden_death_tick: u32::MAX,
            ..Rules::default()
        },
    );

    let mut ended = None;
    for _ in 0..40 {
        if let Some(outcome) = state.step(&idle(2)).ended {
            ended = Some(outcome);
            break;
        }
    }

    assert_eq!(ended.expect("timer should end it").reason, EndReason::Timeout);
    assert_eq!(state.tick, 30);
}

/// Sudden death exists so a match between two bots that have both learned to
/// hide still terminates.
#[test]
fn sudden_death_walls_the_board_in_and_kills_whoever_is_caught() {
    let mut state = arena_with(
        2,
        Rules {
            sudden_death_tick: 1,
            round_time_ticks: u32::MAX,
            ..Rules::default()
        },
    );

    for _ in 0..8 {
        if state.finished {
            break;
        }
        state.step(&idle(2));
    }

    assert_eq!(state.board.grid.get(Cell::new(1, 1)), Tile::Solid);
    assert!(!alive(&state, 0), "walled in while standing still");
}

#[test]
fn destroying_a_block_scores() {
    let mut state = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            ..Rules::default()
        },
    );
    state.board.grid.set(Cell::new(2, 1), Tile::Soft);

    state.step(&only(1, 0, bomb()));
    state.step(&idle(1));

    assert_eq!(state.players[0].score, SCORE_BLOCK);
}
