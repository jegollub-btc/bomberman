use std::time::Duration;

/// How long a tick takes to compute.
///
/// Worth measuring rather than assuming: the whole design rests on a tick
/// fitting inside 16.6 ms, and the failure mode when it stops fitting is a
/// game that feels wrong rather than an error anyone would notice.
#[derive(Debug, Default)]
pub struct TickMetrics {
    samples_us: Vec<u32>,
    report_every: usize,
}

impl TickMetrics {
    pub fn reporting_every(ticks: usize) -> Self {
        TickMetrics {
            samples_us: Vec::with_capacity(ticks),
            report_every: ticks,
        }
    }

    pub fn record(&mut self, elapsed: Duration) {
        self.samples_us.push(elapsed.as_micros().min(u32::MAX as u128) as u32);
    }

    /// Returns `(p50, p99, max)` in microseconds once a full window is in.
    pub fn take_report(&mut self) -> Option<(u32, u32, u32)> {
        if self.samples_us.len() < self.report_every.max(1) {
            return None;
        }
        let mut sorted = std::mem::take(&mut self.samples_us);
        sorted.sort_unstable();
        let at = |q: f64| sorted[((sorted.len() - 1) as f64 * q) as usize];
        Some((at(0.50), at(0.99), *sorted.last().unwrap()))
    }
}
