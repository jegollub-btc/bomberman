use bomber_domain::board::Board;
use bomber_domain::shared::PlayerId;

/// The single reply every command gets.
///
/// A command that cannot run right now is [`CommandOutcome::Rejected`] with a
/// reason, never a silent no-op -- otherwise a UI can only report that the
/// button did nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandOutcome {
    Accepted,
    /// Accepted, and the adapter must also drop this player's transport
    /// binding so the seat is genuinely free.
    AcceptedAndReleased(PlayerId),
    /// A generated board, plus the seed that produced it -- without the seed a
    /// preview cannot be pinned for the match it is previewing.
    Preview { board: Box<Board>, seed: u64 },
    Rejected(String),
}

impl CommandOutcome {
    pub fn rejected(reason: impl Into<String>) -> Self {
        CommandOutcome::Rejected(reason.into())
    }

    pub fn is_accepted(&self) -> bool {
        !matches!(self, CommandOutcome::Rejected(_))
    }
}
