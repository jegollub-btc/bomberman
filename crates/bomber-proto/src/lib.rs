//! Wire protocol for the Bomberman bot arena.
//!
//! This crate is the single source of truth for the byte layouts documented in
//! `BOT_GUIDE.md`. The server, the Rust bot SDK and the guide's constants all
//! derive from here, so the handout cannot drift from the implementation.
//!
//! The shape of the protocol is dictated by one constraint: the client -> server
//! packet is exactly two bytes. See [`client`] for what that costs and
//! [`server`] for how the downlink pays for it.

pub mod client;
pub mod codec;
pub mod server;
pub mod types;

pub use client::{seq_is_newer, Action, ClientPacket, HELLO_PACKET, HELLO_PLAYER_ID};
pub use codec::{ProtoError, Reader, Result, Writer};
pub use server::{
    frame_type, record_type, Assigned, BombSnapshot, Delta, DeltaRecord, EndReason, FlameCell,
    Keyframe, LobbyState, LobbyStatus, MatchEnd, MatchInit, PlayerResult, PlayerSnapshot,
    PowerupSnapshot, ServerFrame, MAX_DATAGRAM, NO_PLAYER, PROTOCOL_VERSION,
};
pub use types::{Direction, PowerupKind, Rules, Tile, TileGrid};

/// Default UDP port bots send their two bytes to.
pub const DEFAULT_UDP_PORT: u16 = 47800;

/// Default HTTP/WebSocket port for the visualizer and moderation UI.
pub const DEFAULT_WEB_PORT: u16 = 8080;

/// Simulation rate. Fixed, not negotiated -- determinism depends on it.
pub const TICK_RATE: u8 = 60;

/// Hard ceiling on concurrent players, set by the 4-bit slot mask in
/// `LOBBY_STATUS` and by the number of corner spawns.
pub const MAX_PLAYERS: usize = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_packet_round_trips_every_action() {
        for code in [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15] {
            let action = Action::from_code(code).unwrap();
            for seq in 0..16u8 {
                let pkt = ClientPacket::new(2, seq, action);
                let decoded = ClientPacket::decode(&pkt.encode()).unwrap();
                assert_eq!(decoded, pkt, "action {action:?} seq {seq}");
            }
        }
    }

    #[test]
    fn hello_packet_matches_the_documented_constant() {
        let decoded = ClientPacket::decode(&HELLO_PACKET).unwrap();
        assert!(decoded.is_hello());
        assert_eq!(decoded.action, Action::Hello);
        assert_eq!(ClientPacket::hello().encode(), HELLO_PACKET);
    }

    #[test]
    fn reserved_action_codes_are_rejected() {
        for code in 10..15u8 {
            assert!(Action::from_code(code).is_err(), "code {code}");
        }
    }

    #[test]
    fn short_client_packet_errors_instead_of_panicking() {
        assert!(ClientPacket::decode(&[]).is_err());
        assert!(ClientPacket::decode(&[0x00]).is_err());
    }

    #[test]
    fn seq_wraps_correctly() {
        assert!(seq_is_newer(1, 0));
        assert!(seq_is_newer(0, 15)); // wrapped
        assert!(!seq_is_newer(0, 0)); // duplicate
        assert!(!seq_is_newer(15, 0)); // stale/reordered
    }

    fn sample_grid() -> TileGrid {
        let mut g = TileGrid::new(15, 13);
        for y in 0..13u8 {
            for x in 0..15u8 {
                let tile = if x == 0 || y == 0 || x == 14 || y == 12 {
                    Tile::Solid
                } else if x % 2 == 0 && y % 2 == 0 {
                    Tile::Solid
                } else if (x + y) % 3 == 0 {
                    Tile::Soft
                } else {
                    Tile::Empty
                };
                g.set(x, y, tile);
            }
        }
        g
    }

    #[test]
    fn tile_grid_packs_two_bits_per_cell() {
        let grid = sample_grid();
        let mut w = Writer::new();
        grid.encode_packed(&mut w);
        let bytes = w.finish();
        assert_eq!(bytes.len(), TileGrid::packed_len(15, 13));
        assert_eq!(bytes.len(), (15 * 13 + 3) / 4);

        let mut r = Reader::new(&bytes);
        assert_eq!(TileGrid::decode_packed(&mut r, 15, 13).unwrap(), grid);
    }

    fn all_frames() -> Vec<ServerFrame> {
        let grid = sample_grid();
        vec![
            ServerFrame::Assigned(Assigned {
                tick: 7,
                protocol_version: PROTOCOL_VERSION,
                player_id: 1,
                tick_rate: TICK_RATE,
                max_players: 4,
            }),
            ServerFrame::LobbyStatus(LobbyStatus {
                tick: 42,
                state: LobbyState::Countdown,
                players_connected: 3,
                max_players: 4,
                slot_mask: 0b0111,
                countdown_ticks: 180,
            }),
            ServerFrame::MatchInit(MatchInit {
                tick: 0,
                protocol_version: PROTOCOL_VERSION,
                match_id: 0xDEAD_BEEF,
                seed: 0x0123_4567_89AB_CDEF,
                your_player_id: 2,
                grid: grid.clone(),
                spawns: vec![(1, 1), (13, 1), (1, 11), (13, 11)],
                rules: Rules::default(),
            }),
            ServerFrame::Keyframe(Keyframe {
                tick: 600,
                grid,
                players: vec![PlayerSnapshot {
                    id: 0,
                    alive: true,
                    moving: true,
                    x: 3,
                    y: 5,
                    dir: Direction::Left,
                    move_progress: 4,
                    bombs_max: 2,
                    flame: 3,
                    speed: 1,
                    score: 1234,
                }],
                bombs: vec![BombSnapshot {
                    id: 9,
                    owner: 0,
                    x: 3,
                    y: 6,
                    fuse_remaining: 77,
                }],
                flames: vec![FlameCell {
                    x: 5,
                    y: 5,
                    ticks_remaining: 12,
                }],
                powerups: vec![PowerupSnapshot {
                    id: 4,
                    x: 7,
                    y: 7,
                    kind: PowerupKind::Speed,
                }],
                ticks_remaining: 9000,
            }),
            ServerFrame::Delta(Delta {
                tick: 601,
                base_tick: 600,
                records: vec![
                    DeltaRecord::PlayerState {
                        id: 0,
                        alive: true,
                        moving: false,
                        x: 3,
                        y: 5,
                        dir: Direction::Up,
                        move_progress: 0,
                    },
                    DeltaRecord::PlayerStats {
                        id: 0,
                        bombs_max: 2,
                        flame: 4,
                        speed: 1,
                        score: 1300,
                    },
                    DeltaRecord::BombAdd {
                        id: 10,
                        owner: 1,
                        x: 9,
                        y: 9,
                        fuse: 120,
                    },
                    DeltaRecord::BombRemove { id: 9 },
                    DeltaRecord::Explosion {
                        x: 3,
                        y: 6,
                        up: 1,
                        down: 2,
                        left: 0,
                        right: 3,
                    },
                    DeltaRecord::TileSet {
                        x: 4,
                        y: 6,
                        tile: Tile::Empty,
                    },
                    DeltaRecord::PowerupAdd {
                        id: 5,
                        x: 4,
                        y: 6,
                        kind: PowerupKind::ExtraBomb,
                    },
                    DeltaRecord::PowerupRemove {
                        id: 4,
                        taken_by: 0,
                    },
                    DeltaRecord::PlayerDeath { id: 3, killer: 0 },
                    DeltaRecord::FlameAdd {
                        x: 3,
                        y: 6,
                        ticks: 30,
                    },
                    DeltaRecord::FlameRemove { x: 5, y: 5 },
                    DeltaRecord::Timer {
                        ticks_remaining: 8999,
                    },
                ],
            }),
            ServerFrame::MatchEnd(MatchEnd {
                tick: 5000,
                reason: EndReason::LastStanding,
                winner: 0,
                results: vec![
                    PlayerResult {
                        id: 0,
                        placement: 1,
                        score: 1300,
                    },
                    PlayerResult {
                        id: 1,
                        placement: 2,
                        score: 400,
                    },
                ],
            }),
        ]
    }

    #[test]
    fn every_server_frame_round_trips() {
        for frame in all_frames() {
            let bytes = frame.encode();
            let decoded = ServerFrame::decode(&bytes).unwrap();
            assert_eq!(decoded, frame, "frame type 0x{:02x}", frame.frame_type());
        }
    }

    #[test]
    fn frames_fit_the_datagram_budget() {
        for frame in all_frames() {
            let len = frame.encode().len();
            assert!(
                len <= MAX_DATAGRAM,
                "frame 0x{:02x} is {len} bytes",
                frame.frame_type()
            );
        }
    }

    /// Anything can arrive on a UDP socket. Truncation must never panic.
    #[test]
    fn truncated_frames_error_at_every_length() {
        for frame in all_frames() {
            let bytes = frame.encode();
            for cut in 0..bytes.len() {
                let _ = ServerFrame::decode(&bytes[..cut]);
            }
        }
    }

    #[test]
    fn unknown_frame_and_record_types_are_reported() {
        assert_eq!(
            ServerFrame::decode(&[0x7F, 0, 0, 0, 0]),
            Err(ProtoError::UnknownFrame(0x7F))
        );
        // A delta claiming one record, whose record tag is nonsense.
        let bad = [frame_type::DELTA, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0x7E];
        assert_eq!(
            ServerFrame::decode(&bad),
            Err(ProtoError::UnknownRecord(0x7E))
        );
    }

    #[test]
    fn speed_levels_never_reach_zero_ticks_per_cell() {
        let rules = Rules {
            ticks_per_cell: 3,
            speed_step_ticks: 2,
            ..Rules::default()
        };
        assert_eq!(rules.ticks_per_cell_at(0), 3);
        assert_eq!(rules.ticks_per_cell_at(1), 1);
        assert_eq!(rules.ticks_per_cell_at(9), 1);
    }
}
