use serde::{Deserialize, Serialize};

/// Where a session sits between matches.
///
/// ```text
///   Open ──lock──► Locked ──start──► Countdown ──► Running ──► MatchOver
///     ▲                                                             │
///     └──────────────────────── reset ──────────────────────────────┘
/// ```
///
/// Only a moderator moves `Locked -> Countdown`. Nothing starts a match on its
/// own, because "wait until it looks full and go" is exactly the behaviour that
/// starts a tournament round without one of the competitors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum LobbyState {
    /// Accepting new bots.
    Open = 0,
    /// Roster frozen; no new bots admitted.
    Locked = 1,
    Countdown = 2,
    Running = 3,
    MatchOver = 4,
}

impl LobbyState {
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(LobbyState::Open),
            1 => Some(LobbyState::Locked),
            2 => Some(LobbyState::Countdown),
            3 => Some(LobbyState::Running),
            4 => Some(LobbyState::MatchOver),
            _ => None,
        }
    }

    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Is a match being simulated right now?
    pub const fn is_playing(self) -> bool {
        matches!(self, LobbyState::Running)
    }

    pub const fn admits_new_players(self) -> bool {
        matches!(self, LobbyState::Open)
    }
}
