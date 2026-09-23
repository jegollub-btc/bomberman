use bomber_application::moderation::MapSettingsPatch;
use bomber_application::ModeratorCommand;
use bomber_domain::board::Symmetry;
use bomber_domain::shared::PlayerId;
use serde::Deserialize;

/// What the moderation UI sends.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Inbound {
    Start,
    Pause,
    Resume,
    /// Stop the running match, leaving the results up.
    End,
    /// Clear the results and return to the lobby.
    Reset,
    Lock,
    Unlock,
    Kick {
        id: u8,
    },
    Rename {
        id: u8,
        name: String,
    },
    ConfigureMap {
        #[serde(default)]
        width: Option<u8>,
        #[serde(default)]
        height: Option<u8>,
        #[serde(default)]
        density: Option<f32>,
        #[serde(default)]
        symmetry: Option<Symmetry>,
        #[serde(default)]
        seed: Option<u64>,
    },
    PreviewMap,
}

impl From<Inbound> for ModeratorCommand {
    fn from(inbound: Inbound) -> Self {
        match inbound {
            Inbound::Start => ModeratorCommand::Start,
            Inbound::Pause => ModeratorCommand::Pause,
            Inbound::Resume => ModeratorCommand::Resume,
            Inbound::End => ModeratorCommand::End,
            Inbound::Reset => ModeratorCommand::Reset,
            Inbound::Lock => ModeratorCommand::Lock,
            Inbound::Unlock => ModeratorCommand::Unlock,
            Inbound::Kick { id } => ModeratorCommand::Kick(PlayerId::new(id)),
            Inbound::Rename { id, name } => ModeratorCommand::Rename {
                player: PlayerId::new(id),
                // A name is shown to humans and stored on the server, so it is
                // bounded here rather than trusted.
                name: name.chars().take(32).collect(),
            },
            Inbound::ConfigureMap {
                width,
                height,
                density,
                symmetry,
                seed,
            } => ModeratorCommand::ConfigureMap(MapSettingsPatch {
                width,
                height,
                density,
                symmetry,
                seed,
            }),
            Inbound::PreviewMap => ModeratorCommand::PreviewMap,
        }
    }
}
