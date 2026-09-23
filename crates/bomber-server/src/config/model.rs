//! The shape of `server.toml`.
//!
//! Every field has a default, so a missing or partial file is a valid one. The
//! shipped file is documentation of the defaults rather than a requirement.

use bomber_domain::board::Symmetry;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ConfigFile {
    pub server: ServerSection,
    pub lobby: LobbySection,
    pub rules: RulesSection,
    pub map: MapSection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ServerSection {
    /// Where bots send their two bytes.
    pub bind: String,
    /// Where the visualizer and moderation UI connect.
    pub web_bind: String,
    /// How often a full keyframe goes out. This is the ceiling on how long a
    /// bot stays desynced after losing a datagram.
    pub keyframe_interval_ticks: u32,
    /// Consecutive ticks `MATCH_INIT` is repeated on.
    pub match_init_repeats: u32,
    /// Directory holding the built visualizer, served at `/`.
    pub static_dir: String,
}

impl Default for ServerSection {
    fn default() -> Self {
        ServerSection {
            bind: format!("0.0.0.0:{}", bomber_protocol::DEFAULT_UDP_PORT),
            web_bind: format!("127.0.0.1:{}", bomber_protocol::DEFAULT_WEB_PORT),
            keyframe_interval_ticks: 30,
            match_init_repeats: 5,
            static_dir: "visualizer/dist".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LobbySection {
    pub min_players: u8,
    pub max_players: u8,
    pub countdown_ticks: u16,
    /// A seat silent for this long is shown as stale in the moderation UI.
    pub stale_after_ticks: u32,
    pub lobby_status_interval_ticks: u32,
}

impl Default for LobbySection {
    fn default() -> Self {
        LobbySection {
            min_players: 2,
            max_players: 4,
            countdown_ticks: 180,
            stale_after_ticks: 120,
            lobby_status_interval_ticks: 30,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RulesSection {
    pub bomb_fuse_ticks: u16,
    pub flame_duration_ticks: u16,
    pub ticks_per_cell: u8,
    pub speed_step_ticks: u8,
    pub start_bombs: u8,
    pub start_flame: u8,
    pub max_flame: u8,
    pub max_speed: u8,
    pub powerup_chance_pct: u8,
    pub round_time_secs: u32,
    pub sudden_death_secs: u32,
}

impl Default for RulesSection {
    fn default() -> Self {
        let rules = bomber_domain::game::Rules::default();
        RulesSection {
            bomb_fuse_ticks: rules.bomb_fuse_ticks,
            flame_duration_ticks: rules.flame_duration_ticks,
            ticks_per_cell: rules.ticks_per_cell,
            speed_step_ticks: rules.speed_step_ticks,
            start_bombs: rules.start_bombs,
            start_flame: rules.start_flame,
            max_flame: rules.max_flame,
            max_speed: rules.max_speed,
            powerup_chance_pct: rules.powerup_chance_pct,
            round_time_secs: 180,
            sudden_death_secs: 120,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MapSection {
    pub width: u8,
    pub height: u8,
    pub soft_block_density: f32,
    pub symmetry: Symmetry,
    /// `0` rolls a fresh board every match; anything else pins it.
    pub seed: u64,
}

impl Default for MapSection {
    fn default() -> Self {
        let generation = bomber_domain::board::GenerationConfig::default();
        MapSection {
            width: generation.width,
            height: generation.height,
            soft_block_density: generation.soft_block_density,
            symmetry: generation.symmetry,
            seed: 0,
        }
    }
}
