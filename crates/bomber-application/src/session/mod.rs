//! The arena session: one lobby, one match at a time, and the policy that
//! decides what every connected bot is owed on every tick.

mod config;
mod delivery;
mod inputs;
mod presence;
mod arena;
mod snapshot;

pub use config::{MapSettings, SessionConfig};
pub use inputs::InputSlots;
pub use presence::{Presence, SeatPresence};
pub use arena::{ArenaSession, TickReport};
pub use snapshot::{SeatView, SessionSnapshot};
