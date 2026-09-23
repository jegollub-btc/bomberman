//! The bot-facing UDP adapter.
//!
//! Two jobs, and nothing else: turn datagrams into session calls, and turn the
//! session's frames back into datagrams. It owns the one piece of knowledge the
//! session deliberately does not have -- which socket address belongs to which
//! seat -- because that is a property of the transport, not of the game.

mod endpoint;
mod registry;

pub use endpoint::{listen, Endpoint};
pub use registry::Registry;
