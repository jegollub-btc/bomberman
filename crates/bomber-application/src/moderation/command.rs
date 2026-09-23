use bomber_domain::board::Symmetry;
use bomber_domain::shared::PlayerId;

/// Fields of the map configuration, all optional so a UI can send only what
/// the operator actually changed.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MapSettingsPatch {
    pub width: Option<u8>,
    pub height: Option<u8>,
    pub density: Option<f32>,
    pub symmetry: Option<Symmetry>,
    /// `Some(0)` means "random per match".
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModeratorCommand {
    /// Begin the countdown, then the match.
    Start,
    /// Freeze the tick loop. Bots keep their sockets; the clock stops.
    Pause,
    Resume,
    /// Stop the running match now, leaving the results up.
    End,
    /// Clear the results and return to the lobby.
    Reset,
    Lock,
    Unlock,
    Kick(PlayerId),
    Rename {
        player: PlayerId,
        name: String,
    },
    /// Settings for the *next* match.
    ConfigureMap(MapSettingsPatch),
    /// Generate a board with the current settings without starting anything.
    PreviewMap,
}

impl ModeratorCommand {
    /// The name echoed back in the `ack` / `error` reply.
    pub fn name(&self) -> &'static str {
        match self {
            ModeratorCommand::Start => "start",
            ModeratorCommand::Pause => "pause",
            ModeratorCommand::Resume => "resume",
            ModeratorCommand::End => "end",
            ModeratorCommand::Reset => "reset",
            ModeratorCommand::Lock => "lock",
            ModeratorCommand::Unlock => "unlock",
            ModeratorCommand::Kick(_) => "kick",
            ModeratorCommand::Rename { .. } => "rename",
            ModeratorCommand::ConfigureMap(_) => "configure_map",
            ModeratorCommand::PreviewMap => "preview_map",
        }
    }
}
