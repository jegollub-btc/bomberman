//! The shared kernel.
//!
//! Value objects that every bounded context needs to speak about the same
//! things. Kept deliberately small: anything that belongs to one context only
//! lives in that context instead.

mod cell;
mod direction;
mod player_id;

pub use cell::Cell;
pub use direction::Direction;
pub use player_id::PlayerId;
