//! The two-byte uplink, and what fits in it.

use bomber_domain::game::Intent;
use bomber_domain::shared::{Direction, PlayerId};
use bomber_protocol::*;

const EVERY_CODE: [u8; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15];

#[test]
fn a_packet_round_trips_for_every_action_and_sequence() {
    for code in EVERY_CODE {
        let action = Action::from_code(code).unwrap();
        for seq in 0..16u8 {
            let packet = ClientPacket::new(PlayerId::new(2), seq, action);
            assert_eq!(
                ClientPacket::decode(&packet.encode()),
                Ok(packet),
                "action {action:?} seq {seq}"
            );
        }
    }
}

#[test]
fn the_hello_packet_matches_the_documented_constant() {
    let decoded = ClientPacket::decode(&HELLO_PACKET).unwrap();
    assert!(decoded.is_hello());
    assert_eq!(decoded.action, Action::Hello);
    assert_eq!(decoded.claimed_player(), None);
    assert_eq!(ClientPacket::hello().encode(), HELLO_PACKET);
}

#[test]
fn reserved_action_codes_are_rejected() {
    for code in 10..15u8 {
        assert!(Action::from_code(code).is_err(), "code {code}");
    }
}

#[test]
fn a_short_packet_errors_instead_of_panicking() {
    assert!(ClientPacket::decode(&[]).is_err());
    assert!(ClientPacket::decode(&[0x00]).is_err());
}

#[test]
fn actions_map_onto_domain_intents() {
    assert_eq!(Action::Idle.intent(), Intent::IDLE);
    assert_eq!(
        Action::LeftBomb.intent(),
        Intent::moving(Direction::Left).with_bomb()
    );
    assert_eq!(Action::Bomb.intent(), Intent::bomb());
    // Hello is a transport request, not a move.
    assert_eq!(Action::Hello.intent(), Intent::IDLE);
}

#[test]
fn every_intent_has_an_action_that_expresses_it() {
    for direction in Direction::ALL {
        for bomb in [false, true] {
            let intent = Intent {
                movement: Some(direction),
                place_bomb: bomb,
            };
            assert_eq!(Action::from_intent(intent).intent(), intent);
        }
    }
    assert_eq!(Action::from_intent(Intent::bomb()), Action::Bomb);
    assert_eq!(Action::from_intent(Intent::IDLE), Action::Idle);
}

/// A bot is not obliged to transmit on ticks where it has nothing to say, so
/// an explicit idle must be exactly as good as silence.
#[test]
#[allow(clippy::unnecessary_literal_unwrap)]
fn idle_is_the_same_intent_as_no_packet_at_all() {
    let nothing_received: Option<Intent> = None;
    let effective = nothing_received.unwrap_or(Intent::IDLE);
    assert_eq!(effective, Action::Idle.intent());
}
