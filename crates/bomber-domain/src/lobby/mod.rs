//! Who is allowed in, and whether a match may start.
//!
//! The lobby is deliberately transport-agnostic: it knows about seats and
//! player ids, not about sockets or addresses. Binding a seat to whoever is on
//! the other end of a connection is an adapter's job -- which is what lets the
//! same lobby serve UDP bots today and anything else later.

mod roster;
mod seat;
mod state;

pub use roster::{AdmissionError, Lobby, StartError};
pub use seat::Seat;
pub use state::LobbyState;
