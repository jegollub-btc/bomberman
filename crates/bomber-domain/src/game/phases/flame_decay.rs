//! Phase 1: existing fire ages by one tick and expired cells go out.
//!
//! Runs before detonation so that a flame kindled this tick burns for its full
//! duration rather than losing its first tick immediately.

use crate::game::{Event, FlameField};

pub fn run(flames: &mut FlameField, events: &mut Vec<Event>) {
    for cell in flames.decay() {
        events.push(Event::FlameDied { cell });
    }
}
