//! The per-entity records inside a keyframe.

use serde::{Deserialize, Serialize};

use bomber_domain::game::{Bomb, BombId, Player, Powerup, PowerupId, PowerupKind};
use bomber_domain::shared::{Direction, PlayerId};

use crate::codec::{ProtoError, Reader, Result, Writer};

fn direction_from_code(code: u8) -> Result<Direction> {
    match code {
        0 => Ok(Direction::Down),
        1 => Ok(Direction::Up),
        2 => Ok(Direction::Left),
        3 => Ok(Direction::Right),
        other => Err(ProtoError::invalid("direction", other)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub id: PlayerId,
    pub alive: bool,
    pub moving: bool,
    /// While moving this is the **destination** cell -- a step commits the
    /// moment it starts.
    pub x: u8,
    pub y: u8,
    pub dir: Direction,
    pub move_progress: u8,
    pub bombs_max: u8,
    pub flame: u8,
    pub speed: u8,
    pub score: u16,
}

impl PlayerSnapshot {
    pub const ENCODED_LEN: usize = 11;

    pub fn from_player(player: &Player) -> Self {
        PlayerSnapshot {
            id: player.id,
            alive: player.alive,
            moving: player.is_moving(),
            x: player.cell.x,
            y: player.cell.y,
            dir: player.facing,
            move_progress: player.move_progress,
            bombs_max: player.bombs_max,
            flame: player.flame,
            speed: player.speed,
            score: player.score,
        }
    }

    pub fn encode(&self, w: &mut Writer) {
        let flags = (self.alive as u8) | ((self.moving as u8) << 1);
        w.u8(self.id.raw())
            .u8(flags)
            .u8(self.x)
            .u8(self.y)
            .u8(self.dir as u8)
            .u8(self.move_progress)
            .u8(self.bombs_max)
            .u8(self.flame)
            .u8(self.speed)
            .u16(self.score);
    }

    pub fn decode(r: &mut Reader) -> Result<Self> {
        let id = PlayerId::new(r.u8()?);
        let flags = r.u8()?;
        Ok(PlayerSnapshot {
            id,
            alive: flags & 0b1 != 0,
            moving: flags & 0b10 != 0,
            x: r.u8()?,
            y: r.u8()?,
            dir: direction_from_code(r.u8()?)?,
            move_progress: r.u8()?,
            bombs_max: r.u8()?,
            flame: r.u8()?,
            speed: r.u8()?,
            score: r.u16()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BombSnapshot {
    pub id: BombId,
    pub owner: PlayerId,
    pub x: u8,
    pub y: u8,
    pub fuse_remaining: u16,
}

impl BombSnapshot {
    pub const ENCODED_LEN: usize = 7;

    pub fn from_bomb(bomb: &Bomb) -> Self {
        BombSnapshot {
            id: bomb.id,
            owner: bomb.owner,
            x: bomb.cell.x,
            y: bomb.cell.y,
            fuse_remaining: bomb.fuse,
        }
    }

    pub fn encode(&self, w: &mut Writer) {
        w.u16(self.id)
            .u8(self.owner.raw())
            .u8(self.x)
            .u8(self.y)
            .u16(self.fuse_remaining);
    }

    pub fn decode(r: &mut Reader) -> Result<Self> {
        Ok(BombSnapshot {
            id: r.u16()?,
            owner: PlayerId::new(r.u8()?),
            x: r.u8()?,
            y: r.u8()?,
            fuse_remaining: r.u16()?,
        })
    }
}

/// One lethal cell. Flame cells have no id: they are addressed by position,
/// because that is the only thing anyone ever asks about them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlameCell {
    pub x: u8,
    pub y: u8,
    pub ticks_remaining: u8,
}

impl FlameCell {
    pub const ENCODED_LEN: usize = 3;

    pub fn encode(&self, w: &mut Writer) {
        w.u8(self.x).u8(self.y).u8(self.ticks_remaining);
    }

    pub fn decode(r: &mut Reader) -> Result<Self> {
        Ok(FlameCell {
            x: r.u8()?,
            y: r.u8()?,
            ticks_remaining: r.u8()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerupSnapshot {
    pub id: PowerupId,
    pub x: u8,
    pub y: u8,
    pub kind: PowerupKind,
}

impl PowerupSnapshot {
    pub const ENCODED_LEN: usize = 5;

    pub fn from_powerup(powerup: &Powerup) -> Self {
        PowerupSnapshot {
            id: powerup.id,
            x: powerup.cell.x,
            y: powerup.cell.y,
            kind: powerup.kind,
        }
    }

    pub fn encode(&self, w: &mut Writer) {
        w.u16(self.id).u8(self.x).u8(self.y).u8(self.kind.code());
    }

    pub fn decode(r: &mut Reader) -> Result<Self> {
        let id = r.u16()?;
        let x = r.u8()?;
        let y = r.u8()?;
        let raw = r.u8()?;
        Ok(PowerupSnapshot {
            id,
            x,
            y,
            kind: PowerupKind::from_code(raw)
                .ok_or_else(|| ProtoError::invalid("powerup_kind", raw))?,
        })
    }
}
