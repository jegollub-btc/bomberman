use crate::board::Tile;
use crate::shared::{Cell, Direction, PlayerId};

use super::{BombId, PowerupId, PowerupKind};

/// Something the domain did during a tick.
///
/// Events are the domain's only output channel. They are deliberately not wire
/// records: the rules must not know whether they are being sent as bytes to a
/// bot, as JSON to a visualizer, or to a recording file. Adapters translate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Position, facing or liveness changed.
    PlayerStateChanged {
        id: PlayerId,
        alive: bool,
        moving: bool,
        cell: Cell,
        facing: Direction,
        move_progress: u8,
        move_total: u8,
    },
    /// Inventory or score changed.
    PlayerStatsChanged {
        id: PlayerId,
        bombs_max: u8,
        flame: u8,
        speed: u8,
        score: u16,
    },
    BombPlaced {
        id: BombId,
        owner: PlayerId,
        cell: Cell,
        fuse: u16,
    },
    BombRemoved {
        id: BombId,
    },
    /// The blast *shape*, for animation. The cells that actually kill arrive as
    /// [`Event::FlameKindled`].
    Exploded {
        centre: Cell,
        /// Arm length per direction, indexed like [`Direction::ALL`].
        arms: [u8; 4],
    },
    TileChanged {
        cell: Cell,
        tile: Tile,
    },
    PowerupSpawned {
        id: PowerupId,
        cell: Cell,
        kind: PowerupKind,
    },
    /// `taken_by` is `None` when the power-up was destroyed by fire rather than
    /// collected.
    PowerupRemoved {
        id: PowerupId,
        cell: Cell,
        kind: PowerupKind,
        taken_by: Option<PlayerId>,
    },
    PlayerDied {
        id: PlayerId,
        killer: Option<PlayerId>,
    },
    FlameKindled {
        cell: Cell,
        ticks: u8,
    },
    FlameDied {
        cell: Cell,
    },
    /// Sudden death walled off a cell.
    WallClosed {
        cell: Cell,
    },
    TimerChanged {
        ticks_remaining: u32,
    },
}
