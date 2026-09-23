//! Map generation.
//!
//! Boards are generated from a width/height pair rather than authored, so there
//! are no map files to keep in sync with the code. Generation is a pure
//! function of `(config, seed, players)`: the same inputs always produce a
//! byte-identical board, which is what makes a match reproducible from its
//! seed alone.
//!
//! The steps are deliberately split across modules because they answer
//! different questions:
//!
//! * [`layout`]     -- the fixed geometry nobody can change.
//! * [`blocks`]     -- the part that is random, and how it is kept fair.
//! * [`validation`] -- the invariants a match depends on.

mod blocks;
mod config;
mod layout;
mod validation;

pub use config::{GenerationConfig, Symmetry, MAX_DIMENSION, MIN_DIMENSION};
pub use validation::{validate, MapError};

use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;

use super::Board;

/// Generate a board.
///
/// The result is always playable: [`validation::validate`] holds by
/// construction, and is asserted in debug builds so a future change to any step
/// cannot quietly start shipping unfair maps.
pub fn generate(config: &GenerationConfig, seed: u64, players: u8) -> Board {
    let (width, height) = config.normalized_dimensions();

    let mut grid = layout::structural_grid(width, height);
    let spawns = layout::spawn_cells(width, height, players);
    let protected = layout::clear_safe_zones(&mut grid, &spawns);

    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    blocks::scatter(&mut grid, &protected, config, &mut rng);

    let board = Board { grid, spawns };
    debug_assert!(
        validate(&board).is_ok(),
        "generated an unplayable board: {:?}",
        validate(&board)
    );
    board
}
