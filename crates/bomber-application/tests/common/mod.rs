#![allow(dead_code)]

use bomber_application::{ArenaSession, ModeratorCommand, SessionConfig};
use bomber_domain::game::Rules;
use bomber_domain::shared::PlayerId;
use bomber_protocol::ServerFrame;

/// A session that starts a match on the first tick after `Start`, so tests do
/// not sit through a three-second countdown.
pub fn session(players: u8) -> ArenaSession {
    let config = SessionConfig {
        countdown_ticks: 1,
        min_players: 2,
        max_players: 4,
        rules: Rules {
            round_time_ticks: 3600,
            sudden_death_tick: 1800,
            ..Rules::default()
        },
        ..SessionConfig::default()
    };
    let mut session = ArenaSession::new(config, 0xC0FFEE);
    for _ in 0..players {
        session.admit().expect("seat available");
    }
    session
}

/// Admit players, start, and tick until the match is running.
pub fn running(players: u8) -> ArenaSession {
    let mut session = session(players);
    assert!(session.execute(ModeratorCommand::Start).is_accepted());
    session.tick(); // the countdown expires and the match begins on this tick
    session
}

pub fn player(raw: u8) -> PlayerId {
    PlayerId::new(raw)
}

pub fn is_keyframe(frame: &ServerFrame) -> bool {
    matches!(frame, ServerFrame::Keyframe(_))
}

pub fn as_delta(frame: &ServerFrame) -> Option<&bomber_protocol::Delta> {
    match frame {
        ServerFrame::Delta(delta) => Some(delta),
        _ => None,
    }
}
