//! The match aggregate.
//!
//! [`GameState`] owns everything that changes during a match and is the only
//! place allowed to mutate it. The tick phases in [`super::phases`] are given
//! `&mut GameState` and do one job each; this module is the orchestration and
//! the shared helpers, not the rules.

use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;

use crate::board::Board;
use crate::shared::{Cell, PlayerId};

use super::phases;
use super::{
    Bomb, BombId, Event, FlameField, Intent, MatchOutcome, Player, Powerup, PowerupId, Rules,
};

/// What a tick produced.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TickOutcome {
    pub events: Vec<Event>,
    /// Set on the tick the match ends, and only on that tick.
    pub ended: Option<MatchOutcome>,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub tick: u32,
    pub board: Board,
    pub rules: Rules,
    pub players: Vec<Player>,
    pub bombs: Vec<Bomb>,
    pub flames: FlameField,
    pub powerups: Vec<Powerup>,
    pub finished: bool,

    pub(crate) rng: Pcg64Mcg,
    pub(crate) closing_order: Vec<Cell>,
    pub(crate) closed_count: usize,
    next_bomb_id: BombId,
    next_powerup_id: PowerupId,
}

impl GameState {
    /// `participants` are the seats actually taking part, in spawn order.
    ///
    /// Seats need not be contiguous: kicking the middle bot of three must not
    /// renumber the survivors, because the id they put in byte 0 of every
    /// packet is the id they were assigned at the door.
    pub fn new(board: Board, rules: Rules, seed: u64, participants: &[PlayerId]) -> Self {
        let players = participants
            .iter()
            .enumerate()
            .map(|(spawn_index, &id)| Player::spawn(id, board.spawns[spawn_index], &rules))
            .collect();
        let flames = FlameField::for_grid(&board.grid);
        let closing_order = phases::sudden_death::closing_order(board.width(), board.height());

        GameState {
            tick: 0,
            board,
            rules,
            players,
            bombs: Vec::new(),
            flames,
            powerups: Vec::new(),
            finished: false,
            // Mixing the seed keeps the tick RNG independent of the map
            // generator's stream, so retuning map generation does not silently
            // change power-up drops.
            rng: Pcg64Mcg::seed_from_u64(seed ^ 0x9E37_79B9_7F4A_7C15),
            closing_order,
            closed_count: 0,
            next_bomb_id: 1,
            next_powerup_id: 1,
        }
    }

    /// Advance exactly one tick.
    ///
    /// `intents` is indexed by **player id**, not by position in `players` --
    /// seats can be sparse. `None` means the bot sent nothing,
    /// which is treated the same as an explicit idle -- a bot is never required
    /// to transmit on a tick where it has nothing to say.
    pub fn step(&mut self, intents: &[Option<Intent>]) -> TickOutcome {
        let mut outcome = TickOutcome::default();
        if self.finished {
            return outcome;
        }
        self.tick += 1;
        let events = &mut outcome.events;

        phases::flame_decay::run(&mut self.flames, events);
        phases::detonation::run(self, events);
        phases::placement::run(self, intents, events);
        phases::movement::run(self, intents, events);
        phases::pickups::run(self, events);
        phases::casualties::run(self, events);
        phases::sudden_death::run(self, events);
        outcome.ended = phases::completion::run(self, events);

        outcome
    }

    // -- queries -------------------------------------------------------------

    pub fn player(&self, id: PlayerId) -> Option<&Player> {
        self.players.iter().find(|p| p.id == id)
    }

    pub fn player_mut(&mut self, id: PlayerId) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.id == id)
    }

    pub fn player_at(&self, cell: Cell) -> Option<&Player> {
        self.players.iter().find(|p| p.alive && p.cell == cell)
    }

    pub fn bomb_at(&self, cell: Cell) -> Option<&Bomb> {
        self.bombs.iter().find(|b| b.cell == cell)
    }

    pub fn powerup_at(&self, cell: Cell) -> Option<&Powerup> {
        self.powerups.iter().find(|p| p.cell == cell)
    }

    pub fn alive_count(&self) -> usize {
        self.players.iter().filter(|p| p.alive).count()
    }

    pub fn ticks_remaining(&self) -> u32 {
        self.rules.round_time_ticks.saturating_sub(self.tick)
    }

    pub fn outcome(&self, reason: super::EndReason) -> MatchOutcome {
        phases::completion::outcome(self, reason)
    }

    // -- mutations shared by phases ------------------------------------------

    pub(crate) fn next_bomb_id(&mut self) -> BombId {
        let id = self.next_bomb_id;
        self.next_bomb_id = self.next_bomb_id.wrapping_add(1).max(1);
        id
    }

    pub(crate) fn next_powerup_id(&mut self) -> PowerupId {
        let id = self.next_powerup_id;
        self.next_powerup_id = self.next_powerup_id.wrapping_add(1).max(1);
        id
    }

    pub(crate) fn award(&mut self, id: PlayerId, points: u16, events: &mut Vec<Event>) {
        if let Some(player) = self.player_mut(id) {
            player.score = player.score.saturating_add(points);
            let event = player.stats_event();
            events.push(event);
        }
    }

    pub(crate) fn kill(&mut self, index: usize, killer: Option<PlayerId>, events: &mut Vec<Event>) {
        let tick = self.tick;
        let player = &mut self.players[index];
        if !player.alive {
            return;
        }
        player.alive = false;
        player.died_at = Some(tick);
        player.move_progress = 0;
        player.move_total = 0;
        let id = player.id;

        events.push(Event::PlayerDied { id, killer });
        events.push(self.players[index].state_event());

        if let Some(killer) = killer.filter(|k| *k != id) {
            self.award(killer, super::scoring::SCORE_KILL, events);
        }
    }

    /// An order-sensitive hash of everything that defines the state.
    ///
    /// Two runs that agree here agree on the match. The determinism tests are
    /// built on this, and so is any future desync check between server and bot.
    pub fn state_hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |value: u64| {
            hash ^= value;
            hash = hash.wrapping_mul(0x1000_0000_01b3);
        };

        feed(self.tick as u64);
        for tile in self.board.grid.cells() {
            feed(tile.code() as u64);
        }
        for p in &self.players {
            for value in [
                p.id.raw() as u64,
                p.alive as u64,
                p.cell.x as u64,
                p.cell.y as u64,
                p.facing as u64,
                p.move_progress as u64,
                p.bombs_max as u64,
                p.bombs_active as u64,
                p.flame as u64,
                p.speed as u64,
                p.score as u64,
            ] {
                feed(value);
            }
        }
        for b in &self.bombs {
            for value in [
                b.id as u64,
                b.owner.raw() as u64,
                b.cell.x as u64,
                b.cell.y as u64,
                b.fuse as u64,
                b.flame as u64,
            ] {
                feed(value);
            }
        }
        for ticks in self.flames.raw() {
            feed(*ticks as u64);
        }
        for p in &self.powerups {
            for value in [p.id as u64, p.cell.x as u64, p.cell.y as u64, p.kind as u64] {
                feed(value);
            }
        }
        hash
    }
}

impl Player {
    pub(crate) fn state_event(&self) -> Event {
        Event::PlayerStateChanged {
            id: self.id,
            alive: self.alive,
            moving: self.is_moving(),
            cell: self.cell,
            facing: self.facing,
            move_progress: self.move_progress,
            move_total: self.move_total,
        }
    }

    pub(crate) fn stats_event(&self) -> Event {
        Event::PlayerStatsChanged {
            id: self.id,
            bombs_max: self.bombs_max,
            flame: self.flame,
            speed: self.speed,
            score: self.score,
        }
    }
}
