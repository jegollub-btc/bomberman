use serde::{Deserialize, Serialize};

use bomber_domain::board::Tile;
use bomber_domain::game::{BombId, PowerupId, PowerupKind};
use bomber_domain::shared::{Direction, PlayerId};

use crate::codec::{ProtoError, Reader, Result, Writer};
use crate::{decode_optional_player, encode_optional_player};

/// Record tags. Stable -- adding a record means adding a tag, never renumbering.
pub mod record_type {
    pub const PLAYER_STATE: u8 = 0x01;
    pub const PLAYER_STATS: u8 = 0x02;
    pub const BOMB_ADD: u8 = 0x03;
    pub const BOMB_REMOVE: u8 = 0x04;
    pub const EXPLOSION: u8 = 0x05;
    pub const TILE_SET: u8 = 0x06;
    pub const POWERUP_ADD: u8 = 0x07;
    pub const POWERUP_REMOVE: u8 = 0x08;
    pub const PLAYER_DEATH: u8 = 0x09;
    pub const FLAME_ADD: u8 = 0x0A;
    pub const FLAME_REMOVE: u8 = 0x0B;
    pub const TIMER: u8 = 0x0C;
    pub const WALL_CLOSED: u8 = 0x0D;
}

/// One change. See `BOT_GUIDE.md` for which id namespace each field lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "snake_case")]
pub enum DeltaRecord {
    /// Position, facing or liveness changed. References a **player id**.
    PlayerState {
        id: PlayerId,
        alive: bool,
        moving: bool,
        x: u8,
        y: u8,
        dir: Direction,
        move_progress: u8,
    },
    /// Inventory or score changed. References a **player id**.
    PlayerStats {
        id: PlayerId,
        bombs_max: u8,
        flame: u8,
        speed: u8,
        score: u16,
    },
    /// A new bomb exists. `id` is a **bomb id**; `owner` is a **player id**.
    BombAdd {
        id: BombId,
        owner: PlayerId,
        x: u8,
        y: u8,
        fuse: u16,
    },
    /// That bomb is gone. Always paired with an `Explosion` at its cell.
    BombRemove { id: BombId },
    /// Cosmetic blast shape, for animation. The cells that actually kill arrive
    /// as `FlameAdd`.
    Explosion {
        x: u8,
        y: u8,
        up: u8,
        down: u8,
        left: u8,
        right: u8,
    },
    /// A cell changed: a crate was destroyed, or sudden death walled it off.
    TileSet { x: u8, y: u8, tile: Tile },
    /// A power-up dropped. `id` is a **power-up id**.
    PowerupAdd {
        id: PowerupId,
        x: u8,
        y: u8,
        kind: PowerupKind,
    },
    /// A power-up left the board. `taken_by` is `None` when fire destroyed it
    /// rather than a player collecting it.
    PowerupRemove {
        id: PowerupId,
        taken_by: Option<PlayerId>,
    },
    /// `killer` is `None`: chain reactions make kill credit genuinely ambiguous,
    /// so nobody is blamed rather than the wrong player being blamed.
    PlayerDeath {
        id: PlayerId,
        killer: Option<PlayerId>,
    },
    /// This cell is now lethal for `ticks`. Flames are addressed by position,
    /// not by id.
    FlameAdd { x: u8, y: u8, ticks: u8 },
    FlameRemove { x: u8, y: u8 },
    Timer { ticks_remaining: u32 },
    /// Sudden death closed this cell.
    WallClosed { x: u8, y: u8 },
}

impl DeltaRecord {
    pub fn encode(&self, w: &mut Writer) {
        use record_type as rt;
        match *self {
            DeltaRecord::PlayerState {
                id,
                alive,
                moving,
                x,
                y,
                dir,
                move_progress,
            } => {
                let flags = (alive as u8) | ((moving as u8) << 1);
                w.u8(rt::PLAYER_STATE)
                    .u8(id.raw())
                    .u8(flags)
                    .u8(x)
                    .u8(y)
                    .u8(dir as u8)
                    .u8(move_progress);
            }
            DeltaRecord::PlayerStats {
                id,
                bombs_max,
                flame,
                speed,
                score,
            } => {
                w.u8(rt::PLAYER_STATS)
                    .u8(id.raw())
                    .u8(bombs_max)
                    .u8(flame)
                    .u8(speed)
                    .u16(score);
            }
            DeltaRecord::BombAdd {
                id,
                owner,
                x,
                y,
                fuse,
            } => {
                w.u8(rt::BOMB_ADD)
                    .u16(id)
                    .u8(owner.raw())
                    .u8(x)
                    .u8(y)
                    .u16(fuse);
            }
            DeltaRecord::BombRemove { id } => {
                w.u8(rt::BOMB_REMOVE).u16(id);
            }
            DeltaRecord::Explosion {
                x,
                y,
                up,
                down,
                left,
                right,
            } => {
                w.u8(rt::EXPLOSION)
                    .u8(x)
                    .u8(y)
                    .u8(up)
                    .u8(down)
                    .u8(left)
                    .u8(right);
            }
            DeltaRecord::TileSet { x, y, tile } => {
                w.u8(rt::TILE_SET).u8(x).u8(y).u8(tile.code());
            }
            DeltaRecord::PowerupAdd { id, x, y, kind } => {
                w.u8(rt::POWERUP_ADD).u16(id).u8(x).u8(y).u8(kind.code());
            }
            DeltaRecord::PowerupRemove { id, taken_by } => {
                w.u8(rt::POWERUP_REMOVE)
                    .u16(id)
                    .u8(encode_optional_player(taken_by));
            }
            DeltaRecord::PlayerDeath { id, killer } => {
                w.u8(rt::PLAYER_DEATH)
                    .u8(id.raw())
                    .u8(encode_optional_player(killer));
            }
            DeltaRecord::FlameAdd { x, y, ticks } => {
                w.u8(rt::FLAME_ADD).u8(x).u8(y).u8(ticks);
            }
            DeltaRecord::FlameRemove { x, y } => {
                w.u8(rt::FLAME_REMOVE).u8(x).u8(y);
            }
            DeltaRecord::Timer { ticks_remaining } => {
                w.u8(rt::TIMER).u32(ticks_remaining);
            }
            DeltaRecord::WallClosed { x, y } => {
                w.u8(rt::WALL_CLOSED).u8(x).u8(y);
            }
        }
    }

    pub fn decode(r: &mut Reader) -> Result<Self> {
        use record_type as rt;
        let tag = r.u8()?;
        Ok(match tag {
            rt::PLAYER_STATE => {
                let id = PlayerId::new(r.u8()?);
                let flags = r.u8()?;
                DeltaRecord::PlayerState {
                    id,
                    alive: flags & 0b1 != 0,
                    moving: flags & 0b10 != 0,
                    x: r.u8()?,
                    y: r.u8()?,
                    dir: decode_direction(r.u8()?)?,
                    move_progress: r.u8()?,
                }
            }
            rt::PLAYER_STATS => DeltaRecord::PlayerStats {
                id: PlayerId::new(r.u8()?),
                bombs_max: r.u8()?,
                flame: r.u8()?,
                speed: r.u8()?,
                score: r.u16()?,
            },
            rt::BOMB_ADD => DeltaRecord::BombAdd {
                id: r.u16()?,
                owner: PlayerId::new(r.u8()?),
                x: r.u8()?,
                y: r.u8()?,
                fuse: r.u16()?,
            },
            rt::BOMB_REMOVE => DeltaRecord::BombRemove { id: r.u16()? },
            rt::EXPLOSION => DeltaRecord::Explosion {
                x: r.u8()?,
                y: r.u8()?,
                up: r.u8()?,
                down: r.u8()?,
                left: r.u8()?,
                right: r.u8()?,
            },
            rt::TILE_SET => {
                let x = r.u8()?;
                let y = r.u8()?;
                let raw = r.u8()?;
                DeltaRecord::TileSet {
                    x,
                    y,
                    tile: Tile::from_code(raw).ok_or_else(|| ProtoError::invalid("tile", raw))?,
                }
            }
            rt::POWERUP_ADD => {
                let id = r.u16()?;
                let x = r.u8()?;
                let y = r.u8()?;
                let raw = r.u8()?;
                DeltaRecord::PowerupAdd {
                    id,
                    x,
                    y,
                    kind: PowerupKind::from_code(raw)
                        .ok_or_else(|| ProtoError::invalid("powerup_kind", raw))?,
                }
            }
            rt::POWERUP_REMOVE => DeltaRecord::PowerupRemove {
                id: r.u16()?,
                taken_by: decode_optional_player(r.u8()?),
            },
            rt::PLAYER_DEATH => DeltaRecord::PlayerDeath {
                id: PlayerId::new(r.u8()?),
                killer: decode_optional_player(r.u8()?),
            },
            rt::FLAME_ADD => DeltaRecord::FlameAdd {
                x: r.u8()?,
                y: r.u8()?,
                ticks: r.u8()?,
            },
            rt::FLAME_REMOVE => DeltaRecord::FlameRemove {
                x: r.u8()?,
                y: r.u8()?,
            },
            rt::TIMER => DeltaRecord::Timer {
                ticks_remaining: r.u32()?,
            },
            rt::WALL_CLOSED => DeltaRecord::WallClosed {
                x: r.u8()?,
                y: r.u8()?,
            },
            other => return Err(ProtoError::UnknownRecord(other)),
        })
    }
}

fn decode_direction(code: u8) -> Result<Direction> {
    match code {
        0 => Ok(Direction::Down),
        1 => Ok(Direction::Up),
        2 => Ok(Direction::Left),
        3 => Ok(Direction::Right),
        other => Err(ProtoError::invalid("direction", other)),
    }
}
