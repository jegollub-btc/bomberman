use serde::{Deserialize, Serialize};

/// Smallest board with an interior worth playing in.
pub const MIN_DIMENSION: u8 = 7;
/// Cell coordinates are `u8` on the wire, and this is already absurd.
pub const MAX_DIMENSION: u8 = 63;

/// How the soft-block roll is mirrored across the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Symmetry {
    /// Every eligible cell rolled independently.
    None,
    /// The left half is rolled and mirrored onto the right.
    MirrorX,
    /// The top-left quadrant is rolled and mirrored onto the other three.
    #[default]
    Quad,
}

impl Symmetry {
    /// The region that actually gets rolled; the rest is mirrored from it.
    pub fn source_region(self, width: u8, height: u8) -> (u8, u8) {
        match self {
            Symmetry::None => (width, height),
            Symmetry::MirrorX => (width.div_ceil(2), height),
            Symmetry::Quad => (width.div_ceil(2), height.div_ceil(2)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub width: u8,
    pub height: u8,
    /// Fraction of eligible cells that become soft blocks.
    pub soft_block_density: f32,
    pub symmetry: Symmetry,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            width: 15,
            height: 13,
            soft_block_density: 0.75,
            symmetry: Symmetry::Quad,
        }
    }
}

impl GenerationConfig {
    /// Dimensions rounded up to odd and clamped into the playable range.
    ///
    /// Odd dimensions are not cosmetic: they are what makes the interior pillar
    /// lattice line up with the solid border on every side.
    pub fn normalized_dimensions(&self) -> (u8, u8) {
        (normalize(self.width), normalize(self.height))
    }

    pub fn density(&self) -> f64 {
        self.soft_block_density.clamp(0.0, 1.0) as f64
    }
}

fn normalize(value: u8) -> u8 {
    let value = value.clamp(MIN_DIMENSION, MAX_DIMENSION);
    if value.is_multiple_of(2) {
        (value + 1).min(MAX_DIMENSION)
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_are_forced_odd_and_clamped() {
        assert_eq!(normalize(0), 7);
        assert_eq!(normalize(4), 7);
        assert_eq!(normalize(14), 15);
        assert_eq!(normalize(15), 15);
        assert_eq!(normalize(255), 63);
    }
}
