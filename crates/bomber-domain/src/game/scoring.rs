//! What a match is worth.
//!
//! Kept in one place so the numbers can be argued about without reading the
//! tick phases.

/// Blowing up a soft block.
pub const SCORE_BLOCK: u16 = 10;
/// Killing another player.
pub const SCORE_KILL: u16 = 100;
/// Being alive when the match ends.
pub const SCORE_SURVIVAL: u16 = 500;
