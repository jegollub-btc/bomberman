use crate::shared::{Cell, PlayerId};

pub type BombId = u16;

/// A placed bomb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bomb {
    pub id: BombId,
    pub owner: PlayerId,
    pub cell: Cell,
    pub fuse: u16,
    /// Blast radius captured when the bomb was *placed*, not when it detonates.
    ///
    /// Picking up a flame power-up therefore never strengthens a bomb already
    /// on the ground, which keeps a bot's own planning honest.
    pub flame: u8,
}

impl Bomb {
    pub fn tick_fuse(&mut self) {
        self.fuse = self.fuse.saturating_sub(1);
    }

    pub fn is_due(&self) -> bool {
        self.fuse == 0
    }
}
