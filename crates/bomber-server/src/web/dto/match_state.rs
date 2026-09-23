use bomber_application::{ArenaSession, SessionSnapshot};
use bomber_domain::board::{Board, Tile};
use bomber_domain::game::{Event, GameState, MatchOutcome, PowerupKind, Rules};
use bomber_domain::shared::Direction;
use bomber_protocol::TICK_RATE;
use serde::Serialize;

use super::EventDto;

#[derive(Debug, Clone, Serialize)]
pub struct MatchInitDto {
    pub match_id: u32,
    /// A string: a 64-bit seed loses precision as a JSON number.
    pub seed: String,
    pub tick_rate: u8,
    pub width: u8,
    pub height: u8,
    /// Flat, row-major, `y * width + x`. `0` empty, `1` solid, `2` soft.
    pub tiles: Vec<u8>,
    pub spawns: Vec<[u8; 2]>,
    pub players: Vec<MatchPlayerDto>,
    pub rules: Rules,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchPlayerDto {
    pub id: u8,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerStateDto {
    pub id: u8,
    pub alive: bool,
    pub moving: bool,
    /// While moving this is the **destination** cell: a step commits the moment
    /// it starts. Interpolation walks backwards from here.
    pub x: u8,
    pub y: u8,
    pub dir: Direction,
    pub move_progress: u8,
    /// Sent so the front end can interpolate without re-deriving it from the
    /// speed level and the rules.
    pub move_total: u8,
    pub bombs_max: u8,
    pub flame: u8,
    pub speed: u8,
    pub score: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct BombDto {
    pub id: u16,
    pub owner: u8,
    pub x: u8,
    pub y: u8,
    pub fuse: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlameDto {
    pub x: u8,
    pub y: u8,
    pub ticks: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct PowerupDto {
    pub id: u16,
    pub x: u8,
    pub y: u8,
    pub kind: PowerupKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct TileChangeDto {
    pub x: u8,
    pub y: u8,
    pub tile: Tile,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateDto {
    pub tick: u32,
    pub ticks_remaining: u32,
    /// Complete every tick: replace wholesale rather than merging.
    pub players: Vec<PlayerStateDto>,
    pub bombs: Vec<BombDto>,
    pub flames: Vec<FlameDto>,
    pub powerups: Vec<PowerupDto>,
    /// The grid is not resent; apply these to the one from `match_init`.
    pub tile_changes: Vec<TileChangeDto>,
    pub events: Vec<EventDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchResultDto {
    pub id: u8,
    pub placement: u8,
    pub score: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchEndDto {
    pub tick: u32,
    pub reason: bomber_domain::game::EndReason,
    /// `None` on a draw.
    pub winner: Option<u8>,
    pub results: Vec<MatchResultDto>,
}

pub fn board_dto(
    board: &Board,
    match_id: u32,
    seed: u64,
    rules: Rules,
    players: Vec<MatchPlayerDto>,
) -> MatchInitDto {
    MatchInitDto {
        match_id,
        seed: seed.to_string(),
        tick_rate: TICK_RATE,
        width: board.width(),
        height: board.height(),
        tiles: board.grid.cells().iter().map(|t| t.code()).collect(),
        spawns: board.spawns.iter().map(|c| [c.x, c.y]).collect(),
        players,
        rules,
    }
}

pub fn match_init_dto(session: &ArenaSession, snapshot: &SessionSnapshot) -> Option<MatchInitDto> {
    let board = session.board()?;
    let game = session.game()?;
    let players = game
        .players
        .iter()
        .map(|player| MatchPlayerDto {
            id: player.id.raw(),
            name: snapshot
                .seats
                .iter()
                .find(|seat| seat.id == player.id)
                .map(|seat| seat.name.clone())
                .unwrap_or_else(|| format!("bot-{}", player.id.raw())),
        })
        .collect();

    Some(board_dto(
        board,
        session.match_id(),
        session.map_settings().seed,
        session.config().rules,
        players,
    ))
}

pub fn state_dto(game: &GameState, events: &[Event]) -> StateDto {
    StateDto {
        tick: game.tick,
        ticks_remaining: game.ticks_remaining(),
        players: game
            .players
            .iter()
            .map(|player| PlayerStateDto {
                id: player.id.raw(),
                alive: player.alive,
                moving: player.is_moving(),
                x: player.cell.x,
                y: player.cell.y,
                dir: player.facing,
                move_progress: player.move_progress,
                move_total: player.move_total,
                bombs_max: player.bombs_max,
                flame: player.flame,
                speed: player.speed,
                score: player.score,
            })
            .collect(),
        bombs: game
            .bombs
            .iter()
            .map(|bomb| BombDto {
                id: bomb.id,
                owner: bomb.owner.raw(),
                x: bomb.cell.x,
                y: bomb.cell.y,
                fuse: bomb.fuse,
            })
            .collect(),
        flames: game
            .flames
            .burning()
            .map(|(cell, ticks)| FlameDto {
                x: cell.x,
                y: cell.y,
                ticks,
            })
            .collect(),
        powerups: game
            .powerups
            .iter()
            .map(|powerup| PowerupDto {
                id: powerup.id,
                x: powerup.cell.x,
                y: powerup.cell.y,
                kind: powerup.kind,
            })
            .collect(),
        tile_changes: events
            .iter()
            .filter_map(|event| match event {
                Event::TileChanged { cell, tile } => Some(TileChangeDto {
                    x: cell.x,
                    y: cell.y,
                    tile: *tile,
                }),
                _ => None,
            })
            .collect(),
        events: events.iter().filter_map(EventDto::from_domain).collect(),
    }
}

pub fn match_end_dto(tick: u32, outcome: &MatchOutcome) -> MatchEndDto {
    MatchEndDto {
        tick,
        reason: outcome.reason,
        winner: outcome.winner.map(|id| id.raw()),
        results: outcome
            .results
            .iter()
            .map(|result| MatchResultDto {
                id: result.id.raw(),
                placement: result.placement,
                score: result.score,
            })
            .collect(),
    }
}
