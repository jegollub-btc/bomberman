use crate::shared::PlayerId;

/// One slot at the table.
///
/// Seats exist whether or not anyone is in them, so a UI can render a fixed set
/// of places rather than a list that grows and shifts as bots connect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seat {
    pub id: PlayerId,
    pub occupied: bool,
    /// Display name. Bots cannot supply one -- their uplink is two bytes wide --
    /// so this is set by a moderator and defaults to `bot-<id>`.
    pub name: String,
}

impl Seat {
    pub fn empty(id: PlayerId) -> Self {
        Seat {
            id,
            occupied: false,
            name: format!("bot-{}", id.raw()),
        }
    }

    pub fn vacate(&mut self) {
        self.occupied = false;
        self.name = format!("bot-{}", self.id.raw());
    }
}
