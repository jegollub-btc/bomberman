//! The authoritative Bomberman simulation.
//!
//! This crate performs no I/O and holds no sockets. That is deliberate: a pure
//! `step()` over an explicit state is what makes the game deterministic, and
//! determinism is what buys replays, regression tests and the ability to settle
//! an argument about a match after the fact.
//!
//! Given the same seed and the same per-tick inputs, `GameState` produces the
//! same bytes on every machine, every run.

pub mod map;

use std::collections::VecDeque;

use bomber_proto::{
    Action, BombSnapshot, DeltaRecord, Direction, EndReason, FlameCell, Keyframe, PlayerResult,
    PlayerSnapshot, PowerupKind, PowerupSnapshot, Rules, Tile, TileGrid, NO_PLAYER,
};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

pub use map::{GeneratedMap, MapConfig, MapError, Symmetry};

/// Score awarded for blowing up a soft block.
pub const SCORE_BLOCK: u16 = 10;
/// Score awarded for killing another player.
pub const SCORE_KILL: u16 = 100;
/// Score awarded for surviving to the end of the match.
pub const SCORE_WIN: u16 = 500;
/// During sudden death, one more cell turns solid every this many ticks.
pub const SUDDEN_DEATH_PERIOD: u32 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub id: u8,
    pub alive: bool,
    /// The cell the player occupies. A step commits immediately, so while
    /// moving this is already the *destination* -- the player cannot be
    /// stopped or redirected mid-step.
    pub x: u8,
    pub y: u8,
    pub dir: Direction,
    /// Ticks elapsed in the current step; 0 means settled.
    pub move_progress: u8,
    /// Ticks the current step takes in total.
    pub move_total: u8,
    pub bombs_max: u8,
    pub bombs_active: u8,
    pub flame: u8,
    pub speed: u8,
    pub score: u16,
    /// Tick the player died on, for placement ordering.
    pub died_at: Option<u32>,
}

impl Player {
    fn new(id: u8, spawn: (u8, u8), rules: &Rules) -> Self {
        Player {
            id,
            alive: true,
            x: spawn.0,
            y: spawn.1,
            dir: Direction::Down,
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

    fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            id: self.id,
            alive: self.alive,
            moving: self.is_moving(),
            x: self.x,
            y: self.y,
            dir: self.dir,
            move_progress: self.move_progress,
            bombs_max: self.bombs_max,
            flame: self.flame,
            speed: self.speed,
            score: self.score,
        }
    }

    fn state_record(&self) -> DeltaRecord {
        DeltaRecord::PlayerState {
            id: self.id,
            alive: self.alive,
            moving: self.is_moving(),
            x: self.x,
            y: self.y,
            dir: self.dir,
            move_progress: self.move_progress,
        }
    }

    fn stats_record(&self) -> DeltaRecord {
        DeltaRecord::PlayerStats {
            id: self.id,
            bombs_max: self.bombs_max,
            flame: self.flame,
            speed: self.speed,
            score: self.score,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bomb {
    pub id: u16,
    pub owner: u8,
    pub x: u8,
    pub y: u8,
    pub fuse: u16,
    /// Blast radius captured when the bomb was placed, not when it detonates.
    pub flame: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Powerup {
    pub id: u16,
    pub x: u8,
    pub y: u8,
    pub kind: PowerupKind,
}

/// What `step` produced this tick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TickOutcome {
    /// Everything that changed, in the same vocabulary the wire protocol uses.
    /// The server forwards these verbatim as a `DELTA` frame.
    pub records: Vec<DeltaRecord>,
    pub ended: Option<MatchOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchOutcome {
    pub reason: EndReason,
    /// `NO_PLAYER` on a draw.
    pub winner: u8,
    pub results: Vec<PlayerResult>,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub tick: u32,
    pub grid: TileGrid,
    pub spawns: Vec<(u8, u8)>,
    pub rules: Rules,
    pub players: Vec<Player>,
    pub bombs: Vec<Bomb>,
    /// Remaining lethal ticks per cell; 0 means no flame. Indexed like the grid.
    pub flames: Vec<u8>,
    pub powerups: Vec<Powerup>,
    pub finished: bool,
    rng: Pcg64Mcg,
    next_bomb_id: u16,
    next_powerup_id: u16,
    /// Cells sudden death will wall off, outermost ring first.
    sudden_death_order: Vec<(u8, u8)>,
    sudden_death_filled: usize,
}

impl GameState {
    pub fn new(map: GeneratedMap, rules: Rules, seed: u64, player_count: u8) -> Self {
        let GeneratedMap { grid, spawns } = map;
        let players = (0..player_count)
            .map(|id| Player::new(id, spawns[id as usize], &rules))
            .collect();
        let flames = vec![0u8; grid.width as usize * grid.height as usize];
        let sudden_death_order = spiral_order(grid.width, grid.height);
        GameState {
            tick: 0,
            grid,
            spawns,
            rules,
            players,
            bombs: Vec::new(),
            flames,
            powerups: Vec::new(),
            finished: false,
            // Deriving the tick RNG from a mixed seed keeps it independent of
            // the map generator's stream, so retuning map generation does not
            // change power-up drops.
            rng: Pcg64Mcg::seed_from_u64(seed ^ 0x9E37_79B9_7F4A_7C15),
            next_bomb_id: 1,
            next_powerup_id: 1,
            sudden_death_order,
            sudden_death_filled: 0,
        }
    }

    fn idx(&self, x: u8, y: u8) -> usize {
        y as usize * self.grid.width as usize + x as usize
    }

    pub fn flame_at(&self, x: u8, y: u8) -> u8 {
        self.flames[self.idx(x, y)]
    }

    pub fn bomb_at(&self, x: u8, y: u8) -> Option<&Bomb> {
        self.bombs.iter().find(|b| b.x == x && b.y == y)
    }

    pub fn player_at(&self, x: u8, y: u8) -> Option<&Player> {
        self.players
            .iter()
            .find(|p| p.alive && p.x == x && p.y == y)
    }

    pub fn ticks_remaining(&self) -> u32 {
        self.rules.round_time_ticks.saturating_sub(self.tick)
    }

    pub fn alive_count(&self) -> usize {
        self.players.iter().filter(|p| p.alive).count()
    }

    pub fn keyframe(&self) -> Keyframe {
        Keyframe {
            tick: self.tick,
            grid: self.grid.clone(),
            players: self.players.iter().map(Player::snapshot).collect(),
            bombs: self
                .bombs
                .iter()
                .map(|b| BombSnapshot {
                    id: b.id,
                    owner: b.owner,
                    x: b.x,
                    y: b.y,
                    fuse_remaining: b.fuse,
                })
                .collect(),
            flames: (0..self.grid.height)
                .flat_map(|y| (0..self.grid.width).map(move |x| (x, y)))
                .filter_map(|(x, y)| {
                    let t = self.flame_at(x, y);
                    (t > 0).then_some(FlameCell {
                        x,
                        y,
                        ticks_remaining: t,
                    })
                })
                .collect(),
            powerups: self
                .powerups
                .iter()
                .map(|p| PowerupSnapshot {
                    id: p.id,
                    x: p.x,
                    y: p.y,
                    kind: p.kind,
                })
                .collect(),
            ticks_remaining: self.ticks_remaining(),
        }
    }

    /// Advance one tick.
    ///
    /// `inputs` is indexed by player id and holds the newest action received
    /// inside this tick's window, or `None` if the bot sent nothing (treated as
    /// `Noop`).
    ///
    /// The phase order below *is* the rules. Changing it changes outcomes, so it
    /// is fixed and documented rather than incidental.
    pub fn step(&mut self, inputs: &[Option<Action>]) -> TickOutcome {
        let mut out = TickOutcome::default();
        if self.finished {
            return out;
        }
        self.tick += 1;

        self.expire_flames(&mut out);
        self.detonate_bombs(&mut out);
        self.apply_inputs(inputs, &mut out);
        self.collect_powerups(&mut out);
        self.burn_players(&mut out);
        self.advance_sudden_death(&mut out);
        self.check_end(&mut out);

        out
    }

    // -- phase A: age out flames from previous ticks ------------------------

    fn expire_flames(&mut self, out: &mut TickOutcome) {
        for y in 0..self.grid.height {
            for x in 0..self.grid.width {
                let i = self.idx(x, y);
                if self.flames[i] == 0 {
                    continue;
                }
                self.flames[i] -= 1;
                if self.flames[i] == 0 {
                    out.records.push(DeltaRecord::FlameRemove { x, y });
                }
            }
        }
    }

    // -- phase B: fuses, chain reactions, blast ------------------------------

    fn detonate_bombs(&mut self, out: &mut TickOutcome) {
        for bomb in &mut self.bombs {
            bomb.fuse = bomb.fuse.saturating_sub(1);
        }

        // Every bomb whose fuse ran out seeds the chain. A bomb caught by another
        // bomb's flame is appended as the front expands, so an arbitrarily long
        // chain resolves inside this one tick -- players never see a chain
        // stutter across ticks.
        let mut queue: VecDeque<u16> = self
            .bombs
            .iter()
            .filter(|b| b.fuse == 0)
            .map(|b| b.id)
            .collect();
        if queue.is_empty() {
            return;
        }

        let mut detonated: Vec<u16> = Vec::new();
        while let Some(bomb_id) = queue.pop_front() {
            if detonated.contains(&bomb_id) {
                continue;
            }
            let Some(bomb) = self.bombs.iter().find(|b| b.id == bomb_id).cloned() else {
                continue;
            };
            detonated.push(bomb_id);

            let arms = self.blast_arms(&bomb);
            out.records.push(DeltaRecord::Explosion {
                x: bomb.x,
                y: bomb.y,
                up: arms[1],
                down: arms[0],
                left: arms[2],
                right: arms[3],
            });
            out.records.push(DeltaRecord::BombRemove { id: bomb.id });

            self.ignite(bomb.x, bomb.y, bomb.owner, out, &mut queue);
            for (di, dir) in Direction::ALL.iter().enumerate() {
                let (dx, dy) = dir.delta();
                for step in 1..=arms[di] as i32 {
                    let nx = bomb.x as i32 + dx * step;
                    let ny = bomb.y as i32 + dy * step;
                    self.ignite(nx as u8, ny as u8, bomb.owner, out, &mut queue);
                }
            }

            if let Some(owner) = self.players.iter_mut().find(|p| p.id == bomb.owner) {
                owner.bombs_active = owner.bombs_active.saturating_sub(1);
            }
        }

        self.bombs.retain(|b| !detonated.contains(&b.id));
    }

    /// How far the blast reaches in each direction, indexed like `Direction::ALL`.
    ///
    /// A soft block stops the blast *and* is destroyed; a solid wall stops it
    /// without being touched.
    fn blast_arms(&self, bomb: &Bomb) -> [u8; 4] {
        let mut arms = [0u8; 4];
        for (i, dir) in Direction::ALL.iter().enumerate() {
            let (dx, dy) = dir.delta();
            for step in 1..=bomb.flame as i32 {
                let nx = bomb.x as i32 + dx * step;
                let ny = bomb.y as i32 + dy * step;
                match self.grid.get_or_solid(nx, ny) {
                    Tile::Solid => break,
                    Tile::Soft => {
                        arms[i] = step as u8;
                        break;
                    }
                    Tile::Empty => arms[i] = step as u8,
                }
            }
        }
        arms
    }

    /// Set one cell alight, destroying what is there and chaining into bombs.
    fn ignite(
        &mut self,
        x: u8,
        y: u8,
        owner: u8,
        out: &mut TickOutcome,
        queue: &mut VecDeque<u16>,
    ) {
        let i = self.idx(x, y);

        if self.grid.get(x, y) == Tile::Soft {
            self.grid.set(x, y, Tile::Empty);
            out.records.push(DeltaRecord::TileSet {
                x,
                y,
                tile: Tile::Empty,
            });
            self.award(owner, SCORE_BLOCK, out);
            self.maybe_drop_powerup(x, y, out);
        }

        // A power-up caught in a blast is destroyed rather than banked.
        if let Some(pos) = self.powerups.iter().position(|p| p.x == x && p.y == y) {
            let removed = self.powerups.remove(pos);
            out.records.push(DeltaRecord::PowerupRemove {
                id: removed.id,
                taken_by: NO_PLAYER,
            });
        }

        let ticks = self.rules.flame_duration_ticks.min(u8::MAX as u16) as u8;
        if self.flames[i] < ticks {
            self.flames[i] = ticks;
        }
        out.records.push(DeltaRecord::FlameAdd { x, y, ticks });

        if let Some(bomb) = self.bombs.iter().find(|b| b.x == x && b.y == y) {
            queue.push_back(bomb.id);
        }
    }

    fn maybe_drop_powerup(&mut self, x: u8, y: u8, out: &mut TickOutcome) {
        if !self
            .rng
            .gen_bool(self.rules.powerup_chance_pct.min(100) as f64 / 100.0)
        {
            return;
        }
        let kind = PowerupKind::ALL[self.rng.gen_range(0..PowerupKind::ALL.len())];
        let id = self.next_powerup_id;
        self.next_powerup_id = self.next_powerup_id.wrapping_add(1).max(1);
        self.powerups.push(Powerup { id, x, y, kind });
        out.records
            .push(DeltaRecord::PowerupAdd { id, x, y, kind });
    }

    // -- phase C: bombs then movement ----------------------------------------

    fn apply_inputs(&mut self, inputs: &[Option<Action>], out: &mut TickOutcome) {
        // Bomb placement resolves for everyone before anyone moves, so dropping
        // a bomb and stepping away in the same action always works.
        for id in 0..self.players.len() {
            let action = inputs.get(id).copied().flatten().unwrap_or(Action::Noop);
            if action.places_bomb() {
                self.try_place_bomb(id, out);
            }
        }

        for id in 0..self.players.len() {
            let action = inputs.get(id).copied().flatten().unwrap_or(Action::Noop);
            self.advance_player(id, action.movement(), out);
        }
    }

    fn try_place_bomb(&mut self, index: usize, out: &mut TickOutcome) {
        let player = &self.players[index];
        if !player.alive || player.bombs_active >= player.bombs_max {
            return;
        }
        let (x, y, owner, flame) = (player.x, player.y, player.id, player.flame);
        if self.bomb_at(x, y).is_some() {
            return;
        }

        let id = self.next_bomb_id;
        self.next_bomb_id = self.next_bomb_id.wrapping_add(1).max(1);
        let fuse = self.rules.bomb_fuse_ticks;
        self.bombs.push(Bomb {
            id,
            owner,
            x,
            y,
            fuse,
            flame,
        });
        self.players[index].bombs_active += 1;
        out.records.push(DeltaRecord::BombAdd {
            id,
            owner,
            x,
            y,
            fuse,
        });
    }

    fn advance_player(&mut self, index: usize, intent: Option<Direction>, out: &mut TickOutcome) {
        if !self.players[index].alive {
            return;
        }

        if self.players[index].is_moving() {
            let p = &mut self.players[index];
            p.move_progress += 1;
            if p.move_progress >= p.move_total {
                p.move_progress = 0;
                p.move_total = 0;
            }
            out.records.push(self.players[index].state_record());
            return;
        }

        let Some(dir) = intent else { return };
        let (dx, dy) = dir.delta();
        let (nx, ny) = (
            self.players[index].x as i32 + dx,
            self.players[index].y as i32 + dy,
        );

        // Turning in place is always allowed, even into a wall, so a bot can aim
        // without having to find an open cell first.
        self.players[index].dir = dir;

        if self.can_enter(index, nx, ny) {
            let speed = self.players[index].speed;
            let total = self.rules.ticks_per_cell_at(speed);
            let p = &mut self.players[index];
            p.x = nx as u8;
            p.y = ny as u8;
            p.move_total = total;
            p.move_progress = 1;
        }
        out.records.push(self.players[index].state_record());
    }

    fn can_enter(&self, index: usize, nx: i32, ny: i32) -> bool {
        if self.grid.get_or_solid(nx, ny) != Tile::Empty {
            return false;
        }
        let (nx, ny) = (nx as u8, ny as u8);

        // A bomb is passable only for the player standing on it -- which, since
        // you can only be on a bomb you just placed, is exactly the classic
        // "walk off your own bomb" rule, with no extra state to track.
        if self.bomb_at(nx, ny).is_some() {
            return false;
        }

        // One player per cell. Players are resolved in id order, so on a
        // contested cell the lower id arrives first and the higher id is blocked.
        !self
            .players
            .iter()
            .enumerate()
            .any(|(i, p)| i != index && p.alive && p.x == nx && p.y == ny)
    }

    // -- phase D: pickups -----------------------------------------------------

    fn collect_powerups(&mut self, out: &mut TickOutcome) {
        let mut taken: Vec<(usize, usize)> = Vec::new();
        for (pi, player) in self.players.iter().enumerate() {
            if !player.alive {
                continue;
            }
            if let Some(ui) = self
                .powerups
                .iter()
                .position(|u| u.x == player.x && u.y == player.y)
            {
                taken.push((pi, ui));
            }
        }
        // Remove from the back so earlier indices stay valid.
        taken.sort_unstable_by_key(|&(_, ui)| std::cmp::Reverse(ui));
        for (pi, ui) in taken {
            let powerup = self.powerups.remove(ui);
            let rules = self.rules;
            let player = &mut self.players[pi];
            match powerup.kind {
                PowerupKind::ExtraBomb => player.bombs_max = player.bombs_max.saturating_add(1),
                PowerupKind::Flame => player.flame = (player.flame + 1).min(rules.max_flame),
                PowerupKind::Speed => player.speed = (player.speed + 1).min(rules.max_speed),
            }
            out.records.push(DeltaRecord::PowerupRemove {
                id: powerup.id,
                taken_by: player.id,
            });
            out.records.push(player.stats_record());
        }
    }

    // -- phase E: anyone standing in fire ------------------------------------

    fn burn_players(&mut self, out: &mut TickOutcome) {
        let victims: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.alive && self.flames[self.idx(p.x, p.y)] > 0)
            .map(|(i, _)| i)
            .collect();
        for i in victims {
            // Kill credit is not tracked per flame cell: with chain reactions the
            // answer is genuinely ambiguous, so deaths are self-credited unless a
            // future rule says otherwise.
            self.kill(i, NO_PLAYER, out);
        }
    }

    fn kill(&mut self, index: usize, killer: u8, out: &mut TickOutcome) {
        let tick = self.tick;
        let player = &mut self.players[index];
        if !player.alive {
            return;
        }
        player.alive = false;
        player.died_at = Some(tick);
        player.move_progress = 0;
        let id = player.id;
        out.records.push(DeltaRecord::PlayerDeath { id, killer });
        out.records.push(self.players[index].state_record());

        if killer != NO_PLAYER && killer != id {
            self.award(killer, SCORE_KILL, out);
        }
    }

    fn award(&mut self, player_id: u8, points: u16, out: &mut TickOutcome) {
        if let Some(p) = self.players.iter_mut().find(|p| p.id == player_id) {
            p.score = p.score.saturating_add(points);
            let rec = p.stats_record();
            out.records.push(rec);
        }
    }

    // -- phase F: sudden death -----------------------------------------------

    fn advance_sudden_death(&mut self, out: &mut TickOutcome) {
        if self.tick < self.rules.sudden_death_tick {
            return;
        }
        let elapsed = self.tick - self.rules.sudden_death_tick;
        if elapsed % SUDDEN_DEATH_PERIOD != 0 {
            return;
        }
        let Some(&(x, y)) = self.sudden_death_order.get(self.sudden_death_filled) else {
            return;
        };
        self.sudden_death_filled += 1;

        self.grid.set(x, y, Tile::Solid);
        out.records.push(DeltaRecord::TileSet {
            x,
            y,
            tile: Tile::Solid,
        });
        self.bombs.retain(|b| !(b.x == x && b.y == y));
        self.powerups.retain(|p| !(p.x == x && p.y == y));

        if let Some(i) = self
            .players
            .iter()
            .position(|p| p.alive && p.x == x && p.y == y)
        {
            self.kill(i, NO_PLAYER, out);
        }
    }

    // -- phase G: is it over? -------------------------------------------------

    fn check_end(&mut self, out: &mut TickOutcome) {
        let alive = self.alive_count();
        let timeout = self.tick >= self.rules.round_time_ticks;
        // A normal match ends when one player is left. A solo match (practice,
        // tests, a single bot warming up) would otherwise end on tick one, so it
        // runs until that player dies instead.
        let survivors_needed = if self.players.len() > 1 { 2 } else { 1 };
        if alive >= survivors_needed && !timeout {
            return;
        }

        let reason = if alive < survivors_needed {
            EndReason::LastStanding
        } else {
            EndReason::Timeout
        };

        for p in self.players.iter_mut().filter(|p| p.alive) {
            p.score = p.score.saturating_add(SCORE_WIN);
        }
        for i in 0..self.players.len() {
            let rec = self.players[i].stats_record();
            out.records.push(rec);
        }

        self.finished = true;
        out.ended = Some(self.outcome(reason));
    }

    /// Rank players: survivors first, then by how long they lasted, then by score.
    pub fn outcome(&self, reason: EndReason) -> MatchOutcome {
        let mut ranked: Vec<&Player> = self.players.iter().collect();
        ranked.sort_by_key(|p| {
            (
                !p.alive,
                std::cmp::Reverse(p.died_at.unwrap_or(u32::MAX)),
                std::cmp::Reverse(p.score),
            )
        });

        let mut results = Vec::with_capacity(ranked.len());
        let mut placement = 0u8;
        let mut prev_key: Option<(bool, u32, u16)> = None;
        for (i, p) in ranked.iter().enumerate() {
            let key = (p.alive, p.died_at.unwrap_or(u32::MAX), p.score);
            // Tied players share a placement; the next distinct player skips
            // ahead, so placements read like a leaderboard.
            if prev_key != Some(key) {
                placement = i as u8 + 1;
                prev_key = Some(key);
            }
            results.push(PlayerResult {
                id: p.id,
                placement,
                score: p.score,
            });
        }
        results.sort_by_key(|r| r.id);

        let survivors: Vec<u8> = self
            .players
            .iter()
            .filter(|p| p.alive)
            .map(|p| p.id)
            .collect();
        let winner = if survivors.len() == 1 {
            survivors[0]
        } else if survivors.is_empty() {
            NO_PLAYER
        } else {
            // Timeout with several alive: highest score wins, ties draw.
            let best = survivors
                .iter()
                .map(|id| self.players[*id as usize].score)
                .max()
                .unwrap_or(0);
            let top: Vec<u8> = survivors
                .into_iter()
                .filter(|id| self.players[*id as usize].score == best)
                .collect();
            if top.len() == 1 {
                top[0]
            } else {
                NO_PLAYER
            }
        };

        MatchOutcome {
            reason,
            winner,
            results,
        }
    }

    /// A cheap order-sensitive hash of everything that defines the state.
    ///
    /// Two runs that agree here agree on the match; the determinism test is
    /// built on this.
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x1000_0000_01b3);
        };
        feed(self.tick as u64);
        for tile in &self.grid.cells {
            feed(*tile as u64);
        }
        for p in &self.players {
            for v in [
                p.id as u64,
                p.alive as u64,
                p.x as u64,
                p.y as u64,
                p.dir as u64,
                p.move_progress as u64,
                p.bombs_max as u64,
                p.bombs_active as u64,
                p.flame as u64,
                p.speed as u64,
                p.score as u64,
            ] {
                feed(v);
            }
        }
        for b in &self.bombs {
            for v in [
                b.id as u64,
                b.owner as u64,
                b.x as u64,
                b.y as u64,
                b.fuse as u64,
                b.flame as u64,
            ] {
                feed(v);
            }
        }
        for f in &self.flames {
            feed(*f as u64);
        }
        for p in &self.powerups {
            for v in [p.id as u64, p.x as u64, p.y as u64, p.kind as u64] {
                feed(v);
            }
        }
        h
    }
}

/// Interior cells ordered outermost ring first, clockwise -- the order sudden
/// death walls them off in.
fn spiral_order(width: u8, height: u8) -> Vec<(u8, u8)> {
    let (mut top, mut bottom) = (1i32, height as i32 - 2);
    let (mut left, mut right) = (1i32, width as i32 - 2);
    let mut out = Vec::new();
    while top <= bottom && left <= right {
        for x in left..=right {
            out.push((x as u8, top as u8));
        }
        for y in (top + 1)..=bottom {
            out.push((right as u8, y as u8));
        }
        if top < bottom {
            for x in (left..right).rev() {
                out.push((x as u8, bottom as u8));
            }
        }
        if left < right {
            for y in ((top + 1)..bottom).rev() {
                out.push((left as u8, y as u8));
            }
        }
        top += 1;
        bottom -= 1;
        left += 1;
        right -= 1;
    }
    out
}
