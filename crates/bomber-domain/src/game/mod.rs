//! A match in progress.
//!
//! [`GameState`] is the aggregate root: it owns the board, the players, the
//! bombs, the fire and the clock, and it is the only thing allowed to mutate
//! them. Everything it does is driven by [`GameState::step`], which advances
//! exactly one tick.
//!
//! The domain reports what happened as [`Event`]s rather than wire records, so
//! nothing in here knows what a datagram or a WebSocket is. Mapping events onto
//! a transport is an adapter's job.

pub mod phases;

mod bomb;
mod event;
mod flames;
mod intent;
mod outcome;
mod player;
mod powerup;
mod rules;
mod scoring;
mod state;

pub use bomb::{Bomb, BombId};
pub use event::Event;
pub use flames::FlameField;
pub use intent::Intent;
pub use outcome::{EndReason, MatchOutcome, PlayerResult};
pub use player::Player;
pub use powerup::{Powerup, PowerupId, PowerupKind};
pub use rules::Rules;
pub use scoring::{SCORE_BLOCK, SCORE_KILL, SCORE_SURVIVAL};
pub use state::{GameState, TickOutcome};
