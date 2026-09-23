//! Translating domain events into wire records.
//!
//! This is the seam between the hexagon and the outside world: the domain says
//! what happened in its own vocabulary, and exactly one place decides how that
//! is spelled on a socket.

use bomber_domain::game::Event;
use bomber_domain::shared::Direction;

use super::DeltaRecord;

impl From<&Event> for DeltaRecord {
    fn from(event: &Event) -> Self {
        match *event {
            Event::PlayerStateChanged {
                id,
                alive,
                moving,
                cell,
                facing,
                move_progress,
                // `move_total` is derivable from the rules and the player's
                // speed, so bots reconstruct it rather than paying for it on
                // every state record.
                move_total: _,
            } => DeltaRecord::PlayerState {
                id,
                alive,
                moving,
                x: cell.x,
                y: cell.y,
                dir: facing,
                move_progress,
            },
            Event::PlayerStatsChanged {
                id,
                bombs_max,
                flame,
                speed,
                score,
            } => DeltaRecord::PlayerStats {
                id,
                bombs_max,
                flame,
                speed,
                score,
            },
            Event::BombPlaced {
                id,
                owner,
                cell,
                fuse,
            } => DeltaRecord::BombAdd {
                id,
                owner,
                x: cell.x,
                y: cell.y,
                fuse,
            },
            Event::BombRemoved { id } => DeltaRecord::BombRemove { id },
            Event::Exploded { centre, arms, .. } => DeltaRecord::Explosion {
                x: centre.x,
                y: centre.y,
                // `arms` is indexed like `Direction::ALL`; naming the indices
                // here keeps that coupling in one visible place.
                up: arms[index_of(Direction::Up)],
                down: arms[index_of(Direction::Down)],
                left: arms[index_of(Direction::Left)],
                right: arms[index_of(Direction::Right)],
            },
            Event::TileChanged { cell, tile } => DeltaRecord::TileSet {
                x: cell.x,
                y: cell.y,
                tile,
            },
            Event::PowerupSpawned { id, cell, kind } => DeltaRecord::PowerupAdd {
                id,
                x: cell.x,
                y: cell.y,
                kind,
            },
            Event::PowerupRemoved { id, taken_by, .. } => {
                DeltaRecord::PowerupRemove { id, taken_by }
            }
            Event::PlayerDied { id, killer } => DeltaRecord::PlayerDeath { id, killer },
            Event::FlameKindled { cell, ticks } => DeltaRecord::FlameAdd {
                x: cell.x,
                y: cell.y,
                ticks,
            },
            Event::FlameDied { cell } => DeltaRecord::FlameRemove {
                x: cell.x,
                y: cell.y,
            },
            Event::WallClosed { cell } => DeltaRecord::WallClosed {
                x: cell.x,
                y: cell.y,
            },
            Event::TimerChanged { ticks_remaining } => DeltaRecord::Timer { ticks_remaining },
        }
    }
}

const fn index_of(direction: Direction) -> usize {
    direction as usize
}

/// Convert a tick's worth of events into wire records.
pub fn records_from_events(events: &[Event]) -> Vec<DeltaRecord> {
    events.iter().map(DeltaRecord::from).collect()
}
