use crate::shared::Direction;

/// What a player wants to do on a tick.
///
/// This is the domain's own vocabulary, not the wire format's. Adapters
/// translate whatever arrives on their transport into an `Intent`, which keeps
/// the rules independent of how bots happen to talk to us.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Intent {
    pub movement: Option<Direction>,
    pub place_bomb: bool,
}

impl Intent {
    /// Do nothing. Also what a player gets when no input arrived at all --
    /// silence and an explicit idle are indistinguishable by design, so a bot
    /// is never obliged to transmit on ticks where it has nothing to say.
    pub const IDLE: Intent = Intent {
        movement: None,
        place_bomb: false,
    };

    pub const fn moving(direction: Direction) -> Self {
        Intent {
            movement: Some(direction),
            place_bomb: false,
        }
    }

    pub const fn bomb() -> Self {
        Intent {
            movement: None,
            place_bomb: true,
        }
    }

    pub const fn with_bomb(mut self) -> Self {
        self.place_bomb = true;
        self
    }
}
