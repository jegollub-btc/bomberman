use serde::{Deserialize, Serialize};

use bomber_domain::game::{EndReason, MatchOutcome};
use bomber_domain::shared::PlayerId;

use crate::codec::{ProtoError, Reader, Result, Writer};
use crate::{decode_optional_player, encode_optional_player};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerResultRecord {
    pub id: PlayerId,
    /// 1 is the winner. Tied players share a placement.
    pub placement: u8,
    pub score: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchEnd {
    pub tick: u32,
    pub reason: EndReason,
    /// `None` on a draw.
    pub winner: Option<PlayerId>,
    pub results: Vec<PlayerResultRecord>,
}

impl MatchEnd {
    pub fn from_outcome(tick: u32, outcome: &MatchOutcome) -> Self {
        MatchEnd {
            tick,
            reason: outcome.reason,
            winner: outcome.winner,
            results: outcome
                .results
                .iter()
                .map(|r| PlayerResultRecord {
                    id: r.id,
                    placement: r.placement,
                    score: r.score,
                })
                .collect(),
        }
    }

    pub fn encode(&self, w: &mut Writer) {
        w.u8(self.reason.code())
            .u8(encode_optional_player(self.winner))
            .u8(self.results.len() as u8);
        for result in &self.results {
            w.u8(result.id.raw()).u8(result.placement).u16(result.score);
        }
    }

    pub fn decode(tick: u32, r: &mut Reader) -> Result<Self> {
        let raw = r.u8()?;
        let reason =
            EndReason::from_code(raw).ok_or_else(|| ProtoError::invalid("end_reason", raw))?;
        let winner = decode_optional_player(r.u8()?);
        let count = r.u8()? as usize;
        let results = r.repeat(count, |r| {
            Ok(PlayerResultRecord {
                id: PlayerId::new(r.u8()?),
                placement: r.u8()?,
                score: r.u16()?,
            })
        })?;
        Ok(MatchEnd {
            tick,
            reason,
            winner,
            results,
        })
    }
}
