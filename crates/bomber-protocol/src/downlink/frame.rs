use serde::{Deserialize, Serialize};

use crate::codec::{ProtoError, Reader, Result, Writer};

use super::{Assigned, Delta, Keyframe, LobbyStatus, MatchEnd, MatchInit};

/// Frame tags. Stable -- adding a frame means adding a tag, never renumbering.
pub mod frame_type {
    pub const ASSIGNED: u8 = 0x00;
    pub const LOBBY_STATUS: u8 = 0x01;
    pub const MATCH_INIT: u8 = 0x02;
    pub const KEYFRAME: u8 = 0x03;
    pub const DELTA: u8 = 0x04;
    pub const MATCH_END: u8 = 0x05;
}

/// Anything the server sends a bot.
///
/// Every variant shares the `[type][tick]` header, which is written and read
/// here so no individual frame module has to remember to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ServerFrame {
    Assigned(Assigned),
    LobbyStatus(LobbyStatus),
    MatchInit(MatchInit),
    Keyframe(Keyframe),
    Delta(Delta),
    MatchEnd(MatchEnd),
}

impl ServerFrame {
    pub const HEADER_LEN: usize = 5;

    pub fn frame_type(&self) -> u8 {
        match self {
            ServerFrame::Assigned(_) => frame_type::ASSIGNED,
            ServerFrame::LobbyStatus(_) => frame_type::LOBBY_STATUS,
            ServerFrame::MatchInit(_) => frame_type::MATCH_INIT,
            ServerFrame::Keyframe(_) => frame_type::KEYFRAME,
            ServerFrame::Delta(_) => frame_type::DELTA,
            ServerFrame::MatchEnd(_) => frame_type::MATCH_END,
        }
    }

    pub fn tick(&self) -> u32 {
        match self {
            ServerFrame::Assigned(f) => f.tick,
            ServerFrame::LobbyStatus(f) => f.tick,
            ServerFrame::MatchInit(f) => f.tick,
            ServerFrame::Keyframe(f) => f.tick,
            ServerFrame::Delta(f) => f.tick,
            ServerFrame::MatchEnd(f) => f.tick,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(128);
        w.u8(self.frame_type()).u32(self.tick());
        match self {
            ServerFrame::Assigned(f) => f.encode(&mut w),
            ServerFrame::LobbyStatus(f) => f.encode(&mut w),
            ServerFrame::MatchInit(f) => f.encode(&mut w),
            ServerFrame::Keyframe(f) => f.encode(&mut w),
            ServerFrame::Delta(f) => f.encode(&mut w),
            ServerFrame::MatchEnd(f) => f.encode(&mut w),
        }
        w.finish()
    }

    pub fn decode(buf: &[u8]) -> Result<Self> {
        let mut r = Reader::new(buf);
        let frame_type = r.u8()?;
        let tick = r.u32()?;
        Ok(match frame_type {
            self::frame_type::ASSIGNED => ServerFrame::Assigned(Assigned::decode(tick, &mut r)?),
            self::frame_type::LOBBY_STATUS => {
                ServerFrame::LobbyStatus(LobbyStatus::decode(tick, &mut r)?)
            }
            self::frame_type::MATCH_INIT => ServerFrame::MatchInit(MatchInit::decode(tick, &mut r)?),
            self::frame_type::KEYFRAME => ServerFrame::Keyframe(Keyframe::decode(tick, &mut r)?),
            self::frame_type::DELTA => ServerFrame::Delta(Delta::decode(tick, &mut r)?),
            self::frame_type::MATCH_END => ServerFrame::MatchEnd(MatchEnd::decode(tick, &mut r)?),
            other => return Err(ProtoError::UnknownFrame(other)),
        })
    }
}
