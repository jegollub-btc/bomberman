//! The board a match is played on must be playable and fair.

mod common;

use bomber_domain::board::{generate, validate, GenerationConfig, Symmetry, Tile};
use bomber_domain::shared::Cell;

#[test]
fn border_is_solid_and_the_pillar_lattice_is_in_place() {
    let board = generate(&GenerationConfig::default(), 42, 4);
    let grid = &board.grid;

    for x in 0..grid.width() {
        assert_eq!(grid.get(Cell::new(x, 0)), Tile::Solid);
        assert_eq!(grid.get(Cell::new(x, grid.height() - 1)), Tile::Solid);
    }
    for y in 0..grid.height() {
        assert_eq!(grid.get(Cell::new(0, y)), Tile::Solid);
        assert_eq!(grid.get(Cell::new(grid.width() - 1, y)), Tile::Solid);
    }
    for y in (2..grid.height() - 1).step_by(2) {
        for x in (2..grid.width() - 1).step_by(2) {
            assert_eq!(grid.get(Cell::new(x, y)), Tile::Solid, "pillar at {x},{y}");
        }
    }
}

#[test]
fn spawns_are_clear_and_have_an_escape_route() {
    for seed in 0..50u64 {
        let board = generate(&GenerationConfig::default(), seed, 4);
        for &spawn in &board.spawns {
            assert_eq!(board.grid.get(spawn), Tile::Empty, "seed {seed}");
        }
        assert_eq!(validate(&board), Ok(()), "seed {seed}");
    }
}

/// Size and density are moderator-tunable, so extreme settings must still
/// produce a playable board rather than a trap.
#[test]
fn every_size_and_density_validates() {
    for width in (7..=31u8).step_by(2) {
        for height in (7..=31u8).step_by(2) {
            for density in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
                for symmetry in [Symmetry::None, Symmetry::MirrorX, Symmetry::Quad] {
                    let config = GenerationConfig {
                        width,
                        height,
                        soft_block_density: density,
                        symmetry,
                    };
                    let board = generate(&config, 7, 4);
                    assert_eq!(
                        validate(&board),
                        Ok(()),
                        "{width}x{height} density {density} {symmetry:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn dimensions_are_forced_odd() {
    let config = GenerationConfig {
        width: 14,
        height: 20,
        ..GenerationConfig::default()
    };
    let board = generate(&config, 1, 4);
    assert_eq!((board.width(), board.height()), (15, 21));
}

#[test]
fn the_same_seed_gives_an_identical_board() {
    let config = GenerationConfig::default();
    assert_eq!(generate(&config, 12345, 4), generate(&config, 12345, 4));
    assert_ne!(generate(&config, 12345, 4), generate(&config, 12346, 4));
}

/// Without this, one bot is simply dealt a more open corner and match results
/// stop being a comparison of bots.
#[test]
fn quad_symmetry_actually_holds_outside_the_spawn_zones() {
    let config = GenerationConfig {
        symmetry: Symmetry::Quad,
        ..GenerationConfig::default()
    };
    let board = generate(&config, 99, 4);
    let grid = &board.grid;

    // Spawn safe zones are cleared after mirroring, so exempt cells near them.
    let near_spawn = |cell: Cell| {
        board.spawns.iter().any(|s| {
            let dx = (s.x as i32 - cell.x as i32).abs();
            let dy = (s.y as i32 - cell.y as i32).abs();
            dx + dy <= 1
        })
    };

    for cell in grid.iter_cells() {
        let mirrored = Cell::new(grid.width() - 1 - cell.x, cell.y);
        if near_spawn(cell) || near_spawn(mirrored) {
            continue;
        }
        assert_eq!(
            grid.get(cell),
            grid.get(mirrored),
            "x-mirror broken at {cell:?}"
        );
    }
}
