use std::sync::Arc;
use std::time::{Duration, Instant};

use bomber_protocol::{ServerFrame, TICK_RATE};
use tokio::time::MissedTickBehavior;

use crate::udp::Endpoint;
use crate::web::dto::{match_end_dto, match_init_dto, state_dto, Outbound};

use super::{Shared, TickMetrics};

const TICK_PERIOD: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE as u64);
/// Log tick timings once every ten seconds.
const REPORT_EVERY: usize = TICK_RATE as usize * 10;
/// Refresh the lobby view twice a second even when nothing structural changed.
///
/// Packets-per-second and staleness are measurements, not events: without a
/// heartbeat the moderation UI's health readout would freeze at whatever it
/// happened to show when the roster last changed, which is exactly when it
/// matters least.
const LOBBY_REFRESH_TICKS: u32 = TICK_RATE as u32 / 2;

/// The 60 Hz driver.
///
/// Everything it sends is decided by the session, synchronously, while the lock
/// is held; the lock is then released and only the I/O happens. Nothing here
/// knows the rules.
pub async fn run(endpoint: Endpoint, shared: Shared) {
    let mut ticker = tokio::time::interval(TICK_PERIOD);
    // A hitch must not be repaid by sprinting through the backlog: the
    // simulation is tick-based, so catching up would fast-forward the match.
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut metrics = TickMetrics::reporting_every(REPORT_EVERY);

    loop {
        ticker.tick().await;
        let started = Instant::now();

        let outbox = build_outbox(&shared);
        metrics.record(started.elapsed());

        deliver(&endpoint, &shared, outbox).await;

        if let Some((p50, p99, max)) = metrics.take_report() {
            tracing::debug!(p50_us = p50, p99_us = p99, max_us = max, "tick timings");
            if max > TICK_PERIOD.as_micros() as u32 {
                tracing::warn!(max_us = max, "a tick exceeded its budget");
            }
        }
    }
}

#[derive(Default)]
struct Outbox {
    unicast: Vec<(bomber_domain::shared::PlayerId, ServerFrame)>,
    broadcast: Vec<ServerFrame>,
    messages: Vec<Arc<str>>,
    lobby_changed: bool,
}

fn build_outbox(shared: &Shared) -> Outbox {
    let mut outbox = Outbox::default();

    {
        let mut session = shared.session.lock().unwrap();
        let report = session.tick();

        outbox.unicast = report.unicast;
        outbox.broadcast = report.broadcast;
        outbox.lobby_changed = report.lobby_changed
            || session.session_tick().is_multiple_of(LOBBY_REFRESH_TICKS);

        if report.match_started {
            let snapshot = session.snapshot();
            if let Some(init) = match_init_dto(&session, &snapshot) {
                outbox.messages.push(json(&Outbound::MatchInit(init)));
            }
        }
        if report.simulated {
            if let Some(game) = session.game() {
                outbox
                    .messages
                    .push(json(&Outbound::State(state_dto(game, &report.events))));
            }
        }
        if let Some(outcome) = &report.match_ended {
            let tick = session.game().map_or(0, |game| game.tick);
            outbox
                .messages
                .push(json(&Outbound::MatchEnd(match_end_dto(tick, outcome))));
        }
    }

    // The lobby message needs the registry, so it is built after the session
    // lock is released rather than while holding both.
    if outbox.lobby_changed {
        outbox.messages.insert(0, json(&shared.lobby_message()));
    }

    outbox
}

fn json(outbound: &Outbound) -> Arc<str> {
    Arc::from(outbound.to_json())
}

async fn deliver(endpoint: &Endpoint, shared: &Shared, outbox: Outbox) {
    for (player, frame) in &outbox.unicast {
        endpoint.send_to_player(shared, *player, frame).await;
    }

    if !outbox.broadcast.is_empty() {
        let endpoints = shared.registry.lock().unwrap().endpoints();
        for frame in &outbox.broadcast {
            for (_, addr) in &endpoints {
                endpoint.send(*addr, frame).await;
            }
        }
    }

    for message in outbox.messages {
        shared.publish_json(message);
    }
}
