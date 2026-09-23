//! The browser-facing adapter: a spectator feed and a moderation channel.
//!
//! Both are WebSockets carrying JSON. Unlike the bots' UDP, this runs over TCP,
//! so nothing here has to think about loss or reordering -- the whole
//! keyframe/delta apparatus is absent by design and every state message is
//! complete.

pub mod dto;

mod admin;
mod placeholder;
mod router;
mod spectate;

pub use router::serve;
