//! The Bomberman domain model.
//!
//! This crate is the hexagon's core: it holds the rules and nothing else. It
//! performs no I/O, opens no sockets, knows no wire format and depends on no
//! framework. Everything here is a pure function of explicit state.
//!
//! That purity is not tidiness for its own sake -- it is what makes a match
//! reproducible from a seed plus an input log, which in turn buys replays,
//! regression tests, and the ability to settle an argument about a match after
//! the fact.
//!
//! Three bounded contexts live side by side:
//!
//! * [`board`]  -- the playing field and how one is generated.
//! * [`game`]   -- a match in progress: players, bombs, fire, and the tick.
//! * [`lobby`]  -- who is allowed in, and whether a match may start.
//!
//! [`shared`] is the shared kernel: the handful of value objects all three
//! contexts speak in.

pub mod board;
pub mod game;
pub mod lobby;
pub mod shared;

pub use board::{Board, Tile, TileGrid};
pub use game::{Event, GameState, MatchOutcome, Rules, TickOutcome};
pub use lobby::{Lobby, LobbyState, Seat};
pub use shared::{Cell, Direction, PlayerId};
