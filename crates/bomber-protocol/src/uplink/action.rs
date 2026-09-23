use serde::{Deserialize, Serialize};

use bomber_domain::game::Intent;
use bomber_domain::shared::Direction;

use crate::codec::{ProtoError, Result};

/// What a bot asks for on a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Action {
    /// "I am alive and doing nothing."
    ///
    /// Identical in effect to sending no packet at all -- it exists purely so a
    /// bot that is genuinely idle can still be told apart from a crashed one in
    /// the moderation UI. It is a liveness heartbeat, sent a couple of times a
    /// second at most, **not** something to transmit on every tick.
    Idle = 0,
    Up = 1,
    Down = 2,
    Left = 3,
    Right = 4,
    Bomb = 5,
    UpBomb = 6,
    DownBomb = 7,
    LeftBomb = 8,
    RightBomb = 9,
    /// Request a slot, or ask for `MATCH_INIT` to be sent again.
    Hello = 15,
}

impl Action {
    pub fn from_code(code: u8) -> Result<Self> {
        Ok(match code {
            0 => Action::Idle,
            1 => Action::Up,
            2 => Action::Down,
            3 => Action::Left,
            4 => Action::Right,
            5 => Action::Bomb,
            6 => Action::UpBomb,
            7 => Action::DownBomb,
            8 => Action::LeftBomb,
            9 => Action::RightBomb,
            15 => Action::Hello,
            other => return Err(ProtoError::invalid("action", other)),
        })
    }

    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The movement component, if any.
    pub const fn movement(self) -> Option<Direction> {
        match self {
            Action::Up | Action::UpBomb => Some(Direction::Up),
            Action::Down | Action::DownBomb => Some(Direction::Down),
            Action::Left | Action::LeftBomb => Some(Direction::Left),
            Action::Right | Action::RightBomb => Some(Direction::Right),
            _ => None,
        }
    }

    /// Does this also drop a bomb on the player's current cell?
    pub const fn places_bomb(self) -> bool {
        matches!(
            self,
            Action::Bomb
                | Action::UpBomb
                | Action::DownBomb
                | Action::LeftBomb
                | Action::RightBomb
        )
    }

    /// Translate into the domain's vocabulary.
    ///
    /// [`Action::Hello`] carries no intent -- it is a transport-level request,
    /// not a move -- so it maps to idle.
    pub const fn intent(self) -> Intent {
        Intent {
            movement: self.movement(),
            place_bomb: self.places_bomb(),
        }
    }

    /// Combine a direction with an optional bomb drop.
    pub const fn moving(direction: Direction, bomb: bool) -> Action {
        match (direction, bomb) {
            (Direction::Up, false) => Action::Up,
            (Direction::Down, false) => Action::Down,
            (Direction::Left, false) => Action::Left,
            (Direction::Right, false) => Action::Right,
            (Direction::Up, true) => Action::UpBomb,
            (Direction::Down, true) => Action::DownBomb,
            (Direction::Left, true) => Action::LeftBomb,
            (Direction::Right, true) => Action::RightBomb,
        }
    }

    /// The action that expresses `intent`, if one exists.
    pub fn from_intent(intent: Intent) -> Action {
        match (intent.movement, intent.place_bomb) {
            (Some(direction), bomb) => Action::moving(direction, bomb),
            (None, true) => Action::Bomb,
            (None, false) => Action::Idle,
        }
    }
}
