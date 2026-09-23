//! The invariants a match depends on.
//!
//! All of this holds by construction for the pillar lattice. It is checked
//! anyway, because a future change to generation must not be able to ship an
//! unfair or unplayable board silently.

use crate::board::{Board, Tile};
use crate::shared::{Cell, Direction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    SpawnOnSolid { player: u8, cell: Cell },
    /// A spawn with no adjacent walkable cell is a death sentence on tick one.
    SpawnHasNoEscape { player: u8 },
    SpawnUnreachable { player: u8 },
}

pub fn validate(board: &Board) -> Result<(), MapError> {
    for (id, &spawn) in board.spawns.iter().enumerate() {
        if board.grid.get(spawn) != Tile::Empty {
            return Err(MapError::SpawnOnSolid {
                player: id as u8,
                cell: spawn,
            });
        }
        let has_escape = Direction::ALL.iter().any(|d| {
            spawn
                .neighbour(*d)
                .is_some_and(|n| board.grid.get(n) == Tile::Empty)
        });
        if !has_escape {
            return Err(MapError::SpawnHasNoEscape { player: id as u8 });
        }
    }

    let Some(&start) = board.spawns.first() else {
        return Ok(());
    };
    let reachable = flood_fill(board, start);
    for (id, &spawn) in board.spawns.iter().enumerate() {
        if !reachable[board.grid.index_of(spawn)] {
            return Err(MapError::SpawnUnreachable { player: id as u8 });
        }
    }
    Ok(())
}

/// Reachability through solid geometry only.
///
/// Soft blocks count as passable: they are destructible, so they delay a player
/// rather than separating them.
fn flood_fill(board: &Board, start: Cell) -> Vec<bool> {
    let grid = &board.grid;
    let mut seen = vec![false; grid.width() as usize * grid.height() as usize];
    seen[grid.index_of(start)] = true;
    let mut stack = vec![start];
    while let Some(cell) = stack.pop() {
        for dir in Direction::ALL {
            let Some(next) = cell.neighbour(dir) else {
                continue;
            };
            if !grid.contains(next) || grid.get(next) == Tile::Solid {
                continue;
            }
            let idx = grid.index_of(next);
            if !seen[idx] {
                seen[idx] = true;
                stack.push(next);
            }
        }
    }
    seen
}
