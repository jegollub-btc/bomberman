use super::Rules;
use crate::shared::{Cell, Direction, PlayerId};

/// A player in a match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub id: PlayerId,
    pub alive: bool,
    /// The cell the player occupies.
    ///
    /// A step commits the moment it starts, so while moving this is already the
    /// *destination*. The player cannot be stopped or redirected mid-step, and
    /// the cell is theirs from the first tick of the move.
    pub cell: Cell,
    pub facing: Direction,
    /// Ticks elapsed in the current step; 0 means settled.
    pub move_progress: u8,
    /// Ticks the current step takes in total.
    pub move_total: u8,
    pub bombs_max: u8,
    pub bombs_active: u8,
    pub flame: u8,
    pub speed: u8,
    pub score: u16,
    /// Tick the player died on. Drives placement ordering at the end.
    pub died_at: Option<u32>,
}

impl Player {
    pub fn spawn(id: PlayerId, cell: Cell, rules: &Rules) -> Self {
        Player {
            id,
            alive: true,
            cell,
            facing: Direction::Down,
            move_progress: 0,
            move_total: 0,
            bombs_max: rules.start_bombs,
            bombs_active: 0,
            flame: rules.start_flame,
            speed: 0,
            score: 0,
            died_at: None,
        }
    }

    pub fn is_moving(&self) -> bool {
        self.move_progress > 0
    }

    pub fn can_place_bomb(&self) -> bool {
        self.alive && self.bombs_active < self.bombs_max
    }

    /// The cell the player is stepping away from, if mid-step. Rendering needs
    /// it; the rules do not.
    pub fn origin_cell(&self) -> Option<Cell> {
        self.is_moving()
            .then(|| self.cell.neighbour(self.facing.opposite()))
            .flatten()
    }
}
