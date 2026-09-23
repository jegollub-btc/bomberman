//! Changes since an explicitly named tick.
//!
//! A delta is only meaningful against the exact state it was built from, which
//! is why `base_tick` is on the wire rather than implied. A client that does
//! not hold that tick must discard the frame and wait for the next keyframe --
//! with no acknowledgement channel, that is the whole of loss recovery.

mod from_event;
mod record;

pub use from_event::records_from_events;
pub use record::{record_type, DeltaRecord};

use serde::{Deserialize, Serialize};

use crate::codec::{Reader, Result, Writer};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delta {
    pub tick: u32,
    /// The tick this delta builds on. Discard the frame unless you hold state
    /// at exactly this tick.
    pub base_tick: u32,
    pub records: Vec<DeltaRecord>,
}

impl Delta {
    pub fn encode(&self, w: &mut Writer) {
        // u16 count: a single chain reaction can produce hundreds of records.
        w.u32(self.base_tick).u16(self.records.len() as u16);
        for record in &self.records {
            record.encode(w);
        }
    }

    pub fn decode(tick: u32, r: &mut Reader) -> Result<Self> {
        let base_tick = r.u32()?;
        let count = r.u16()? as usize;
        let records = r.repeat(count, DeltaRecord::decode)?;
        Ok(Delta {
            tick,
            base_tick,
            records,
        })
    }
}
