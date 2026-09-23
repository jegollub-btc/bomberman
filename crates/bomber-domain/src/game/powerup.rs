use serde::{Deserialize, Serialize};

use crate::shared::Cell;

pub type PowerupId = u16;

/// What a power-up grants when walked over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum PowerupKind {
    /// +1 concurrent bomb.
    ExtraBomb = 0,
    /// +1 blast radius, capped at `max_flame`.
    Flame = 1,
    /// +1 speed level: fewer ticks to cross a cell, capped at `max_speed`.
    Speed = 2,
}

impl PowerupKind {
    /// Drop order. Fixed, because the roll is seeded and observable.
    pub const ALL: [PowerupKind; 3] = [
        PowerupKind::ExtraBomb,
        PowerupKind::Flame,
        PowerupKind::Speed,
    ];

    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(PowerupKind::ExtraBomb),
            1 => Some(PowerupKind::Flame),
            2 => Some(PowerupKind::Speed),
            _ => None,
        }
    }

    pub const fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Powerup {
    pub id: PowerupId,
    pub cell: Cell,
    pub kind: PowerupKind,
}
