use serde::{Deserialize, Serialize};

/// The tunable constants of a match.
///
/// Shipped verbatim to every bot at match start so nobody has to hardcode them:
/// the moderator can retune the server and bots follow along without a rebuild.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    /// Ticks from placement to detonation.
    pub bomb_fuse_ticks: u16,
    /// Ticks a flame cell stays lethal.
    pub flame_duration_ticks: u16,
    /// Ticks to cross one cell at speed level 0.
    pub ticks_per_cell: u8,
    /// Ticks subtracted from `ticks_per_cell` per speed level.
    pub speed_step_ticks: u8,
    pub start_bombs: u8,
    pub start_flame: u8,
    pub max_flame: u8,
    pub max_speed: u8,
    /// Chance in percent that a destroyed soft block drops a power-up. The
    /// kinds are then equally likely.
    pub powerup_chance_pct: u8,
    /// Total match length in ticks.
    pub round_time_ticks: u32,
    /// Tick at which the board starts closing in. `u32::MAX` disables it.
    pub sudden_death_tick: u32,
}

impl Rules {
    /// Ticks to cross one cell at a given speed level.
    ///
    /// Never returns 0: a zero-tick step would let a player cross the board in
    /// a single tick and divide by zero during interpolation.
    pub fn ticks_per_cell_at(&self, speed: u8) -> u8 {
        let penalty = self.speed_step_ticks.saturating_mul(speed);
        self.ticks_per_cell.saturating_sub(penalty).max(1)
    }
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            bomb_fuse_ticks: 120,
            flame_duration_ticks: 30,
            ticks_per_cell: 8,
            speed_step_ticks: 1,
            start_bombs: 1,
            start_flame: 1,
            max_flame: 6,
            max_speed: 3,
            powerup_chance_pct: 30,
            round_time_ticks: 180 * 60,
            sudden_death_tick: 120 * 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_levels_never_reach_zero_ticks_per_cell() {
        let rules = Rules {
            ticks_per_cell: 3,
            speed_step_ticks: 2,
            ..Rules::default()
        };
        assert_eq!(rules.ticks_per_cell_at(0), 3);
        assert_eq!(rules.ticks_per_cell_at(1), 1);
        assert_eq!(rules.ticks_per_cell_at(9), 1);
    }
}
