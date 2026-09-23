//! The Bomberman arena server.
//!
//! Composition root and nothing else: it loads configuration, builds the
//! session, and wires the two adapters to it. Every decision about the game is
//! made further in -- the rules in `bomber-domain`, the delivery policy in
//! `bomber-application` -- so this file stays a description of the plumbing.

mod config;
mod runtime;
mod udp;
mod web;

use anyhow::{Context, Result};
use bomber_application::ArenaSession;

use runtime::Shared;
use udp::Endpoint;

/// Where the config lives unless `--config` says otherwise.
const DEFAULT_CONFIG: &str = "config/server.toml";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,bomber_server=info".into()),
        )
        .init();

    let path = config_path();
    let config: config::ServerConfig = config::load(&path).with_context(|| format!("loading {path}"))?;
    tracing::info!(
        map = format!(
            "{}x{}",
            config.session.map.generation.width, config.session.map.generation.height
        ),
        players = format!(
            "{}-{}",
            config.session.min_players, config.session.max_players
        ),
        "configuration loaded"
    );

    // The only entropy the whole system needs: everything downstream of the
    // match seed is deterministic, so this is the single point where a run
    // becomes irreproducible -- and pinning `map.seed` removes even that.
    let entropy = rand::random::<u64>();
    let shared = Shared::new(ArenaSession::new(config.session, entropy));

    let endpoint = Endpoint::bind(config.udp_bind)
        .await
        .with_context(|| format!("binding {}", config.udp_bind))?;

    tokio::spawn(udp::listen(endpoint.clone(), shared.clone()));
    tokio::spawn(runtime::run(endpoint, shared.clone()));

    web::serve(config.web_bind, &config.static_dir, shared).await
}

fn config_path() -> String {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" | "-c" => {
                if let Some(path) = args.next() {
                    return path;
                }
            }
            other if other.starts_with("--config=") => {
                return other.trim_start_matches("--config=").to_string();
            }
            _ => {}
        }
    }
    DEFAULT_CONFIG.to_string()
}
