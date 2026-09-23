//! Rules tests.
//!
//! Each test pins one rule that a bot author can read in `BOT_GUIDE.md`, so the
//! guide and the simulation cannot drift apart silently.

use bomber_proto::{Action, Direction, EndReason, PowerupKind, Rules, Tile, NO_PLAYER};
use bomber_sim::{map, GameState, MapConfig, Symmetry};

/// An empty arena: pillar lattice only, no soft blocks, so movement and blast
/// tests are not at the mercy of the generator.
fn arena(players: u8) -> GameState {
    arena_with(players, Rules::default())
}

fn arena_with(players: u8, rules: Rules) -> GameState {
    let cfg = MapConfig {
        width: 15,
        height: 13,
        soft_block_density: 0.0,
        symmetry: Symmetry::Quad,
    };
    GameState::new(map::generate(&cfg, 1, players), rules, 1, players)
}

fn inputs(actions: &[Option<Action>]) -> Vec<Option<Action>> {
    actions.to_vec()
}

fn idle(n: usize) -> Vec<Option<Action>> {
    vec![None; n]
}

// ---------------------------------------------------------------------------
// Determinism -- the property everything else is built on.
// ---------------------------------------------------------------------------

/// Scripted inputs derived arithmetically from the tick, so the sequence is
/// identical across runs without needing a recorded log file.
fn scripted_action(tick: u32, player: u8) -> Option<Action> {
    let n = (tick.wrapping_mul(2_654_435_761) ^ (player as u32).wrapping_mul(97)) % 16;
    match n {
        0..=2 => Some(Action::Up),
        3..=5 => Some(Action::Down),
        6..=8 => Some(Action::Left),
        9..=11 => Some(Action::Right),
        12 => Some(Action::Bomb),
        13 => Some(Action::RightBomb),
        14 => Some(Action::Noop),
        _ => None,
    }
}

fn run_scripted(seed: u64, ticks: u32) -> u64 {
    let cfg = MapConfig::default();
    let mut state = GameState::new(map::generate(&cfg, seed, 4), Rules::default(), seed, 4);
    for tick in 0..ticks {
        let acts: Vec<Option<Action>> = (0..4).map(|p| scripted_action(tick, p)).collect();
        state.step(&acts);
    }
    state.state_hash()
}

#[test]
fn same_seed_and_inputs_produce_the_same_match() {
    assert_eq!(run_scripted(0xABCD, 1200), run_scripted(0xABCD, 1200));
}

#[test]
fn a_different_seed_produces_a_different_match() {
    assert_ne!(run_scripted(0xABCD, 1200), run_scripted(0xABCE, 1200));
}

#[test]
fn replaying_from_the_same_seed_reproduces_every_intermediate_tick() {
    let cfg = MapConfig::default();
    let mut a = GameState::new(map::generate(&cfg, 7, 4), Rules::default(), 7, 4);
    let mut b = GameState::new(map::generate(&cfg, 7, 4), Rules::default(), 7, 4);
    for tick in 0..600 {
        let acts: Vec<Option<Action>> = (0..4).map(|p| scripted_action(tick, p)).collect();
        a.step(&acts);
        b.step(&acts);
        assert_eq!(a.state_hash(), b.state_hash(), "diverged at tick {tick}");
    }
}

// ---------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------

#[test]
fn a_step_takes_ticks_per_cell_and_commits_immediately() {
    let mut s = arena(1);
    let total = s.rules.ticks_per_cell_at(0);
    assert_eq!(s.players[0].x, 1);

    s.step(&inputs(&[Some(Action::Right)]));
    // The destination is occupied from the first tick of the step.
    assert_eq!((s.players[0].x, s.players[0].y), (2, 1));
    assert!(s.players[0].is_moving());

    for progress in 2..total {
        s.step(&idle(1));
        assert!(s.players[0].is_moving(), "still mid-step at progress {progress}");
    }
    s.step(&idle(1));
    assert!(!s.players[0].is_moving());
    assert_eq!((s.players[0].x, s.players[0].y), (2, 1));
}

#[test]
fn a_step_cannot_be_redirected_once_started() {
    let mut s = arena(1);
    s.step(&inputs(&[Some(Action::Right)]));
    // Spamming a different direction mid-step must not move the player.
    s.step(&inputs(&[Some(Action::Down)]));
    assert_eq!((s.players[0].x, s.players[0].y), (2, 1));
    assert_eq!(s.players[0].dir, Direction::Right);
}

#[test]
fn walking_into_a_wall_still_turns_the_player() {
    let mut s = arena(1);
    // (1,1) is the top-left spawn; up is the solid border.
    s.step(&inputs(&[Some(Action::Up)]));
    assert_eq!((s.players[0].x, s.players[0].y), (1, 1));
    assert!(!s.players[0].is_moving());
    assert_eq!(s.players[0].dir, Direction::Up);
}

#[test]
fn speed_power_ups_shorten_the_step() {
    let mut s = arena(1);
    s.players[0].speed = 2;
    let expected = s.rules.ticks_per_cell_at(2);
    assert!(expected < s.rules.ticks_per_cell_at(0));
    s.step(&inputs(&[Some(Action::Right)]));
    assert_eq!(s.players[0].move_total, expected);
}

#[test]
fn a_cell_holds_one_player_and_the_lower_id_wins_a_tie() {
    let mut s = arena(2);
    // Put both players either side of (3,1) and send them at it.
    s.players[0].x = 2;
    s.players[0].y = 1;
    s.players[1].x = 4;
    s.players[1].y = 1;

    s.step(&inputs(&[Some(Action::Right), Some(Action::Left)]));
    assert_eq!((s.players[0].x, s.players[0].y), (3, 1), "lower id claims it");
    assert_eq!((s.players[1].x, s.players[1].y), (4, 1), "higher id blocked");
}

// ---------------------------------------------------------------------------
// Bombs and blast
// ---------------------------------------------------------------------------

#[test]
fn a_bomb_detonates_exactly_on_its_fuse() {
    let rules = Rules {
        bomb_fuse_ticks: 10,
        ..Rules::default()
    };
    let mut s = arena_with(1, rules);
    s.step(&inputs(&[Some(Action::Bomb)]));
    assert_eq!(s.bombs.len(), 1);
    for _ in 0..9 {
        s.step(&idle(1));
        assert_eq!(s.bombs.len(), 1);
    }
    s.step(&idle(1));
    assert!(s.bombs.is_empty());
    assert!(s.flame_at(1, 1) > 0);
}

#[test]
fn you_can_walk_off_your_own_bomb_but_not_back_onto_it() {
    let mut s = arena(1);
    let total = s.rules.ticks_per_cell_at(0);

    // Drop and step right in one action.
    s.step(&inputs(&[Some(Action::RightBomb)]));
    assert_eq!(s.bombs.len(), 1);
    assert_eq!((s.players[0].x, s.players[0].y), (2, 1));
    for _ in 1..=total {
        s.step(&idle(1));
    }

    // Stepping back onto the bomb is refused.
    s.step(&inputs(&[Some(Action::Left)]));
    assert_eq!((s.players[0].x, s.players[0].y), (2, 1));
}

#[test]
fn a_blast_stops_at_the_first_soft_block_and_destroys_it() {
    let mut s = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            start_flame: 4,
            ..Rules::default()
        },
    );
    s.grid.set(3, 1, Tile::Soft);
    s.grid.set(4, 1, Tile::Soft);

    s.step(&inputs(&[Some(Action::Bomb)]));
    s.step(&idle(1));

    assert_eq!(s.grid.get(3, 1), Tile::Empty, "first block destroyed");
    assert_eq!(s.grid.get(4, 1), Tile::Soft, "block behind it is shielded");
    assert_eq!(s.flame_at(4, 1), 0);
}

#[test]
fn a_blast_does_not_pass_through_a_pillar() {
    let mut s = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            start_flame: 6,
            ..Rules::default()
        },
    );
    // (1,2) is open, (2,2) is a pillar, (3,2) is open beyond it.
    s.players[0].x = 1;
    s.players[0].y = 2;
    assert_eq!(s.grid.get(2, 2), Tile::Solid);

    s.step(&inputs(&[Some(Action::Bomb)]));
    s.step(&idle(1));

    assert!(s.flame_at(1, 2) > 0);
    assert_eq!(s.flame_at(3, 2), 0);
}
#[test]
fn a_chain_reaction_resolves_within_a_single_tick() {
    let mut s = arena_with(
        2,
        Rules {
            bomb_fuse_ticks: 30,
            start_flame: 2,
            ..Rules::default()
        },
    );
    // Player 1 parks a long-fused bomb two cells to the right of player 0.
    s.players[1].x = 3;
    s.players[1].y = 1;
    s.step(&inputs(&[None, Some(Action::Bomb)]));

    // Player 0 drops a short-fused bomb whose blast will reach it.
    s.rules.bomb_fuse_ticks = 1;
    s.step(&inputs(&[Some(Action::Bomb), None]));
    assert_eq!(s.bombs.len(), 2);

    // One tick later the short fuse runs out. The long-fused bomb still has ~28
    // ticks left, so if it survives this tick the chain is broken.
    s.step(&idle(2));

    assert!(
        s.bombs.is_empty(),
        "the caught bomb must detonate in the same tick, not on its own fuse"
    );
    assert!(s.flame_at(1, 1) > 0 && s.flame_at(3, 1) > 0);
    // The chained bomb's own blast reaches two cells further right.
    assert!(s.flame_at(5, 1) > 0, "the chained blast propagates too");
}

#[test]
fn a_player_standing_in_flame_dies() {
    let mut s = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            ..Rules::default()
        },
    );
    s.step(&inputs(&[Some(Action::Bomb)]));
    assert!(s.players[0].alive);
    s.step(&idle(1));
    assert!(!s.players[0].alive);
}

#[test]
fn walking_into_an_existing_flame_also_kills() {
    // Three players: the bomber dies to its own blast, and a bystander keeps the
    // match alive so the walk-in actually gets simulated.
    let mut s = arena_with(
        3,
        Rules {
            bomb_fuse_ticks: 1,
            flame_duration_ticks: 60,
            start_flame: 1,
            ..Rules::default()
        },
    );
    // Player 1 sits two cells right of the blast and walks into it afterwards.
    s.players[1].x = 3;
    s.players[1].y = 1;

    s.step(&inputs(&[Some(Action::Bomb), None, None]));
    s.step(&idle(3)); // detonates; flame covers (1,1), (2,1) and (1,2)
    assert!(s.flame_at(2, 1) > 0);
    assert!(s.players[1].alive);
    assert!(!s.finished, "a third player keeps the match running");

    s.step(&inputs(&[None, Some(Action::Left), None]));
    assert!(!s.players[1].alive, "stepped into a live flame");
}
#[test]
fn a_bomb_is_returned_to_its_owner_after_it_detonates() {
    let mut s = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 2,
            start_bombs: 1,
            ..Rules::default()
        },
    );
    s.step(&inputs(&[Some(Action::Bomb)]));
    assert_eq!(s.players[0].bombs_active, 1);
    // A second bomb is refused while the first is live.
    s.step(&inputs(&[Some(Action::Bomb)]));
    assert_eq!(s.bombs.len(), 1);
    s.step(&idle(1));
    assert_eq!(s.players[0].bombs_active, 0);
}

// ---------------------------------------------------------------------------
// Power-ups
// ---------------------------------------------------------------------------

#[test]
fn power_ups_apply_on_pickup_and_respect_their_caps() {
    let mut s = arena(1);
    s.rules.max_flame = 2;
    s.rules.max_speed = 1;
    let total = s.rules.ticks_per_cell_at(0);

    for (kind, cell) in [
        (PowerupKind::ExtraBomb, (2u8, 1u8)),
        (PowerupKind::Flame, (3, 1)),
        (PowerupKind::Speed, (4, 1)),
    ] {
        s.powerups.push(bomber_sim::Powerup {
            id: cell.0 as u16,
            x: cell.0,
            y: cell.1,
            kind,
        });
    }

    for _ in 0..3 {
        s.step(&inputs(&[Some(Action::Right)]));
        for _ in 1..=total {
            s.step(&idle(1));
        }
    }

    assert_eq!(s.players[0].bombs_max, 2);
    assert_eq!(s.players[0].flame, 2, "capped at max_flame");
    assert_eq!(s.players[0].speed, 1, "capped at max_speed");
    assert!(s.powerups.is_empty());
}

#[test]
fn a_power_up_caught_in_a_blast_is_destroyed_not_banked() {
    let mut s = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            start_flame: 2,
            ..Rules::default()
        },
    );
    s.powerups.push(bomber_sim::Powerup {
        id: 1,
        x: 3,
        y: 1,
        kind: PowerupKind::Flame,
    });
    s.step(&inputs(&[Some(Action::Bomb)]));
    s.step(&idle(1));
    assert!(s.powerups.is_empty());
}

// ---------------------------------------------------------------------------
// Match end
// ---------------------------------------------------------------------------

#[test]
fn the_match_ends_when_one_player_is_left() {
    let mut s = arena_with(
        2,
        Rules {
            bomb_fuse_ticks: 2,
            ..Rules::default()
        },
    );
    s.players[1].x = 1;
    s.players[1].y = 3;

    let mut ended = None;
    for _ in 0..10 {
        let out = s.step(&inputs(&[Some(Action::Bomb), None]));
        if let Some(o) = out.ended {
            ended = Some(o);
            break;
        }
    }
    let outcome = ended.expect("match should have ended");
    assert_eq!(outcome.reason, EndReason::LastStanding);
    assert_eq!(outcome.winner, 1);
    assert_eq!(outcome.results.iter().find(|r| r.id == 1).unwrap().placement, 1);
}

#[test]
fn a_mutual_kill_is_a_draw() {
    let mut s = arena_with(
        2,
        Rules {
            bomb_fuse_ticks: 2,
            start_flame: 1,
            ..Rules::default()
        },
    );
    s.players[0].x = 1;
    s.players[0].y = 1;
    s.players[1].x = 1;
    s.players[1].y = 2;

    let mut ended = None;
    for _ in 0..10 {
        if let Some(o) = s.step(&inputs(&[Some(Action::Bomb), None])).ended {
            ended = Some(o);
            break;
        }
    }
    let outcome = ended.expect("match should have ended");
    assert_eq!(outcome.winner, NO_PLAYER, "nobody survives, so nobody wins");
}

#[test]
fn the_match_ends_on_the_round_timer() {
    let mut s = arena_with(
        2,
        Rules {
            round_time_ticks: 30,
            sudden_death_tick: u32::MAX,
            ..Rules::default()
        },
    );
    let mut ended = None;
    for _ in 0..40 {
        if let Some(o) = s.step(&idle(2)).ended {
            ended = Some(o);
            break;
        }
    }
    let outcome = ended.expect("timer should have ended the match");
    assert_eq!(outcome.reason, EndReason::Timeout);
    assert_eq!(s.tick, 30);
}

#[test]
fn sudden_death_walls_the_board_in_and_kills_whoever_is_caught() {
    let mut s = arena_with(
        2,
        Rules {
            sudden_death_tick: 1,
            round_time_ticks: u32::MAX,
            ..Rules::default()
        },
    );
    // Player 0 sits on (1,1), the first cell the spiral walls off.
    for _ in 0..8 {
        if s.finished {
            break;
        }
        s.step(&idle(2));
    }
    assert_eq!(s.grid.get(1, 1), Tile::Solid);
    assert!(!s.players[0].alive, "walled in while standing still");
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

#[test]
fn destroying_a_block_scores() {
    let mut s = arena_with(
        1,
        Rules {
            bomb_fuse_ticks: 1,
            ..Rules::default()
        },
    );
    s.grid.set(2, 1, Tile::Soft);
    s.step(&inputs(&[Some(Action::Bomb)]));
    s.step(&idle(1));
    assert_eq!(s.players[0].score, bomber_sim::SCORE_BLOCK);
}
