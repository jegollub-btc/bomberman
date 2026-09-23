//! Server configuration.
//!
//! Everything tunable lives in `config/server.toml` so a moderator can retune a
//! tournament without a rebuild -- and because the rules constants are shipped
//! to every bot at match start, the bots follow along without one either.

mod load;
mod model;


pub use load::{load, ServerConfig};

