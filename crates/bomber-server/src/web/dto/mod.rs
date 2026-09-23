//! The JSON the browser sees.
//!
//! These types exist so the wire shape the front end is written against is
//! stated in one place and can change without touching the domain. They are the
//! contract documented in `VISUALIZER_GUIDE.md` and `MODERATION_API.md`.

mod command;
mod events;
mod lobby;
mod match_state;

pub use command::Inbound;
pub use events::EventDto;
pub use lobby::{lobby_dto, LobbyDto};
pub use match_state::{
    board_dto, match_end_dto, match_init_dto, state_dto, MatchEndDto, MatchInitDto, StateDto,
};

use serde::Serialize;

/// Everything the server pushes down a socket.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Outbound {
    Lobby(LobbyDto),
    MatchInit(MatchInitDto),
    State(StateDto),
    MatchEnd(MatchEndDto),
    MapPreview(MatchInitDto),
    /// A command succeeded.
    Ack { cmd: String },
    /// A command could not run, and why. Never a silent no-op: a UI has to be
    /// able to explain the button that did nothing.
    Error { cmd: String, message: String },
}

impl Outbound {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|error| {
            // Serialising our own types cannot fail in practice; if it somehow
            // does, a parseable error beats a dropped frame.
            format!(r#"{{"type":"error","cmd":"","message":"serialize: {error}"}}"#)
        })
    }
}
