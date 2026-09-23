use serde::{Deserialize, Serialize};

/// Which player. A slot number, not an index into an arbitrary list.
///
/// Wrapping this rather than passing `u8` around is worth it here: player ids,
/// bomb ids, power-up ids and raw cell coordinates are all small integers, and
/// mixing them up is the easiest mistake to make in this codebase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerId(u8);

impl PlayerId {
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u8 {
        self.0
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.0)
    }
}
