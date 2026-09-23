use bomber_domain::shared::PlayerId;

/// A one-second ring of packet counts, so "packets per second" is a real
/// measurement rather than a running average that lags reality.
const WINDOW_TICKS: usize = 60;

/// What the moderation UI needs to decide whether a seat is really there.
///
/// Note what is *not* here: round-trip time. Measuring it needs an echo, and a
/// two-byte uplink has no room for a token to echo. Staleness -- how long since
/// this bot last said anything -- is measurable and is the thing that actually
/// answers "is it safe to press Start".
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SeatPresence {
    pub packets_total: u32,
    pub packets_per_sec: u32,
    pub last_seen_tick: Option<u32>,
    pub loss_pct: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Presence {
    seats: Vec<SeatStats>,
}

#[derive(Debug, Clone)]
struct SeatStats {
    packets_total: u32,
    last_seen_tick: Option<u32>,
    window: [u8; WINDOW_TICKS],
}

impl Default for SeatStats {
    fn default() -> Self {
        SeatStats {
            packets_total: 0,
            last_seen_tick: None,
            window: [0; WINDOW_TICKS],
        }
    }
}

impl Presence {
    pub fn with_seats(seats: usize) -> Self {
        Presence {
            seats: vec![SeatStats::default(); seats],
        }
    }

    /// Note that a packet arrived from this seat on this tick.
    pub fn record(&mut self, player: PlayerId, tick: u32) {
        if let Some(stats) = self.seats.get_mut(player.index()) {
            stats.packets_total += 1;
            stats.last_seen_tick = Some(tick);
            let bucket = tick as usize % WINDOW_TICKS;
            stats.window[bucket] = stats.window[bucket].saturating_add(1);
        }
    }

    /// Clear the bucket this tick is about to reuse, so the window stays a
    /// window rather than a total.
    pub fn advance(&mut self, tick: u32) {
        let bucket = tick as usize % WINDOW_TICKS;
        for stats in &mut self.seats {
            stats.window[bucket] = 0;
        }
    }

    pub fn reset_seat(&mut self, player: PlayerId) {
        if let Some(stats) = self.seats.get_mut(player.index()) {
            *stats = SeatStats::default();
        }
    }

    pub fn of(&self, player: PlayerId, loss_pct: f32) -> SeatPresence {
        self.seats
            .get(player.index())
            .map(|stats| SeatPresence {
                packets_total: stats.packets_total,
                packets_per_sec: stats.window.iter().map(|&n| u32::from(n)).sum(),
                last_seen_tick: stats.last_seen_tick,
                loss_pct,
            })
            .unwrap_or_default()
    }

    /// Ticks since this seat last sent anything.
    pub fn staleness(&self, player: PlayerId, now: u32) -> Option<u32> {
        self.seats
            .get(player.index())
            .and_then(|s| s.last_seen_tick)
            .map(|seen| now.saturating_sub(seen))
    }
}
