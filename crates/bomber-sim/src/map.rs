//! Procedural map generation.
//!
//! Maps are generated from a width/height pair rather than authored, so there
//! are no map files to keep in sync. Generation is pure and seeded: the same
//! `(config, seed, players)` always produces byte-identical output, which is
//! what makes a match reproducible from its seed alone.

use bomber_proto::{Tile, TileGrid};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use serde::{Deserialize, Serialize};

/// Smallest board that still has an interior worth playing in.
pub const MIN_DIMENSION: u8 = 7;
/// Coordinates are `u8` on the wire, and a board this size is already absurd.
pub const MAX_DIMENSION: u8 = 63;

/// How the soft-block roll is mirrored across the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Symmetry {
    /// Every eligible cell rolled independently.
    None,
    /// Left half rolled, mirrored onto the right.
    MirrorX,
    /// Top-left quadrant rolled, mirrored onto the other three.
    #[default]
    Quad,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MapConfig {
    pub width: u8,
    pub height: u8,
    /// Fraction of eligible cells that become soft blocks.
    pub soft_block_density: f32,
    pub symmetry: Symmetry,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            width: 15,
            height: 13,
            soft_block_density: 0.75,
            symmetry: Symmetry::Quad,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedMap {
    pub grid: TileGrid,
    /// Indexed by player id.
    pub spawns: Vec<(u8, u8)>,
}

/// Round up to odd and clamp into the playable range.
///
/// Odd dimensions are not cosmetic: they are what makes the interior pillar
/// lattice line up with the solid border on every side.
fn normalize_dimension(v: u8) -> u8 {
    let v = v.clamp(MIN_DIMENSION, MAX_DIMENSION);
    if v % 2 == 0 {
        (v + 1).min(MAX_DIMENSION)
    } else {
        v
    }
}

/// Is this cell part of the fixed geometry (border ring or pillar lattice)?
fn is_structural(x: u8, y: u8, w: u8, h: u8) -> bool {
    x == 0 || y == 0 || x == w - 1 || y == h - 1 || (x % 2 == 0 && y % 2 == 0)
}

/// Spawn cells in player-id order: the four inner corners, then edge midpoints
/// if more than four players are ever supported.
fn spawn_cells(w: u8, h: u8, players: u8) -> Vec<(u8, u8)> {
    let mut cells = vec![
        (1, 1),
        (w - 2, 1),
        (1, h - 2),
        (w - 2, h - 2),
        // Midpoints are snapped to odd coordinates so they never land on a pillar.
        (w / 2 | 1, 1),
        (w / 2 | 1, h - 2),
        (1, h / 2 | 1),
        (w - 2, h / 2 | 1),
    ];
    cells.truncate(players.max(1) as usize);
    cells
}

/// The spawn cell plus the two neighbours pointing at the board centre.
///
/// Keeping this L-shape clear is what stops a bot from spawning walled in, or
/// from being trapped by the first bomb anyone drops.
fn safe_cells(spawn: (u8, u8), w: u8, h: u8) -> [(u8, u8); 3] {
    let (x, y) = spawn;
    let dx: i32 = if (x as i32) < (w as i32) / 2 { 1 } else { -1 };
    let dy: i32 = if (y as i32) < (h as i32) / 2 { 1 } else { -1 };
    [
        (x, y),
        ((x as i32 + dx) as u8, y),
        (x, (y as i32 + dy) as u8),
    ]
}

pub fn generate(cfg: &MapConfig, seed: u64, players: u8) -> GeneratedMap {
    let w = normalize_dimension(cfg.width);
    let h = normalize_dimension(cfg.height);

    let mut grid = TileGrid::new(w, h);
    for y in 0..h {
        for x in 0..w {
            if is_structural(x, y, w, h) {
                grid.set(x, y, Tile::Solid);
            }
        }
    }

    let spawns = spawn_cells(w, h, players);
    let mut protected = vec![false; w as usize * h as usize];
    for spawn in &spawns {
        for (sx, sy) in safe_cells(*spawn, w, h) {
            if grid.in_bounds(sx as i32, sy as i32) && !is_structural(sx, sy, w, h) {
                protected[sy as usize * w as usize + sx as usize] = true;
                grid.set(sx, sy, Tile::Empty);
            }
        }
    }

    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    let density = cfg.soft_block_density.clamp(0.0, 1.0) as f64;

    // Roll once per symmetry class, then paint every mirrored twin with the same
    // result. Without this a bot can simply be dealt a more open corner, and the
    // match stops being a comparison of bots.
    let (x_range, y_range) = match cfg.symmetry {
        Symmetry::None => (w, h),
        Symmetry::MirrorX => (w.div_ceil(2), h),
        Symmetry::Quad => (w.div_ceil(2), h.div_ceil(2)),
    };

    for y in 0..y_range {
        for x in 0..x_range {
            let soft = rng.gen_bool(density);
            if !soft {
                continue;
            }
            for (tx, ty) in mirrors(x, y, w, h, cfg.symmetry) {
                let idx = ty as usize * w as usize + tx as usize;
                if !is_structural(tx, ty, w, h) && !protected[idx] {
                    grid.set(tx, ty, Tile::Soft);
                }
            }
        }
    }

    let map = GeneratedMap { grid, spawns };
    debug_assert!(
        validate(&map).is_ok(),
        "generated an unplayable map: {:?}",
        validate(&map)
    );
    map
}

fn mirrors(x: u8, y: u8, w: u8, h: u8, sym: Symmetry) -> Vec<(u8, u8)> {
    let mx = w - 1 - x;
    let my = h - 1 - y;
    let mut out = match sym {
        Symmetry::None => vec![(x, y)],
        Symmetry::MirrorX => vec![(x, y), (mx, y)],
        Symmetry::Quad => vec![(x, y), (mx, y), (x, my), (mx, my)],
    };
    out.sort_unstable();
    out.dedup();
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    SpawnOnSolid { player: u8, x: u8, y: u8 },
    SpawnUnreachable { player: u8 },
    /// A spawn with no adjacent walkable cell is a death sentence on tick one.
    SpawnHasNoEscape { player: u8 },
}

/// Assert the invariants a match depends on.
///
/// The pillar lattice makes all of this true by construction, but a future
/// tweak to the generator must not be able to ship unfair maps silently.
pub fn validate(map: &GeneratedMap) -> Result<(), MapError> {
    let grid = &map.grid;

    for (id, &(x, y)) in map.spawns.iter().enumerate() {
        if grid.get(x, y) != Tile::Empty {
            return Err(MapError::SpawnOnSolid {
                player: id as u8,
                x,
                y,
            });
        }
        let has_escape = bomber_proto::Direction::ALL.iter().any(|d| {
            let (dx, dy) = d.delta();
            grid.get_or_solid(x as i32 + dx, y as i32 + dy) == Tile::Empty
        });
        if !has_escape {
            return Err(MapError::SpawnHasNoEscape { player: id as u8 });
        }
    }

    // Reachability ignores soft blocks: they are destructible, so they delay a
    // player rather than separating them.
    let Some(&start) = map.spawns.first() else {
        return Ok(());
    };
    let w = grid.width as usize;
    let mut seen = vec![false; w * grid.height as usize];
    let mut stack = vec![start];
    seen[start.1 as usize * w + start.0 as usize] = true;
    while let Some((x, y)) = stack.pop() {
        for d in bomber_proto::Direction::ALL {
            let (dx, dy) = d.delta();
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if grid.get_or_solid(nx, ny) == Tile::Solid {
                continue;
            }
            let idx = ny as usize * w + nx as usize;
            if !seen[idx] {
                seen[idx] = true;
                stack.push((nx as u8, ny as u8));
            }
        }
    }
    for (id, &(x, y)) in map.spawns.iter().enumerate() {
        if !seen[y as usize * w + x as usize] {
            return Err(MapError::SpawnUnreachable { player: id as u8 });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_are_forced_odd_and_clamped() {
        assert_eq!(normalize_dimension(0), 7);
        assert_eq!(normalize_dimension(4), 7);
        assert_eq!(normalize_dimension(14), 15);
        assert_eq!(normalize_dimension(15), 15);
        assert_eq!(normalize_dimension(255), 63);
    }

    #[test]
    fn border_is_solid_and_pillars_are_in_place() {
        let map = generate(&MapConfig::default(), 42, 4);
        let g = &map.grid;
        for x in 0..g.width {
            assert_eq!(g.get(x, 0), Tile::Solid);
            assert_eq!(g.get(x, g.height - 1), Tile::Solid);
        }
        for y in 0..g.height {
            assert_eq!(g.get(0, y), Tile::Solid);
            assert_eq!(g.get(g.width - 1, y), Tile::Solid);
        }
        for y in (2..g.height - 1).step_by(2) {
            for x in (2..g.width - 1).step_by(2) {
                assert_eq!(g.get(x, y), Tile::Solid, "pillar at {x},{y}");
            }
        }
    }

    #[test]
    fn spawns_are_clear_and_have_an_escape_route() {
        for seed in 0..50u64 {
            let map = generate(&MapConfig::default(), seed, 4);
            for &(x, y) in &map.spawns {
                assert_eq!(map.grid.get(x, y), Tile::Empty);
            }
            for spawn in &map.spawns {
                for (sx, sy) in safe_cells(*spawn, map.grid.width, map.grid.height) {
                    assert_eq!(map.grid.get(sx, sy), Tile::Empty, "seed {seed}");
                }
            }
        }
    }

    /// Density and size are moderator-tunable, so extreme settings must still
    /// produce a playable board rather than a trap.
    #[test]
    fn every_size_and_density_validates() {
        for w in (7..=31u8).step_by(2) {
            for h in (7..=31u8).step_by(2) {
                for density in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
                    for sym in [Symmetry::None, Symmetry::MirrorX, Symmetry::Quad] {
                        let cfg = MapConfig {
                            width: w,
                            height: h,
                            soft_block_density: density,
                            symmetry: sym,
                        };
                        let map = generate(&cfg, 7, 4);
                        assert_eq!(
                            validate(&map),
                            Ok(()),
                            "{w}x{h} density {density} {sym:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn same_seed_gives_an_identical_map() {
        let cfg = MapConfig::default();
        assert_eq!(generate(&cfg, 12345, 4), generate(&cfg, 12345, 4));
        assert_ne!(generate(&cfg, 12345, 4), generate(&cfg, 12346, 4));
    }

    #[test]
    fn quad_symmetry_actually_holds() {
        let cfg = MapConfig {
            symmetry: Symmetry::Quad,
            ..MapConfig::default()
        };
        let map = generate(&cfg, 99, 4);
        let g = &map.grid;
        let protected: Vec<(u8, u8)> = map
            .spawns
            .iter()
            .flat_map(|s| safe_cells(*s, g.width, g.height))
            .collect();
        for y in 0..g.height {
            for x in 0..g.width {
                let (mx, my) = (g.width - 1 - x, g.height - 1 - y);
                // Spawn safe zones are cleared after mirroring, so exempt them.
                if protected.contains(&(x, y)) || protected.contains(&(mx, y)) {
                    continue;
                }
                assert_eq!(g.get(x, y), g.get(mx, y), "x-mirror at {x},{y}");
                if !protected.contains(&(x, my)) {
                    assert_eq!(g.get(x, y), g.get(x, my), "y-mirror at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn full_density_still_leaves_spawns_open() {
        let cfg = MapConfig {
            soft_block_density: 1.0,
            ..MapConfig::default()
        };
        let map = generate(&cfg, 1, 4);
        assert_eq!(validate(&map), Ok(()));
    }
}
