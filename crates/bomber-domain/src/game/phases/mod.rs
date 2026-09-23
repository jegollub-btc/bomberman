//! The tick, one phase per module.
//!
//! **The order of these phases is the rules.** Changing it changes outcomes, so
//! it is written down here rather than being an accident of how `step` happens
//! to be laid out:
//!
//! 1. [`flame_decay`]  -- yesterday's fire ages and goes out.
//! 2. [`detonation`] -- fuses run down; detonations and their full chain
//!    resolve, destroying blocks and lighting new fire.
//! 3. [`placement`]    -- every player who asked for a bomb gets one...
//! 4. [`movement`]     -- ...and only then does anyone move.
//! 5. [`pickups`]      -- power-ups are collected.
//! 6. [`casualties`]   -- anyone standing in fire dies.
//! 7. [`sudden_death`] -- the board closes in another cell.
//! 8. [`completion`]   -- is it over?
//!
//! The consequence a bot author feels: fire that appears in phase 2 kills in
//! phase 6 of the *same* tick, even if they moved in phase 4. And because
//! placement is a separate phase from movement, "drop a bomb and step away"
//! always works.

pub mod blast;
pub mod casualties;
pub mod completion;
pub mod detonation;
pub mod flame_decay;
pub mod movement;
pub mod pickups;
pub mod placement;
pub mod sudden_death;
