//! The server -> client downlink.
//!
//! Every frame starts with `[frame_type: u8][tick: u32 LE]`.
//!
//! The downlink is where the 2-byte uplink is paid for. Because a client can
//! never acknowledge anything, the server cannot delta against "last acked
//! tick". Instead it interleaves:
//!
//!   * `Keyframe` -- complete state, every `keyframe_interval` ticks, and
//!   * `Delta`    -- changes since an explicitly named `base_tick`.
//!
//! A client that misses a datagram simply discards deltas whose `base_tick` it
//! does not hold and waits for the next keyframe. That is the entire
//! loss-recovery story, and it is time-bounded rather than ack-bounded.

use serde::{Deserialize, Serialize};

use crate::codec::{ProtoError, Reader, Result, Writer};
use crate::types::{Direction, PowerupKind, Rules, Tile, TileGrid};

pub const PROTOCOL_VERSION: u8 = 1;

/// Conservative payload ceiling: stay well under the smallest path MTU we care
/// about so frames are never fragmented or silently dropped.
pub const MAX_DATAGRAM: usize = 1200;

pub mod frame_type {
    pub const ASSIGNED: u8 = 0x00;
    pub const LOBBY_STATUS: u8 = 0x01;
    pub const MATCH_INIT: u8 = 0x02;
    pub const KEYFRAME: u8 = 0x03;
    pub const DELTA: u8 = 0x04;
    pub const MATCH_END: u8 = 0x05;
}

/// Where the match currently sits in the lobby state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum LobbyState {
    /// Accepting hellos.
    Open = 0,
    /// Roster frozen by the moderator; no new bots admitted.
    Locked = 1,
    /// Countdown running, match about to start.
    Countdown = 2,
    Running = 3,
    MatchOver = 4,
}

impl LobbyState {
    pub fn from_code(code: u8) -> Result<Self> {
        Ok(match code {
            0 => LobbyState::Open,
            1 => LobbyState::Locked,
            2 => LobbyState::Countdown,
            3 => LobbyState::Running,
            4 => LobbyState::MatchOver,
            other => {
                return Err(ProtoError::InvalidValue {
                    field: "lobby_state",
                    value: other as u32,
                })
            }
        })
    }
}

/// Why a match ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum EndReason {
    LastStanding = 0,
    Timeout = 1,
    /// Moderator reset an in-progress match.
    Aborted = 2,
}

impl EndReason {
    pub fn from_code(code: u8) -> Result<Self> {
        Ok(match code {
            0 => EndReason::LastStanding,
            1 => EndReason::Timeout,
            2 => EndReason::Aborted,
            other => {
                return Err(ProtoError::InvalidValue {
                    field: "end_reason",
                    value: other as u32,
                })
            }
        })
    }
}

/// Sentinel for "no winner" in `MatchEnd::winner` and "nobody" in kill credit.
pub const NO_PLAYER: u8 = 0xFF;

// ---------------------------------------------------------------------------
// Entity snapshots, shared by keyframes and deltas.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub id: u8,
    pub alive: bool,
    /// True while mid-step between two cells.
    pub moving: bool,
    /// Cell the player currently occupies. While moving, this is the cell being
    /// moved *into* -- the move is committed the tick it starts.
    pub x: u8,
    pub y: u8,
    pub dir: Direction,
    /// Ticks elapsed in the current step, 0 when standing still.
    pub move_progress: u8,
    pub bombs_max: u8,
    pub flame: u8,
    pub speed: u8,
    pub score: u16,
}

impl PlayerSnapshot {
    pub const ENCODED_LEN: usize = 11;

    fn encode(&self, w: &mut Writer) {
        let flags = (self.alive as u8) | ((self.moving as u8) << 1);
        w.u8(self.id)
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

    fn decode(r: &mut Reader) -> Result<Self> {
        let id = r.u8()?;
        let flags = r.u8()?;
        Ok(PlayerSnapshot {
            id,
            alive: flags & 0b1 != 0,
            moving: flags & 0b10 != 0,
            x: r.u8()?,
            y: r.u8()?,
            dir: Direction::from_code(r.u8()?)?,
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
    pub id: u16,
    pub owner: u8,
    pub x: u8,
    pub y: u8,
    pub fuse_remaining: u16,
}

impl BombSnapshot {
    pub const ENCODED_LEN: usize = 7;

    fn encode(&self, w: &mut Writer) {
        w.u16(self.id)
            .u8(self.owner)
            .u8(self.x)
            .u8(self.y)
            .u16(self.fuse_remaining);
    }

    fn decode(r: &mut Reader) -> Result<Self> {
        Ok(BombSnapshot {
            id: r.u16()?,
            owner: r.u8()?,
            x: r.u8()?,
            y: r.u8()?,
            fuse_remaining: r.u16()?,
        })
    }
}

/// One lethal cell of an active explosion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlameCell {
    pub x: u8,
    pub y: u8,
    pub ticks_remaining: u8,
}

impl FlameCell {
    pub const ENCODED_LEN: usize = 3;

    fn encode(&self, w: &mut Writer) {
        w.u8(self.x).u8(self.y).u8(self.ticks_remaining);
    }

    fn decode(r: &mut Reader) -> Result<Self> {
        Ok(FlameCell {
            x: r.u8()?,
            y: r.u8()?,
            ticks_remaining: r.u8()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerupSnapshot {
    pub id: u16,
    pub x: u8,
    pub y: u8,
    pub kind: PowerupKind,
}

impl PowerupSnapshot {
    pub const ENCODED_LEN: usize = 5;

    fn encode(&self, w: &mut Writer) {
        w.u16(self.id).u8(self.x).u8(self.y).u8(self.kind as u8);
    }

    fn decode(r: &mut Reader) -> Result<Self> {
        Ok(PowerupSnapshot {
            id: r.u16()?,
            x: r.u8()?,
            y: r.u8()?,
            kind: PowerupKind::from_code(r.u8()?)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerResult {
    pub id: u8,
    /// 1 = winner. Tied players share a placement.
    pub placement: u8,
    pub score: u16,
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assigned {
    pub tick: u32,
    pub protocol_version: u8,
    pub player_id: u8,
    pub tick_rate: u8,
    pub max_players: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyStatus {
    pub tick: u32,
    pub state: LobbyState,
    pub players_connected: u8,
    pub max_players: u8,
    /// Bit `i` set means slot `i` is taken.
    pub slot_mask: u8,
    /// Ticks left in the start countdown; 0 outside `Countdown`.
    pub countdown_ticks: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchInit {
    pub tick: u32,
    pub protocol_version: u8,
    pub match_id: u32,
    /// The RNG seed the whole match derives from. Recording it makes any match
    /// exactly reproducible from the seed plus the input log.
    pub seed: u64,
    pub your_player_id: u8,
    pub grid: TileGrid,
    /// Indexed by player id.
    pub spawns: Vec<(u8, u8)>,
    pub rules: Rules,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keyframe {
    pub tick: u32,
    /// Full grid rather than a patch list: two bits per cell is smaller than a
    /// list of destroyed blocks once a match gets going, and it is bounded.
    pub grid: TileGrid,
    pub players: Vec<PlayerSnapshot>,
    pub bombs: Vec<BombSnapshot>,
    pub flames: Vec<FlameCell>,
    pub powerups: Vec<PowerupSnapshot>,
    pub ticks_remaining: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delta {
    pub tick: u32,
    /// The tick this delta is relative to. Discard the frame unless you hold
    /// state at exactly this tick.
    pub base_tick: u32,
    pub records: Vec<DeltaRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchEnd {
    pub tick: u32,
    pub reason: EndReason,
    /// `NO_PLAYER` on a draw.
    pub winner: u8,
    pub results: Vec<PlayerResult>,
}

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "snake_case")]
pub enum DeltaRecord {
    /// Position/facing changed.
    PlayerState {
        id: u8,
        alive: bool,
        moving: bool,
        x: u8,
        y: u8,
        dir: Direction,
        move_progress: u8,
    },
    /// Inventory or score changed.
    PlayerStats {
        id: u8,
        bombs_max: u8,
        flame: u8,
        speed: u8,
        score: u16,
    },
    BombAdd {
        id: u16,
        owner: u8,
        x: u8,
        y: u8,
        fuse: u16,
    },
    BombRemove {
        id: u16,
    },
    /// Cosmetic: the blast shape, so a client can animate it in one go. The
    /// authoritative lethal cells arrive as `FlameAdd` records.
    Explosion {
        x: u8,
        y: u8,
        up: u8,
        down: u8,
        left: u8,
        right: u8,
    },
    TileSet {
        x: u8,
        y: u8,
        tile: Tile,
    },
    PowerupAdd {
        id: u16,
        x: u8,
        y: u8,
        kind: PowerupKind,
    },
    PowerupRemove {
        id: u16,
        taken_by: u8,
    },
    PlayerDeath {
        id: u8,
        /// `NO_PLAYER` when nobody gets credit.
        killer: u8,
    },
    FlameAdd {
        x: u8,
        y: u8,
        ticks: u8,
    },
    FlameRemove {
        x: u8,
        y: u8,
    },
    Timer {
        ticks_remaining: u32,
    },
}

impl DeltaRecord {
    fn encode(&self, w: &mut Writer) {
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
                    .u8(id)
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
                    .u8(id)
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
                w.u8(rt::BOMB_ADD).u16(id).u8(owner).u8(x).u8(y).u16(fuse);
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
                w.u8(rt::TILE_SET).u8(x).u8(y).u8(tile as u8);
            }
            DeltaRecord::PowerupAdd { id, x, y, kind } => {
                w.u8(rt::POWERUP_ADD).u16(id).u8(x).u8(y).u8(kind as u8);
            }
            DeltaRecord::PowerupRemove { id, taken_by } => {
                w.u8(rt::POWERUP_REMOVE).u16(id).u8(taken_by);
            }
            DeltaRecord::PlayerDeath { id, killer } => {
                w.u8(rt::PLAYER_DEATH).u8(id).u8(killer);
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
        }
    }

    fn decode(r: &mut Reader) -> Result<Self> {
        use record_type as rt;
        let tag = r.u8()?;
        Ok(match tag {
            rt::PLAYER_STATE => {
                let id = r.u8()?;
                let flags = r.u8()?;
                DeltaRecord::PlayerState {
                    id,
                    alive: flags & 0b1 != 0,
                    moving: flags & 0b10 != 0,
                    x: r.u8()?,
                    y: r.u8()?,
                    dir: Direction::from_code(r.u8()?)?,
                    move_progress: r.u8()?,
                }
            }
            rt::PLAYER_STATS => DeltaRecord::PlayerStats {
                id: r.u8()?,
                bombs_max: r.u8()?,
                flame: r.u8()?,
                speed: r.u8()?,
                score: r.u16()?,
            },
            rt::BOMB_ADD => DeltaRecord::BombAdd {
                id: r.u16()?,
                owner: r.u8()?,
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
            rt::TILE_SET => DeltaRecord::TileSet {
                x: r.u8()?,
                y: r.u8()?,
                tile: Tile::from_code(r.u8()?)?,
            },
            rt::POWERUP_ADD => DeltaRecord::PowerupAdd {
                id: r.u16()?,
                x: r.u8()?,
                y: r.u8()?,
                kind: PowerupKind::from_code(r.u8()?)?,
            },
            rt::POWERUP_REMOVE => DeltaRecord::PowerupRemove {
                id: r.u16()?,
                taken_by: r.u8()?,
            },
            rt::PLAYER_DEATH => DeltaRecord::PlayerDeath {
                id: r.u8()?,
                killer: r.u8()?,
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
            other => return Err(ProtoError::UnknownRecord(other)),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ServerFrame {
    Assigned(Assigned),
    LobbyStatus(LobbyStatus),
    MatchInit(MatchInit),
    Keyframe(Keyframe),
    Delta(Delta),
    MatchEnd(MatchEnd),
}

impl ServerFrame {
    pub fn frame_type(&self) -> u8 {
        match self {
            ServerFrame::Assigned(_) => frame_type::ASSIGNED,
            ServerFrame::LobbyStatus(_) => frame_type::LOBBY_STATUS,
            ServerFrame::MatchInit(_) => frame_type::MATCH_INIT,
            ServerFrame::Keyframe(_) => frame_type::KEYFRAME,
            ServerFrame::Delta(_) => frame_type::DELTA,
            ServerFrame::MatchEnd(_) => frame_type::MATCH_END,
        }
    }

    pub fn tick(&self) -> u32 {
        match self {
            ServerFrame::Assigned(f) => f.tick,
            ServerFrame::LobbyStatus(f) => f.tick,
            ServerFrame::MatchInit(f) => f.tick,
            ServerFrame::Keyframe(f) => f.tick,
            ServerFrame::Delta(f) => f.tick,
            ServerFrame::MatchEnd(f) => f.tick,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.u8(self.frame_type()).u32(self.tick());
        match self {
            ServerFrame::Assigned(f) => {
                w.u8(f.protocol_version)
                    .u8(f.player_id)
                    .u8(f.tick_rate)
                    .u8(f.max_players);
            }
            ServerFrame::LobbyStatus(f) => {
                w.u8(f.state as u8)
                    .u8(f.players_connected)
                    .u8(f.max_players)
                    .u8(f.slot_mask)
                    .u16(f.countdown_ticks);
            }
            ServerFrame::MatchInit(f) => {
                w.u8(f.protocol_version)
                    .u32(f.match_id)
                    .u64(f.seed)
                    .u8(f.your_player_id)
                    .u8(f.spawns.len() as u8)
                    .u8(f.grid.width)
                    .u8(f.grid.height);
                f.grid.encode_packed(&mut w);
                for (x, y) in &f.spawns {
                    w.u8(*x).u8(*y);
                }
                f.rules.encode(&mut w);
            }
            ServerFrame::Keyframe(f) => {
                w.u8(f.grid.width).u8(f.grid.height);
                f.grid.encode_packed(&mut w);
                w.u8(f.players.len() as u8);
                for p in &f.players {
                    p.encode(&mut w);
                }
                w.u16(f.bombs.len() as u16);
                for b in &f.bombs {
                    b.encode(&mut w);
                }
                w.u16(f.flames.len() as u16);
                for c in &f.flames {
                    c.encode(&mut w);
                }
                w.u16(f.powerups.len() as u16);
                for p in &f.powerups {
                    p.encode(&mut w);
                }
                w.u32(f.ticks_remaining);
            }
            ServerFrame::Delta(f) => {
                w.u32(f.base_tick).u16(f.records.len() as u16);
                for rec in &f.records {
                    rec.encode(&mut w);
                }
            }
            ServerFrame::MatchEnd(f) => {
                w.u8(f.reason as u8)
                    .u8(f.winner)
                    .u8(f.results.len() as u8);
                for r in &f.results {
                    w.u8(r.id).u8(r.placement).u16(r.score);
                }
            }
        }
        w.finish()
    }

    pub fn decode(buf: &[u8]) -> Result<Self> {
        let mut r = Reader::new(buf);
        let ty = r.u8()?;
        let tick = r.u32()?;
        Ok(match ty {
            frame_type::ASSIGNED => ServerFrame::Assigned(Assigned {
                tick,
                protocol_version: r.u8()?,
                player_id: r.u8()?,
                tick_rate: r.u8()?,
                max_players: r.u8()?,
            }),
            frame_type::LOBBY_STATUS => ServerFrame::LobbyStatus(LobbyStatus {
                tick,
                state: LobbyState::from_code(r.u8()?)?,
                players_connected: r.u8()?,
                max_players: r.u8()?,
                slot_mask: r.u8()?,
                countdown_ticks: r.u16()?,
            }),
            frame_type::MATCH_INIT => {
                let protocol_version = r.u8()?;
                let match_id = r.u32()?;
                let seed = r.u64()?;
                let your_player_id = r.u8()?;
                let player_count = r.u8()?;
                let width = r.u8()?;
                let height = r.u8()?;
                let grid = TileGrid::decode_packed(&mut r, width, height)?;
                let mut spawns = Vec::with_capacity(player_count as usize);
                for _ in 0..player_count {
                    spawns.push((r.u8()?, r.u8()?));
                }
                ServerFrame::MatchInit(MatchInit {
                    tick,
                    protocol_version,
                    match_id,
                    seed,
                    your_player_id,
                    grid,
                    spawns,
                    rules: Rules::decode(&mut r)?,
                })
            }
            frame_type::KEYFRAME => {
                let width = r.u8()?;
                let height = r.u8()?;
                let grid = TileGrid::decode_packed(&mut r, width, height)?;
                let player_count = r.u8()?;
                let mut players = Vec::with_capacity(player_count as usize);
                for _ in 0..player_count {
                    players.push(PlayerSnapshot::decode(&mut r)?);
                }
                let bomb_count = r.u16()?;
                let mut bombs = Vec::with_capacity(bomb_count as usize);
                for _ in 0..bomb_count {
                    bombs.push(BombSnapshot::decode(&mut r)?);
                }
                let flame_count = r.u16()?;
                let mut flames = Vec::with_capacity(flame_count as usize);
                for _ in 0..flame_count {
                    flames.push(FlameCell::decode(&mut r)?);
                }
                let powerup_count = r.u16()?;
                let mut powerups = Vec::with_capacity(powerup_count as usize);
                for _ in 0..powerup_count {
                    powerups.push(PowerupSnapshot::decode(&mut r)?);
                }
                ServerFrame::Keyframe(Keyframe {
                    tick,
                    grid,
                    players,
                    bombs,
                    flames,
                    powerups,
                    ticks_remaining: r.u32()?,
                })
            }
            frame_type::DELTA => {
                let base_tick = r.u32()?;
                let count = r.u16()?;
                let mut records = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    records.push(DeltaRecord::decode(&mut r)?);
                }
                ServerFrame::Delta(Delta {
                    tick,
                    base_tick,
                    records,
                })
            }
            frame_type::MATCH_END => {
                let reason = EndReason::from_code(r.u8()?)?;
                let winner = r.u8()?;
                let count = r.u8()?;
                let mut results = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    results.push(PlayerResult {
                        id: r.u8()?,
                        placement: r.u8()?,
                        score: r.u16()?,
                    });
                }
                ServerFrame::MatchEnd(MatchEnd {
                    tick,
                    reason,
                    winner,
                    results,
                })
            }
            other => return Err(ProtoError::UnknownFrame(other)),
        })
    }
}
