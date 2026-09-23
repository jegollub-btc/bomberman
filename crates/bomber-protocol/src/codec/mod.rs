//! Little-endian cursors over a datagram.
//!
//! Every multi-byte field on the wire is little-endian, and decoding **never
//! panics on malformed input**: anything arriving on a UDP socket is
//! attacker- or bug-controlled, so a truncated frame has to be an error value
//! rather than an index out of bounds.

mod error;
mod reader;
mod writer;

pub use error::{ProtoError, Result};
pub use reader::Reader;
pub use writer::Writer;
