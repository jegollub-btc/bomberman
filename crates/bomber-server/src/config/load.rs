use std::net::SocketAddr;
use std::path::Path;

use anyhow::{Context, Result};
use bomber_application::{MapSettings, SessionConfig};
use bomber_domain::board::GenerationConfig;
use bomber_domain::game::Rules;
use bomber_protocol::TICK_RATE;

use super::model::ConfigFile;

/// Configuration in the shape the server actually needs, with the session's
/// share already translated.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub udp_bind: SocketAddr,
    pub web_bind: SocketAddr,
    pub static_dir: String,
    pub session: SessionConfig,
}

/// Read `path`, falling back to defaults if it does not exist.
///
/// A missing config is not an error: the defaults are a playable game, and
/// requiring a file just to start the server would make the first run harder
/// than it needs to be.
pub fn load(path: impl AsRef<Path>) -> Result<ServerConfig> {
    let path = path.as_ref();
    let file = if path.exists() {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str::<ConfigFile>(&text)
            .with_context(|| format!("parsing {}", path.display()))?
    } else {
        tracing::warn!(path = %path.display(), "no config file; using defaults");
        ConfigFile::default()
    };

    let ticks = |secs: u32| secs.saturating_mul(u32::from(TICK_RATE));

    Ok(ServerConfig {
        udp_bind: file
            .server
            .bind
            .parse()
            .with_context(|| format!("server.bind: {:?}", file.server.bind))?,
        web_bind: file
            .server
            .web_bind
            .parse()
            .with_context(|| format!("server.web_bind: {:?}", file.server.web_bind))?,
        static_dir: file.server.static_dir,
        session: SessionConfig {
            rules: Rules {
                bomb_fuse_ticks: file.rules.bomb_fuse_ticks,
                flame_duration_ticks: file.rules.flame_duration_ticks,
                ticks_per_cell: file.rules.ticks_per_cell.max(1),
                speed_step_ticks: file.rules.speed_step_ticks,
                start_bombs: file.rules.start_bombs.max(1),
                start_flame: file.rules.start_flame.max(1),
                max_flame: file.rules.max_flame,
                max_speed: file.rules.max_speed,
                powerup_chance_pct: file.rules.powerup_chance_pct.min(100),
                round_time_ticks: ticks(file.rules.round_time_secs),
                // Sudden death is expressed in seconds from the start of the
                // match, like the round timer, rather than as a raw tick count.
                sudden_death_tick: ticks(file.rules.sudden_death_secs),
            },
            map: MapSettings {
                generation: GenerationConfig {
                    width: file.map.width,
                    height: file.map.height,
                    soft_block_density: file.map.soft_block_density,
                    symmetry: file.map.symmetry,
                },
                seed: file.map.seed,
            },
            min_players: file.lobby.min_players.max(1),
            max_players: file.lobby.max_players.max(1),
            keyframe_interval_ticks: file.server.keyframe_interval_ticks.max(1),
            countdown_ticks: file.lobby.countdown_ticks,
            match_init_repeats: file.server.match_init_repeats.max(1),
            lobby_status_interval_ticks: file.lobby.lobby_status_interval_ticks.max(1),
            stale_after_ticks: file.lobby.stale_after_ticks,
        },
    })
}
