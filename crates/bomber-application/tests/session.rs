//! A whole session -- admission, the countdown, a match, delivery policy --
//! driven as plain function calls. No sockets, no clock, no runtime.

mod common;

use bomber_application::ModeratorCommand;
use bomber_domain::lobby::{AdmissionError, LobbyState};
use bomber_protocol::{Action, ServerFrame};
use common::*;

#[test]
fn seats_are_handed_out_in_order_and_the_assigned_frame_agrees() {
    let mut session = session(0);
    let first = session.admit(None).unwrap();
    let second = session.admit(None).unwrap();
    assert_eq!((first.raw(), second.raw()), (0, 1));

    match session.assigned_frame(second) {
        ServerFrame::Assigned(assigned) => {
            assert_eq!(assigned.player_id, second);
            assert_eq!(assigned.max_players, 4);
        }
        other => panic!("expected ASSIGNED, got {other:?}"),
    }
}

#[test]
fn a_full_lobby_refuses_further_bots() {
    let mut session = session(4);
    assert_eq!(session.admit(None), Err(AdmissionError::Full));
}

#[test]
fn a_match_will_not_start_without_enough_players() {
    let mut session = session(1);
    let outcome = session.execute(ModeratorCommand::Start);
    assert!(!outcome.is_accepted());
    assert_eq!(session.state(), LobbyState::Open);
}

#[test]
fn starting_runs_a_countdown_before_the_match() {
    let mut session = session(2);
    session.execute(ModeratorCommand::Start);
    assert_eq!(session.state(), LobbyState::Countdown);

    let report = session.tick();
    assert!(report.match_started);
    assert_eq!(session.state(), LobbyState::Running);
    assert!(session.game().is_some());
}

/// With no acknowledgements, repetition is the only delivery guarantee there
/// is -- so the frame carrying the board has to go out more than once.
#[test]
fn match_init_is_repeated_and_addressed_to_each_player() {
    let mut session = session(2);
    session.execute(ModeratorCommand::Start);

    let mut inits_per_player = [0usize; 2];
    for _ in 0..8 {
        let report = session.tick();
        for (recipient, frame) in &report.unicast {
            if let ServerFrame::MatchInit(init) = frame {
                assert_eq!(
                    init.your_player_id, *recipient,
                    "each copy must name its own recipient"
                );
                inits_per_player[recipient.index()] += 1;
            }
        }
    }
    assert_eq!(inits_per_player, [5, 5], "the configured five repeats");
}

#[test]
fn the_first_frame_of_a_match_is_a_complete_keyframe() {
    let mut session = session(2);
    session.execute(ModeratorCommand::Start);
    let report = session.tick();

    let keyframes: Vec<_> = report.broadcast.iter().filter(|f| is_keyframe(f)).collect();
    assert_eq!(keyframes.len(), 1, "clients hold nothing yet");
}

/// A frame every tick, without exception. Skipping one because nothing
/// happened would break the base-tick chain and strand every client until the
/// next keyframe.
#[test]
fn every_tick_carries_exactly_one_state_frame_and_the_chain_is_unbroken() {
    let mut session = running(2);
    let mut expected_base = session.game().unwrap().tick;

    for _ in 0..200 {
        let report = session.tick();
        let state_frames: Vec<&ServerFrame> = report
            .broadcast
            .iter()
            .filter(|f| matches!(f, ServerFrame::Keyframe(_) | ServerFrame::Delta(_)))
            .collect();
        assert_eq!(state_frames.len(), 1, "one state frame per tick");

        match state_frames[0] {
            ServerFrame::Keyframe(keyframe) => expected_base = keyframe.tick,
            ServerFrame::Delta(delta) => {
                assert_eq!(
                    delta.base_tick, expected_base,
                    "delta at tick {} does not chain from {}",
                    delta.tick, expected_base
                );
                expected_base = delta.tick;
            }
            _ => unreachable!(),
        }
    }
}

/// The keyframe cadence is the ceiling on how long a bot can stay desynced,
/// since it has no way to ask for a resend.
#[test]
fn a_keyframe_arrives_on_the_configured_cadence() {
    let mut session = running(2);
    let interval = session.config().keyframe_interval_ticks;

    let mut keyframe_ticks = Vec::new();
    for _ in 0..(interval * 4) {
        let report = session.tick();
        for frame in &report.broadcast {
            if let ServerFrame::Keyframe(keyframe) = frame {
                keyframe_ticks.push(keyframe.tick);
            }
        }
    }

    assert!(keyframe_ticks.len() >= 3, "got {keyframe_ticks:?}");
    for pair in keyframe_ticks.windows(2) {
        assert_eq!(pair[1] - pair[0], interval);
    }
}

#[test]
fn an_empty_tick_still_produces_a_delta() {
    let mut session = running(2);
    // Nobody submits anything, so nothing at all happens for a while.
    let mut saw_empty_delta = false;
    for _ in 0..20 {
        let report = session.tick();
        if let Some(delta) = report.broadcast.iter().find_map(as_delta) {
            if delta.records.is_empty() {
                saw_empty_delta = true;
            }
        }
    }
    assert!(saw_empty_delta, "an idle tick must still keep clients current");
}

#[test]
fn inputs_reach_the_simulation() {
    let mut session = running(2);
    let start = session.game().unwrap().player(player(0)).unwrap().cell;

    session.submit(player(0), Action::Right, 1).unwrap();
    session.tick();

    let now = session.game().unwrap().player(player(0)).unwrap().cell;
    assert_eq!(now.x, start.x + 1, "a step commits on the tick it starts");
}

#[test]
fn a_stale_or_duplicated_packet_is_discarded() {
    let mut session = running(2);
    session.submit(player(0), Action::Right, 5).unwrap();
    assert!(
        session.submit(player(0), Action::Left, 5).is_err(),
        "same sequence number is a duplicate"
    );
    assert!(
        session.submit(player(0), Action::Left, 1).is_err(),
        "an older sequence number arrived late"
    );
    assert!(session.submit(player(0), Action::Left, 6).is_ok());
}

#[test]
fn packets_from_an_empty_seat_are_refused() {
    let mut session = session(1);
    assert!(session.submit(player(3), Action::Right, 1).is_err());
}

#[test]
fn a_match_runs_to_completion_and_announces_itself() {
    let mut session = running(2);

    let mut ended = None;
    for tick in 0..4000u32 {
        // Both bots sit still; sudden death eventually settles it.
        let report = session.tick();
        if let Some(outcome) = report.match_ended {
            assert!(
                report
                    .broadcast
                    .iter()
                    .any(|f| matches!(f, ServerFrame::MatchEnd(_))),
                "the end has to be announced on the wire, not just reported internally"
            );
            ended = Some((tick, outcome));
            break;
        }
    }

    let (_, outcome) = ended.expect("sudden death should finish the match");
    assert_eq!(session.state(), LobbyState::MatchOver);
    assert_eq!(outcome.results.len(), 2);
}

/// Seats are the identity a bot puts in byte 0 of every packet, so kicking
/// somebody must not renumber anyone else.
#[test]
fn kicking_a_middle_seat_does_not_renumber_the_survivors() {
    let mut session = session(3);
    session.execute(ModeratorCommand::Kick(player(1)));

    session.execute(ModeratorCommand::Start);
    session.tick();

    let game = session.game().unwrap();
    let ids: Vec<u8> = game.players.iter().map(|p| p.id.raw()).collect();
    assert_eq!(ids, vec![0, 2], "player 2 keeps the id it was assigned");
    assert!(game.player(player(1)).is_none());
}

#[test]
fn a_freed_seat_is_offered_to_the_next_bot() {
    let mut session = session(3);
    session.execute(ModeratorCommand::Kick(player(1)));
    assert_eq!(session.admit(None), Ok(player(1)));
}

#[test]
fn the_lobby_snapshot_reports_what_the_ui_needs() {
    let session = session(2);
    let snapshot = session.snapshot();

    assert_eq!(snapshot.state, LobbyState::Open);
    assert!(snapshot.can_start);
    assert_eq!(snapshot.seats.len(), 4);
    assert_eq!(
        snapshot.seats.iter().filter(|s| s.connected).count(),
        2,
        "empty seats are still listed"
    );
    assert_eq!(snapshot.seats[0].name, "bot-0");
}

#[test]
fn presence_notices_a_bot_that_has_gone_quiet() {
    let mut session = running(2);
    for seq in 0..10u8 {
        session.submit(player(0), Action::Right, seq).unwrap();
        session.tick();
    }
    assert!(!session.snapshot().seats[0].stale);

    let stale_after = session.config().stale_after_ticks;
    for _ in 0..(stale_after + 2) {
        session.tick();
    }

    let snapshot = session.snapshot();
    assert!(snapshot.seats[0].stale, "silence eventually shows as stale");
    assert!(!snapshot.seats[1].stale_ticks.is_some(), "seat 1 never spoke");
}

#[test]
fn dropped_packets_show_up_as_loss() {
    let mut session = running(2);
    // Send every other sequence number: the gaps are inferable losses.
    for seq in (0..16u8).step_by(2) {
        session.submit(player(0), Action::Right, seq).unwrap();
        session.tick();
    }
    let loss = session.snapshot().seats[0].presence.loss_pct;
    assert!(loss > 40.0 && loss < 60.0, "expected about 50%, got {loss}");
}


// ---------------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------------

#[test]
fn a_bot_may_name_itself_when_it_joins() {
    let mut session = session(0);
    let id = session.admit(Some("team-rocket")).unwrap();
    let snapshot = session.snapshot();
    assert_eq!(snapshot.seats[id.index()].name, "team-rocket");
}

#[test]
fn a_bot_that_sends_no_name_gets_the_default() {
    let mut session = session(0);
    session.admit(None).unwrap();
    assert_eq!(session.snapshot().seats[0].name, "bot-0");
}

/// A moderator renames seats to tell two bots apart. A bot must not be able to
/// undo that by reconnecting.
#[test]
fn a_moderator_rename_beats_the_name_a_bot_chose() {
    let mut session = session(0);
    let id = session.admit(Some("self-chosen")).unwrap();
    session.execute(ModeratorCommand::Rename {
        player: id,
        name: "moderator-chosen".into(),
    });
    assert_eq!(session.snapshot().seats[0].name, "moderator-chosen");
}

#[test]
fn a_hostile_name_is_cleaned_up_rather_than_refused() {
    let mut session = session(0);
    session.admit(Some(&"\u{7}nasty\nname".repeat(40))).unwrap();
    let name = &session.snapshot().seats[0].name;
    assert!(name.chars().count() <= 24, "bounded: {name:?}");
    assert!(!name.chars().any(char::is_control), "printable: {name:?}");
}

#[test]
fn the_name_goes_with_the_seat_when_it_is_freed() {
    let mut session = session(0);
    let id = session.admit(Some("transient")).unwrap();
    session.execute(ModeratorCommand::Kick(id));
    assert_eq!(session.snapshot().seats[0].name, "bot-0");
}

#[test]
fn match_players_carry_the_names_they_joined_under() {
    let mut session = session(0);
    session.admit(Some("alpha")).unwrap();
    session.admit(Some("beta")).unwrap();
    session.execute(ModeratorCommand::Start);
    session.tick();

    let snapshot = session.snapshot();
    let names: Vec<&str> = snapshot.seats.iter().take(2).map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "beta"]);
}
