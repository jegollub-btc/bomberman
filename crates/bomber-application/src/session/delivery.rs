use bomber_domain::game::{Event, GameState};
use bomber_protocol::{records_from_events, Delta, Keyframe, ServerFrame};

/// Decides which frame a bot is owed on a given tick.
///
/// A frame goes out on **every** tick, without exception. That is not
/// bandwidth carelessness: a delta names the tick it builds on, and a client
/// applies it only if it holds exactly that tick. Skipping a tick because
/// nothing happened would break the chain and strand every client until the
/// next keyframe -- an empty delta is eleven bytes and keeps them current.
#[derive(Debug, Clone)]
pub struct Delivery {
    keyframe_interval: u32,
    /// The tick the last frame carried, which is the base any delta builds on.
    last_sent_tick: Option<u32>,
}

impl Delivery {
    pub fn new(keyframe_interval: u32) -> Self {
        Delivery {
            keyframe_interval: keyframe_interval.max(1),
            last_sent_tick: None,
        }
    }

    /// Start of a match: the next frame must be a keyframe, since clients hold
    /// nothing yet.
    pub fn restart(&mut self) {
        self.last_sent_tick = None;
    }

    pub fn is_keyframe_tick(&self, tick: u32) -> bool {
        self.last_sent_tick.is_none() || tick.is_multiple_of(self.keyframe_interval)
    }

    pub fn frame_for(&mut self, state: &GameState, events: &[Event]) -> ServerFrame {
        let frame = if self.is_keyframe_tick(state.tick) {
            ServerFrame::Keyframe(Keyframe::from_state(state))
        } else {
            ServerFrame::Delta(Delta {
                tick: state.tick,
                // Always the previous tick, because a frame went out on it.
                base_tick: self.last_sent_tick.unwrap_or(state.tick.saturating_sub(1)),
                records: records_from_events(events),
            })
        };
        self.last_sent_tick = Some(state.tick);
        frame
    }
}
