use bomber_domain::board::{generate, Board};
use bomber_domain::game::{EndReason, Event, GameState, MatchOutcome};
use bomber_domain::lobby::{AdmissionError, Lobby, LobbyState};
use bomber_domain::shared::PlayerId;
use bomber_protocol::{
    Action, Assigned, LobbyStatus, MatchEnd, MatchInit, ServerFrame, MAX_PLAYERS, PROTOCOL_VERSION,
    TICK_RATE,
};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

use crate::moderation::{CommandOutcome, ModeratorCommand};

use super::delivery::Delivery;
use super::inputs::{InputSlots, Rejection};
use super::presence::Presence;
use super::snapshot::{SeatView, SessionSnapshot};
use super::{SessionConfig, MapSettings};

/// What one tick produced, for the adapters to deliver.
///
/// The session never sends anything itself. It returns what *should* be sent
/// and lets the caller decide how -- which is why a whole match can be played
/// out in a unit test without a socket in sight.
#[derive(Debug, Default, Clone)]
pub struct TickReport {
    /// Frames addressed to one bot, because their content differs per player.
    pub unicast: Vec<(PlayerId, ServerFrame)>,
    /// Frames every connected bot gets verbatim.
    pub broadcast: Vec<ServerFrame>,
    /// Domain events from this tick, for the spectator feed.
    pub events: Vec<Event>,
    pub match_started: bool,
    pub match_ended: Option<MatchOutcome>,
    /// The lobby roster or state changed and watchers should be refreshed.
    pub lobby_changed: bool,
    /// Whether the simulation actually advanced.
    pub simulated: bool,
}

pub struct ArenaSession {
    config: SessionConfig,
    lobby: Lobby,
    game: Option<GameState>,
    board: Option<Board>,
    inputs: InputSlots,
    presence: Presence,
    delivery: Delivery,
    paused: bool,
    session_tick: u32,
    match_id: u32,
    match_init: Option<MatchInit>,
    init_repeats_left: u32,
    last_outcome: Option<MatchOutcome>,
    seeds: Pcg64Mcg,
    lobby_dirty: bool,
}

impl ArenaSession {
    pub fn new(config: SessionConfig, entropy_seed: u64) -> Self {
        let seats = config.max_players.min(MAX_PLAYERS as u8);
        ArenaSession {
            lobby: Lobby::new(seats, config.min_players),
            inputs: InputSlots::with_seats(seats as usize),
            presence: Presence::with_seats(seats as usize),
            delivery: Delivery::new(config.keyframe_interval_ticks),
            config,
            game: None,
            board: None,
            paused: false,
            session_tick: 0,
            match_id: 0,
            match_init: None,
            init_repeats_left: 0,
            last_outcome: None,
            seeds: Pcg64Mcg::seed_from_u64(entropy_seed),
            lobby_dirty: true,
        }
    }

    // -- queries -------------------------------------------------------------

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    pub fn game(&self) -> Option<&GameState> {
        self.game.as_ref()
    }

    pub fn board(&self) -> Option<&Board> {
        self.board.as_ref()
    }

    pub fn state(&self) -> LobbyState {
        self.lobby.state()
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn session_tick(&self) -> u32 {
        self.session_tick
    }

    pub fn match_id(&self) -> u32 {
        self.match_id
    }

    pub fn last_outcome(&self) -> Option<&MatchOutcome> {
        self.last_outcome.as_ref()
    }

    pub fn is_seated(&self, player: PlayerId) -> bool {
        self.lobby.is_occupied(player)
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        let now = self.session_tick;
        let seats = self
            .lobby
            .seats()
            .iter()
            .map(|seat| {
                let stale_ticks = self.presence.staleness(seat.id, now);
                SeatView {
                    id: seat.id,
                    name: seat.name.clone(),
                    connected: seat.occupied,
                    presence: self.presence.of(seat.id, self.inputs.loss_pct(seat.id)),
                    stale_ticks,
                    stale: seat.occupied
                        && stale_ticks.is_none_or(|t| t > self.config.stale_after_ticks),
                }
            })
            .collect();

        SessionSnapshot {
            state: self.lobby.state(),
            paused: self.paused,
            can_start: !self.paused && self.lobby.can_start(),
            min_players: self.lobby.min_players(),
            max_players: self.lobby.max_players(),
            countdown_ticks: self.lobby.countdown_ticks(),
            map: self.config.map,
            seats,
            session_tick: now,
            board: self.board.clone(),
        }
    }

    // -- admission -----------------------------------------------------------

    /// Seat a new bot. The caller binds the returned id to whatever transport
    /// the hello arrived on, and is responsible for rejecting later packets
    /// that claim the id from anywhere else.
    pub fn admit(&mut self) -> Result<PlayerId, AdmissionError> {
        let player = self.lobby.admit()?;
        self.inputs.reset_seat(player);
        self.presence.reset_seat(player);
        self.lobby_dirty = true;
        Ok(player)
    }

    pub fn release(&mut self, player: PlayerId) {
        self.lobby.release(player);
        self.inputs.reset_seat(player);
        self.presence.reset_seat(player);
        self.lobby_dirty = true;
    }

    /// Record a bot's intent for this tick.
    pub fn submit(&mut self, player: PlayerId, action: Action, seq: u8) -> Result<(), Rejection> {
        if !self.lobby.is_occupied(player) {
            return Err(Rejection::UnknownSeat);
        }
        self.presence.record(player, self.session_tick);
        self.inputs.submit(player, action, seq)
    }

    // -- frames the adapter sends outside the tick ---------------------------

    pub fn assigned_frame(&self, player: PlayerId) -> ServerFrame {
        ServerFrame::Assigned(Assigned {
            tick: self.session_tick,
            protocol_version: PROTOCOL_VERSION,
            player_id: player,
            tick_rate: TICK_RATE,
            max_players: self.lobby.max_players(),
        })
    }

    /// The match-start frame for one player, if a match is under way.
    ///
    /// Re-sending this is the documented recovery path for a bot that missed
    /// it: with no acknowledgements, asking again is the only option it has.
    pub fn match_init_frame_for(&self, player: PlayerId) -> Option<ServerFrame> {
        let init = self.match_init.as_ref()?;
        Some(ServerFrame::MatchInit(MatchInit {
            tick: self.session_tick,
            your_player_id: player,
            ..init.clone()
        }))
    }

    pub fn lobby_status_frame(&self) -> ServerFrame {
        ServerFrame::LobbyStatus(LobbyStatus {
            tick: self.session_tick,
            state: self.lobby.state(),
            players_connected: self.lobby.occupied_count(),
            max_players: self.lobby.max_players(),
            slot_mask: self.lobby.occupancy_mask(),
            countdown_ticks: self.lobby.countdown_ticks(),
        })
    }

    pub fn take_lobby_dirty(&mut self) -> bool {
        std::mem::take(&mut self.lobby_dirty)
    }

    // -- the tick ------------------------------------------------------------

    pub fn tick(&mut self) -> TickReport {
        let mut report = TickReport::default();
        if self.paused {
            report.lobby_changed = self.take_lobby_dirty();
            return report;
        }

        self.session_tick += 1;
        self.presence.advance(self.session_tick);

        match self.lobby.state() {
            LobbyState::Open | LobbyState::Locked | LobbyState::MatchOver => {
                self.heartbeat(&mut report, self.config.lobby_status_interval_ticks);
            }
            LobbyState::Countdown => {
                // Faster while counting down: the number on screen is changing
                // and a bot may want to get ready.
                self.heartbeat(&mut report, 10);
                if self.lobby.tick_countdown() {
                    self.begin_match(&mut report);
                }
            }
            LobbyState::Running => self.advance_match(&mut report),
        }

        report.lobby_changed |= self.take_lobby_dirty();
        report
    }

    fn heartbeat(&self, report: &mut TickReport, interval: u32) {
        if self.session_tick.is_multiple_of(interval.max(1)) {
            report.broadcast.push(self.lobby_status_frame());
        }
    }

    fn begin_match(&mut self, report: &mut TickReport) {
        let participants = self.lobby.participants();
        if participants.is_empty() {
            self.lobby.reset();
            self.lobby_dirty = true;
            return;
        }

        let seed = match self.config.map.seed {
            0 => self.seeds.gen(),
            pinned => pinned,
        };
        let board = generate(
            &self.config.map.generation,
            seed,
            participants.len() as u8,
        );

        self.match_id = self.match_id.wrapping_add(1);
        self.match_init = Some(MatchInit {
            tick: self.session_tick,
            protocol_version: PROTOCOL_VERSION,
            match_id: self.match_id,
            seed,
            // Overwritten per recipient; this frame is never sent as-is.
            your_player_id: participants[0],
            grid: board.grid.clone(),
            spawns: board.spawns.clone(),
            rules: self.config.rules,
        });
        self.init_repeats_left = self.config.match_init_repeats;

        let game = GameState::new(board.clone(), self.config.rules, seed, &participants);
        self.board = Some(board);
        self.game = Some(game);
        self.inputs.clear();
        self.delivery.restart();
        self.lobby.begin_match();
        self.last_outcome = None;
        self.lobby_dirty = true;

        self.repeat_match_init(report);
        // Clients hold nothing yet, so the first frame has to be complete --
        // and it has to go through , so the keyframe cadence is
        // measured from the start of the match rather than from the first
        // simulated tick.
        let game = self.game.as_ref().expect("just created");
        let opening = self.delivery.frame_for(game, &[]);
        report.broadcast.push(opening);
        report.match_started = true;
    }

    fn repeat_match_init(&mut self, report: &mut TickReport) {
        if self.init_repeats_left == 0 {
            return;
        }
        self.init_repeats_left -= 1;
        for seat in self.lobby.seats() {
            if !seat.occupied {
                continue;
            }
            if let Some(frame) = self.match_init_frame_for(seat.id) {
                report.unicast.push((seat.id, frame));
            }
        }
    }

    fn advance_match(&mut self, report: &mut TickReport) {
        let intents = self.inputs.take();
        let Some(game) = self.game.as_mut() else {
            return;
        };
        let outcome = game.step(&intents);
        report.simulated = true;
        report.events = outcome.events;

        self.repeat_match_init(report);

        // Disjoint field borrows: `delivery` and `game` are separate fields.
        let game = self.game.as_ref().expect("checked above");
        let frame = self.delivery.frame_for(game, &report.events);
        let tick = game.tick;
        report.broadcast.push(frame);

        if let Some(result) = outcome.ended {
            report
                .broadcast
                .push(ServerFrame::MatchEnd(MatchEnd::from_outcome(tick, &result)));
            self.lobby.finish_match();
            self.last_outcome = Some(result.clone());
            report.match_ended = Some(result);
            self.lobby_dirty = true;
        }
    }

    // -- moderation ----------------------------------------------------------

    pub fn execute(&mut self, command: ModeratorCommand) -> CommandOutcome {
        let outcome = self.dispatch(command);
        if outcome.is_accepted() {
            self.lobby_dirty = true;
        }
        outcome
    }

    fn dispatch(&mut self, command: ModeratorCommand) -> CommandOutcome {
        match command {
            ModeratorCommand::Start => self.start(),
            ModeratorCommand::Pause => self.pause(),
            ModeratorCommand::Resume => self.resume(),
            ModeratorCommand::End => self.end_match(),
            ModeratorCommand::Reset => self.reset(),
            ModeratorCommand::Lock => {
                self.lobby.lock();
                CommandOutcome::Accepted
            }
            ModeratorCommand::Unlock => {
                self.lobby.unlock();
                CommandOutcome::Accepted
            }
            ModeratorCommand::Kick(player) => self.kick(player),
            ModeratorCommand::Rename { player, name } => {
                if !self.lobby.seats().iter().any(|s| s.id == player) {
                    return CommandOutcome::rejected(format!("no seat {player}"));
                }
                self.lobby.rename(player, name);
                CommandOutcome::Accepted
            }
            ModeratorCommand::ConfigureMap(patch) => {
                if self.lobby.state().is_playing() {
                    return CommandOutcome::rejected(
                        "cannot change the map while a match is running",
                    );
                }
                self.config.map.apply(patch);
                CommandOutcome::Accepted
            }
            ModeratorCommand::PreviewMap => {
                let (board, seed) = self.preview();
                CommandOutcome::Preview {
                    board: Box::new(board),
                    seed,
                }
            }
        }
    }

    fn start(&mut self) -> CommandOutcome {
        if self.paused {
            return CommandOutcome::rejected("resume before starting");
        }
        match self.lobby.begin_countdown(self.config.countdown_ticks) {
            Ok(()) => CommandOutcome::Accepted,
            Err(bomber_domain::lobby::StartError::AlreadyRunning) => {
                CommandOutcome::rejected("a match is already starting or running")
            }
            Err(bomber_domain::lobby::StartError::NotEnoughPlayers { have, need }) => {
                CommandOutcome::rejected(format!("need at least {need} players, have {have}"))
            }
        }
    }

    fn pause(&mut self) -> CommandOutcome {
        if self.paused {
            return CommandOutcome::rejected("already paused");
        }
        self.paused = true;
        CommandOutcome::Accepted
    }

    fn resume(&mut self) -> CommandOutcome {
        if !self.paused {
            return CommandOutcome::rejected("not paused");
        }
        self.paused = false;
        CommandOutcome::Accepted
    }

    /// Stop the running match, leaving the results up.
    fn end_match(&mut self) -> CommandOutcome {
        if !matches!(
            self.lobby.state(),
            LobbyState::Running | LobbyState::Countdown
        ) {
            return CommandOutcome::rejected("no match to end");
        }
        if let Some(game) = self.game.as_mut() {
            game.finished = true;
            self.last_outcome = Some(game.outcome(EndReason::Aborted));
        }
        self.lobby.finish_match();
        self.paused = false;
        CommandOutcome::Accepted
    }

    /// Clear the results and go back to accepting players.
    fn reset(&mut self) -> CommandOutcome {
        self.game = None;
        self.match_init = None;
        self.init_repeats_left = 0;
        self.inputs.clear();
        self.delivery.restart();
        self.paused = false;
        self.lobby.reset();
        CommandOutcome::Accepted
    }

    fn kick(&mut self, player: PlayerId) -> CommandOutcome {
        if !self.lobby.is_occupied(player) {
            return CommandOutcome::rejected(format!("seat {player} is already empty"));
        }
        self.release(player);
        CommandOutcome::AcceptedAndReleased(player)
    }

    /// Generate a board with the current settings without starting anything.
    fn preview(&mut self) -> (Board, u64) {
        let seed = match self.config.map.seed {
            0 => self.seeds.gen(),
            pinned => pinned,
        };
        let players = self.lobby.occupied_count().max(self.config.min_players);
        (generate(&self.config.map.generation, seed, players), seed)
    }

    /// The settings the next match will use.
    pub fn map_settings(&self) -> MapSettings {
        self.config.map
    }
}
