//! The property everything else rests on.
//!
//! Same seed plus same inputs must produce the same match, byte for byte. That
//! is what makes replays possible, makes these tests meaningful, and makes it
//! possible to answer "what actually happened" after a disputed match.

mod common;

use bomber_domain::board::{generate, GenerationConfig};
use bomber_domain::game::{GameState, Intent, Rules};
use bomber_domain::shared::Direction;

/// Inputs derived arithmetically from the tick, so the sequence is identical
/// across runs without needing a recorded log on disk.
fn scripted(tick: u32, player: u8) -> Option<Intent> {
    let n = (tick.wrapping_mul(2_654_435_761) ^ (player as u32).wrapping_mul(97)) % 16;
    match n {
        0..=2 => Some(Intent::moving(Direction::Up)),
        3..=5 => Some(Intent::moving(Direction::Down)),
        6..=8 => Some(Intent::moving(Direction::Left)),
        9..=11 => Some(Intent::moving(Direction::Right)),
        12 => Some(Intent::bomb()),
        13 => Some(Intent::moving(Direction::Right).with_bomb()),
        14 => Some(Intent::IDLE),
        _ => None,
    }
}

fn fresh(seed: u64) -> GameState {
    let config = GenerationConfig::default();
    GameState::new(generate(&config, seed, 4), Rules::default(), seed, 4)
}

fn run(seed: u64, ticks: u32) -> u64 {
    let mut state = fresh(seed);
    for tick in 0..ticks {
        let intents: Vec<Option<Intent>> = (0..4).map(|p| scripted(tick, p)).collect();
        state.step(&intents);
    }
    state.state_hash()
}

#[test]
fn the_same_seed_and_inputs_produce_the_same_match() {
    assert_eq!(run(0xABCD, 1200), run(0xABCD, 1200));
}

#[test]
fn a_different_seed_produces_a_different_match() {
    assert_ne!(run(0xABCD, 1200), run(0xABCE, 1200));
}

#[test]
fn two_runs_agree_on_every_intermediate_tick_not_just_the_end() {
    let (mut a, mut b) = (fresh(7), fresh(7));
    for tick in 0..600 {
        let intents: Vec<Option<Intent>> = (0..4).map(|p| scripted(tick, p)).collect();
        a.step(&intents);
        b.step(&intents);
        assert_eq!(a.state_hash(), b.state_hash(), "diverged at tick {tick}");
    }
}

/// Silence and an explicit idle must be indistinguishable, or a bot would be
/// forced to transmit on every tick just to avoid changing the outcome.
#[test]
fn an_explicit_idle_is_the_same_as_sending_nothing() {
    let (mut silent, mut idling) = (fresh(3), fresh(3));
    for _ in 0..300 {
        silent.step(&[None, None, None, None]);
        idling.step(&[
            Some(Intent::IDLE),
            Some(Intent::IDLE),
            Some(Intent::IDLE),
            Some(Intent::IDLE),
        ]);
    }
    assert_eq!(silent.state_hash(), idling.state_hash());
}
