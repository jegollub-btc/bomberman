//! What a moderator can do, and what they are told when they cannot.

mod common;

use bomber_application::moderation::MapSettingsPatch;
use bomber_application::{CommandOutcome, ModeratorCommand};
use bomber_domain::board::{validate, Symmetry};
use bomber_domain::game::EndReason;
use bomber_domain::lobby::{AdmissionError, LobbyState};
use bomber_protocol::Action;
use common::*;

#[test]
fn pausing_stops_the_clock_and_resuming_starts_it_again() {
    let mut session = running(2);
    let before = session.game().unwrap().tick;

    assert!(session.execute(ModeratorCommand::Pause).is_accepted());
    for _ in 0..10 {
        let report = session.tick();
        assert!(!report.simulated, "a paused session must not simulate");
    }
    assert_eq!(session.game().unwrap().tick, before, "the clock stopped");

    assert!(session.execute(ModeratorCommand::Resume).is_accepted());
    session.tick();
    assert_eq!(session.game().unwrap().tick, before + 1);
}

#[test]
fn a_command_that_cannot_run_says_why() {
    let mut session = running(2);

    // Not a silent no-op: the UI has to be able to explain the dead button.
    match session.execute(ModeratorCommand::Resume) {
        CommandOutcome::Rejected(reason) => assert!(reason.contains("not paused"), "{reason}"),
        other => panic!("expected a rejection, got {other:?}"),
    }
    match session.execute(ModeratorCommand::Start) {
        CommandOutcome::Rejected(reason) => assert!(reason.contains("already"), "{reason}"),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

#[test]
fn a_start_rejection_names_the_number_of_players_needed() {
    let mut session = session(1);
    match session.execute(ModeratorCommand::Start) {
        CommandOutcome::Rejected(reason) => {
            assert!(reason.contains('2') && reason.contains('1'), "{reason}");
        }
        other => panic!("expected a rejection, got {other:?}"),
    }
}

/// `end` and `reset` are separate so a finished match can be studied before it
/// is cleared away.
#[test]
fn ending_a_match_keeps_the_results_and_reset_clears_them() {
    let mut session = running(2);
    session.submit(player(0), Action::Bomb, 1).unwrap();
    session.tick();

    assert!(session.execute(ModeratorCommand::End).is_accepted());
    assert_eq!(session.state(), LobbyState::MatchOver);

    let outcome = session.last_outcome().expect("results survive the end");
    assert_eq!(outcome.reason, EndReason::Aborted);
    assert_eq!(outcome.results.len(), 2);

    assert!(session.execute(ModeratorCommand::Reset).is_accepted());
    assert_eq!(session.state(), LobbyState::Open);
    assert!(session.game().is_none());
}

#[test]
fn ending_nothing_is_refused() {
    let mut session = session(2);
    assert!(!session.execute(ModeratorCommand::End).is_accepted());
}

#[test]
fn seats_survive_a_reset_so_bots_need_not_say_hello_again() {
    let mut session = running(2);
    session.execute(ModeratorCommand::End);
    session.execute(ModeratorCommand::Reset);

    let snapshot = session.snapshot();
    assert_eq!(snapshot.seats.iter().filter(|s| s.connected).count(), 2);
    assert!(snapshot.can_start, "ready to run another match immediately");
}

#[test]
fn locking_the_lobby_turns_away_new_bots() {
    let mut session = session(1);
    assert!(session.execute(ModeratorCommand::Lock).is_accepted());
    assert_eq!(session.admit(), Err(AdmissionError::NotAcceptingPlayers));

    assert!(session.execute(ModeratorCommand::Unlock).is_accepted());
    assert!(session.admit().is_ok());
}

#[test]
fn kicking_tells_the_adapter_to_drop_the_binding() {
    let mut session = session(2);
    assert_eq!(
        session.execute(ModeratorCommand::Kick(player(1))),
        CommandOutcome::AcceptedAndReleased(player(1)),
        "the transport binding is the adapter's to release"
    );
    assert!(!session.is_seated(player(1)));
    assert!(!session.execute(ModeratorCommand::Kick(player(1))).is_accepted());
}

#[test]
fn renaming_a_seat_shows_up_in_the_snapshot() {
    let mut session = session(2);
    assert!(session
        .execute(ModeratorCommand::Rename {
            player: player(1),
            name: "team-rocket".into(),
        })
        .is_accepted());
    assert_eq!(session.snapshot().seats[1].name, "team-rocket");
}

#[test]
fn map_settings_apply_to_the_next_match_only() {
    let mut session = session(2);
    assert!(session
        .execute(ModeratorCommand::ConfigureMap(MapSettingsPatch {
            width: Some(21),
            height: Some(17),
            symmetry: Some(Symmetry::MirrorX),
            seed: Some(4242),
            ..MapSettingsPatch::default()
        }))
        .is_accepted());

    session.execute(ModeratorCommand::Start);
    session.tick();

    let board = session.board().expect("a match is running");
    assert_eq!((board.width(), board.height()), (21, 17));

    // And not while one is under way.
    assert!(!session
        .execute(ModeratorCommand::ConfigureMap(MapSettingsPatch {
            width: Some(31),
            ..MapSettingsPatch::default()
        }))
        .is_accepted());
}

#[test]
fn a_pinned_seed_reproduces_the_same_board_every_match() {
    let boards: Vec<_> = (0..2)
        .map(|_| {
            let mut session = session(2);
            session.execute(ModeratorCommand::ConfigureMap(MapSettingsPatch {
                seed: Some(9001),
                ..MapSettingsPatch::default()
            }));
            session.execute(ModeratorCommand::Start);
            session.tick();
            session.board().cloned().unwrap()
        })
        .collect();
    assert_eq!(boards[0], boards[1]);
}

#[test]
fn an_unpinned_seed_gives_a_different_board_each_match() {
    let mut session = session(2);
    session.execute(ModeratorCommand::Start);
    session.tick();
    let first = session.board().cloned().unwrap();

    session.execute(ModeratorCommand::End);
    session.execute(ModeratorCommand::Reset);
    session.execute(ModeratorCommand::Start);
    session.tick();
    let second = session.board().cloned().unwrap();

    assert_ne!(first, second);
}

#[test]
fn previewing_a_map_changes_nothing() {
    let mut session = session(2);
    let before = session.state();

    match session.execute(ModeratorCommand::PreviewMap) {
        CommandOutcome::Preview { board, seed } => {
            assert_eq!(validate(&board), Ok(()), "a preview must be playable too");
            assert_ne!(seed, 0, "the preview reports the seed it used");
        }
        other => panic!("expected a preview, got {other:?}"),
    }

    assert_eq!(session.state(), before);
    assert!(session.game().is_none());
}

#[test]
fn can_start_tracks_the_rule_the_server_actually_enforces() {
    let mut session = session(1);
    assert!(!session.snapshot().can_start);

    session.admit().unwrap();
    assert!(session.snapshot().can_start);

    session.execute(ModeratorCommand::Pause);
    assert!(!session.snapshot().can_start, "paused means not startable");
}
