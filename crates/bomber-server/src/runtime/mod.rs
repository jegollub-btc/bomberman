//! Wiring: shared state, and the 60 Hz driver that turns the session's
//! decisions into datagrams and JSON.

mod metrics;
mod shared;
mod tick_loop;

pub use metrics::TickMetrics;
pub use shared::Shared;
pub use tick_loop::run;
