use crate::shared::PlayerId;

/// One slot at the table.
///
/// Seats exist whether or not anyone is in them, so a UI can render a fixed set
/// of places rather than a list that grows and shifts as bots connect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seat {
    pub id: PlayerId,
    pub occupied: bool,
    /// Display name. A bot may propose one when it joins; a moderator may
    /// override it afterwards. Always sanitised -- see [`Seat::set_name`].
    pub name: String,
}

impl Seat {
    /// Longest name kept, in characters.
    ///
    /// Counted in characters rather than bytes because this is the limit a
    /// person sees; the wire format has its own, smaller, byte limit.
    pub const MAX_NAME_CHARS: usize = 24;

    pub fn empty(id: PlayerId) -> Self {
        Seat {
            id,
            occupied: false,
            name: Self::default_name(id),
        }
    }

    pub fn default_name(id: PlayerId) -> String {
        format!("bot-{}", id.raw())
    }

    /// Set the display name, or fall back to the default if nothing usable is
    /// left after cleaning it up.
    ///
    /// Every name entering the system goes through here -- the one a bot sends
    /// in its hello and the one a moderator types -- because a name is shown to
    /// people, and two paths with different rules is how one of them ends up
    /// unbounded or full of control characters.
    pub fn set_name(&mut self, proposed: &str) {
        self.name = sanitize(proposed).unwrap_or_else(|| Self::default_name(self.id));
    }

    pub fn vacate(&mut self) {
        self.occupied = false;
        self.name = Self::default_name(self.id);
    }
}

/// Strip control characters, collapse surrounding whitespace, and cap the
/// length. `None` if nothing usable remains.
fn sanitize(proposed: &str) -> Option<String> {
    let cleaned: String = proposed
        .chars()
        .filter(|c| !c.is_control())
        .take(Seat::MAX_NAME_CHARS)
        .collect();
    let trimmed = cleaned.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(proposed: &str) -> String {
        let mut seat = Seat::empty(PlayerId::new(2));
        seat.set_name(proposed);
        seat.name
    }

    #[test]
    fn an_ordinary_name_is_kept() {
        assert_eq!(named("team-rocket"), "team-rocket");
    }

    #[test]
    fn surrounding_whitespace_goes() {
        assert_eq!(named("  spacey  "), "spacey");
    }

    /// A name goes into a UI and a log line. Control characters in either are
    /// somebody else deciding what those look like.
    #[test]
    fn control_characters_are_stripped() {
        assert_eq!(named("ro\u{7}bot\nnewline"), "robotnewline");
    }

    #[test]
    fn a_long_name_is_capped() {
        let name = named(&"x".repeat(200));
        assert_eq!(name.chars().count(), Seat::MAX_NAME_CHARS);
    }

    #[test]
    fn an_empty_or_blank_name_falls_back_to_the_default() {
        assert_eq!(named(""), "bot-2");
        assert_eq!(named("   "), "bot-2");
        assert_eq!(named("\u{0}\u{1}"), "bot-2");
    }

    #[test]
    fn leaving_a_seat_forgets_the_name() {
        let mut seat = Seat::empty(PlayerId::new(1));
        seat.occupied = true;
        seat.set_name("someone");
        seat.vacate();
        assert_eq!(seat.name, "bot-1");
        assert!(!seat.occupied);
    }
}
