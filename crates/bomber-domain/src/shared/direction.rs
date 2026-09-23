use serde::{Deserialize, Serialize};

/// A facing, and the four ways anything on the board can move.
///
/// The discriminants are part of the wire format and the order matches the
/// asset pack's `player_{down,up,left,right}` naming, so do not reorder them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Direction {
    Down = 0,
    Up = 1,
    Left = 2,
    Right = 3,
}

impl Direction {
    /// Iteration order for anything that fans out in all four directions --
    /// blast arms, reachability, neighbour scans. Fixed, because the simulation
    /// is deterministic and this order is observable in the output.
    pub const ALL: [Direction; 4] = [
        Direction::Down,
        Direction::Up,
        Direction::Left,
        Direction::Right,
    ];

    /// Unit step in cell coordinates. `y` grows downward.
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Direction::Down => (0, 1),
            Direction::Up => (0, -1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }

    pub const fn opposite(self) -> Direction {
        match self {
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}
