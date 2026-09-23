use bomber_domain::board::Tile;
use bomber_domain::game::{Event, PowerupKind};
use serde::Serialize;

/// Discrete things that happened on a tick.
///
/// These exist because some things are invisible in a state diff -- a bomb that
/// explodes and clears within one tick -- and because animations and sounds
/// need a trigger rather than a level.
///
/// Every id field names its namespace (`player`, `bomb`, `powerup`) instead of
/// a bare `id`, so confusing a bomb with a power-up is not expressible.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventDto {
    BombPlaced {
        bomb: u16,
        player: u8,
        x: u8,
        y: u8,
    },
    Explosion {
        bomb: u16,
        x: u8,
        y: u8,
        up: u8,
        down: u8,
        left: u8,
        right: u8,
    },
    BlockDestroyed {
        x: u8,
        y: u8,
    },
    PowerupSpawned {
        powerup: u16,
        x: u8,
        y: u8,
        kind: PowerupKind,
    },
    PowerupTaken {
        powerup: u16,
        player: u8,
        kind: PowerupKind,
    },
    PowerupBurned {
        powerup: u16,
        x: u8,
        y: u8,
    },
    Death {
        player: u8,
    },
    WallClosed {
        x: u8,
        y: u8,
    },
}

impl EventDto {
    /// Not every domain event is worth animating.
    ///
    /// Position, stats, flame and timer changes are all visible in the full
    /// state that accompanies them, so sending them twice would only give the
    /// front end two sources of truth for the same fact.
    pub fn from_domain(event: &Event) -> Option<EventDto> {
        Some(match *event {
            Event::BombPlaced {
                id, owner, cell, ..
            } => EventDto::BombPlaced {
                bomb: id,
                player: owner.raw(),
                x: cell.x,
                y: cell.y,
            },
            Event::Exploded {
                bomb,
                centre,
                arms,
            } => EventDto::Explosion {
                bomb,
                x: centre.x,
                y: centre.y,
                // `arms` is indexed like `Direction::ALL`.
                up: arms[1],
                down: arms[0],
                left: arms[2],
                right: arms[3],
            },
            Event::TileChanged {
                cell,
                tile: Tile::Empty,
            } => EventDto::BlockDestroyed {
                x: cell.x,
                y: cell.y,
            },
            Event::PowerupSpawned { id, cell, kind } => EventDto::PowerupSpawned {
                powerup: id,
                x: cell.x,
                y: cell.y,
                kind,
            },
            Event::PowerupRemoved {
                id,
                cell,
                kind,
                taken_by,
            } => match taken_by {
                Some(player) => EventDto::PowerupTaken {
                    powerup: id,
                    player: player.raw(),
                    kind,
                },
                None => EventDto::PowerupBurned {
                    powerup: id,
                    x: cell.x,
                    y: cell.y,
                },
            },
            Event::PlayerDied { id, .. } => EventDto::Death { player: id.raw() },
            Event::WallClosed { cell } => EventDto::WallClosed {
                x: cell.x,
                y: cell.y,
            },
            _ => return None,
        })
    }
}
