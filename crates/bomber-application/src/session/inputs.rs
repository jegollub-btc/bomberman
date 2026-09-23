use bomber_domain::game::Intent;
use bomber_domain::shared::PlayerId;
use bomber_protocol::{seq_is_newer, Action};

/// The newest intent received from each bot inside the current tick's window.
///
/// Only the newest packet counts: a bot that bursts gains nothing, and one that
/// stays silent simply keeps doing nothing. The sequence counter is what makes
/// "newest" decidable at all -- UDP reorders and duplicates freely, so without
/// it a stale datagram arriving late would overwrite a fresh intent.
#[derive(Debug, Default, Clone)]
pub struct InputSlots {
    slots: Vec<Slot>,
}

#[derive(Debug, Default, Clone, Copy)]
struct Slot {
    pending: Option<Intent>,
    last_seq: Option<u8>,
    /// Packets we can infer were lost, from gaps in the sequence counter.
    missed: u32,
    accepted: u32,
}

/// Why a submitted packet did not become this tick's intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    UnknownSeat,
    /// A duplicate or a datagram that arrived out of order.
    Stale,
}

impl InputSlots {
    pub fn with_seats(seats: usize) -> Self {
        InputSlots {
            slots: vec![Slot::default(); seats],
        }
    }

    /// Record a packet. Returns whether it became the pending intent.
    pub fn submit(
        &mut self,
        player: PlayerId,
        action: Action,
        seq: u8,
    ) -> Result<(), Rejection> {
        let slot = self
            .slots
            .get_mut(player.index())
            .ok_or(Rejection::UnknownSeat)?;

        if let Some(previous) = slot.last_seq {
            if !seq_is_newer(seq, previous) {
                return Err(Rejection::Stale);
            }
            // A gap of n means n-1 packets never arrived. Wrapping keeps this
            // an estimate rather than a count, which is all a health readout
            // needs.
            let gap = seq.wrapping_sub(previous) & 0x0F;
            slot.missed += u32::from(gap.saturating_sub(1));
        }
        slot.last_seq = Some(seq);
        slot.accepted += 1;
        slot.pending = Some(action.intent());
        Ok(())
    }

    /// Take this tick's intents, leaving the slots empty for the next one.
    pub fn take(&mut self) -> Vec<Option<Intent>> {
        self.slots.iter_mut().map(|s| s.pending.take()).collect()
    }

    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            slot.pending = None;
        }
    }

    /// Forget everything about a seat, for a kick or a new occupant.
    pub fn reset_seat(&mut self, player: PlayerId) {
        if let Some(slot) = self.slots.get_mut(player.index()) {
            *slot = Slot::default();
        }
    }

    /// Estimated inbound loss for a seat, in percent.
    pub fn loss_pct(&self, player: PlayerId) -> f32 {
        self.slots
            .get(player.index())
            .map(|slot| {
                let total = slot.accepted + slot.missed;
                if total == 0 {
                    0.0
                } else {
                    slot.missed as f32 * 100.0 / total as f32
                }
            })
            .unwrap_or(0.0)
    }
}
