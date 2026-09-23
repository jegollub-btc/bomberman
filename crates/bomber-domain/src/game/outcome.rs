use serde::{Deserialize, Serialize};

use crate::shared::PlayerId;

/// Why a match ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum EndReason {
    LastStanding = 0,
    Timeout = 1,
    /// A moderator reset a match that was still running.
    Aborted = 2,
}

impl EndReason {
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(EndReason::LastStanding),
            1 => Some(EndReason::Timeout),
            2 => Some(EndReason::Aborted),
            _ => None,
        }
    }

    pub const fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerResult {
    pub id: PlayerId,
    /// 1 is the winner. Tied players share a placement.
    pub placement: u8,
    pub score: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchOutcome {
    pub reason: EndReason,
    /// `None` on a draw.
    pub winner: Option<PlayerId>,
    /// Ordered by player id, so the UI can index into it directly.
    pub results: Vec<PlayerResult>,
}
