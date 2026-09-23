//! Arena session use cases.
//!
//! This crate sits between the domain and the outside world. It decides *what*
//! happens -- who is admitted, when a match starts, which frame a bot is owed
//! on a given tick, what a moderator is allowed to do -- and it does all of it
//! synchronously, with no sockets, no clock and no runtime.
//!
//! That is the point: [`ArenaSession::tick`] is an ordinary function call, so
//! a whole match including admission, delivery policy and moderation can be
//! tested without binding a port or sleeping for 16 milliseconds.
//!
//! It depends on [`bomber_protocol`] as well as the domain, deliberately. The
//! frame taxonomy -- keyframe versus delta versus match init -- *is* the
//! delivery policy this layer owns, not merely an encoding of it. Introducing a
//! parallel vocabulary here would be a second thing to keep in sync for no
//! benefit. Actual bytes are still produced only in the protocol crate.

pub mod moderation;
pub mod ports;
pub mod session;

pub use moderation::{CommandOutcome, ModeratorCommand};
pub use session::{
    ArenaSession, MapSettings, SeatPresence, SessionConfig, SessionSnapshot, TickReport,
};
