use std::net::SocketAddr;

use bomber_application::SessionSnapshot;
use bomber_domain::lobby::LobbyState;
use bomber_protocol::TICK_RATE;
use serde::Serialize;

use crate::udp::Registry;

#[derive(Debug, Clone, Serialize)]
pub struct MapDto {
    pub width: u8,
    pub height: u8,
    pub density: f32,
    pub symmetry: bomber_domain::board::Symmetry,
    /// A string: a 64-bit seed loses precision as a JSON number.
    pub seed: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlotDto {
    pub id: u8,
    /// Always present. Bots cannot send a name -- their uplink is two bytes --
    /// so this is a moderator-set label defaulting to `bot-<id>`.
    pub name: String,
    pub connected: bool,
    /// `None` for an empty seat.
    pub addr: Option<String>,
    pub packets_per_sec: u32,
    pub loss_pct: f32,
    pub last_seen_tick: Option<u32>,
    /// Milliseconds since this seat last sent anything.
    ///
    /// Not round-trip time: measuring that needs an echo, and a two-byte uplink
    /// has no room for a token to echo. Staleness is measurable, and it is what
    /// actually answers "is it safe to press Start".
    pub stale_ms: Option<u32>,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LobbyDto {
    pub state: LobbyState,
    pub paused: bool,
    /// The server's own answer to "would Start succeed right now", so the UI
    /// drives its button from the rule the server enforces instead of
    /// re-deriving it and drifting.
    pub can_start: bool,
    pub min_players: u8,
    pub max_players: u8,
    pub countdown_ticks: u16,
    pub tick_rate: u8,
    pub map: MapDto,
    pub slots: Vec<SlotDto>,
}

pub fn lobby_dto(snapshot: &SessionSnapshot, registry: &Registry) -> LobbyDto {
    let ticks_to_ms = |ticks: u32| ticks * 1000 / u32::from(TICK_RATE);

    LobbyDto {
        state: snapshot.state,
        paused: snapshot.paused,
        can_start: snapshot.can_start,
        min_players: snapshot.min_players,
        max_players: snapshot.max_players,
        countdown_ticks: snapshot.countdown_ticks,
        tick_rate: TICK_RATE,
        map: MapDto {
            width: snapshot.map.generation.width,
            height: snapshot.map.generation.height,
            density: snapshot.map.generation.soft_block_density,
            symmetry: snapshot.map.generation.symmetry,
            seed: snapshot.map.seed.to_string(),
        },
        slots: snapshot
            .seats
            .iter()
            .map(|seat| SlotDto {
                id: seat.id.raw(),
                name: seat.name.clone(),
                connected: seat.connected,
                addr: registry.addr_of(seat.id).as_ref().map(SocketAddr::to_string),
                packets_per_sec: seat.presence.packets_per_sec,
                loss_pct: seat.presence.loss_pct,
                last_seen_tick: seat.presence.last_seen_tick,
                stale_ms: seat.stale_ticks.map(ticks_to_ms),
                stale: seat.stale,
            })
            .collect(),
    }
}
