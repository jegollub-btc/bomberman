//! Every frame must survive a trip through the wire and back unchanged, and no
//! malformed input may panic. Anything arriving on a UDP socket is untrusted.

use bomber_domain::board::{Tile, TileGrid};
use bomber_domain::game::{EndReason, PowerupKind, Rules};
use bomber_domain::lobby::LobbyState;
use bomber_domain::shared::{Cell, Direction, PlayerId};
use bomber_protocol::*;

fn sample_grid() -> TileGrid {
    let (w, h) = (15u8, 13u8);
    let mut grid = TileGrid::filled(w, h, Tile::Empty);
    for cell in grid.iter_cells().collect::<Vec<_>>() {
        let structural = cell.x == 0
            || cell.y == 0
            || cell.x == w - 1
            || cell.y == h - 1
            || (cell.x.is_multiple_of(2) && cell.y.is_multiple_of(2));
        let tile = if structural {
            Tile::Solid
        } else if (cell.x + cell.y) % 3 == 0 {
            Tile::Soft
        } else {
            Tile::Empty
        };
        grid.set(cell, tile);
    }
    grid
}

fn all_frames() -> Vec<ServerFrame> {
    let grid = sample_grid();
    vec![
        ServerFrame::Assigned(Assigned {
            tick: 7,
            protocol_version: PROTOCOL_VERSION,
            player_id: PlayerId::new(1),
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
            your_player_id: PlayerId::new(2),
            grid: grid.clone(),
            spawns: vec![
                Cell::new(1, 1),
                Cell::new(13, 1),
                Cell::new(1, 11),
                Cell::new(13, 11),
            ],
            rules: Rules::default(),
        }),
        ServerFrame::Keyframe(Keyframe {
            tick: 600,
            grid,
            players: vec![PlayerSnapshot {
                id: PlayerId::new(0),
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
                owner: PlayerId::new(0),
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
                    id: PlayerId::new(0),
                    alive: true,
                    moving: false,
                    x: 3,
                    y: 5,
                    dir: Direction::Up,
                    move_progress: 0,
                },
                DeltaRecord::PlayerStats {
                    id: PlayerId::new(0),
                    bombs_max: 2,
                    flame: 4,
                    speed: 1,
                    score: 1300,
                },
                DeltaRecord::BombAdd {
                    id: 10,
                    owner: PlayerId::new(1),
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
                    taken_by: Some(PlayerId::new(0)),
                },
                DeltaRecord::PowerupRemove {
                    id: 6,
                    taken_by: None,
                },
                DeltaRecord::PlayerDeath {
                    id: PlayerId::new(3),
                    killer: None,
                },
                DeltaRecord::FlameAdd {
                    x: 3,
                    y: 6,
                    ticks: 30,
                },
                DeltaRecord::FlameRemove { x: 5, y: 5 },
                DeltaRecord::WallClosed { x: 1, y: 1 },
                DeltaRecord::Timer {
                    ticks_remaining: 8999,
                },
            ],
        }),
        ServerFrame::MatchEnd(MatchEnd {
            tick: 5000,
            reason: EndReason::LastStanding,
            winner: Some(PlayerId::new(0)),
            results: vec![
                PlayerResultRecord {
                    id: PlayerId::new(0),
                    placement: 1,
                    score: 1300,
                },
                PlayerResultRecord {
                    id: PlayerId::new(1),
                    placement: 2,
                    score: 400,
                },
            ],
        }),
        ServerFrame::MatchEnd(MatchEnd {
            tick: 5000,
            reason: EndReason::Timeout,
            winner: None,
            results: vec![],
        }),
    ]
}

#[test]
fn every_server_frame_round_trips() {
    for frame in all_frames() {
        let bytes = frame.encode();
        assert_eq!(
            ServerFrame::decode(&bytes).as_ref(),
            Ok(&frame),
            "frame type 0x{:02x}",
            frame.frame_type()
        );
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

/// Truncation must produce an error, never an index out of bounds.
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
    let bad_record = [frame_type::DELTA, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0x7E];
    assert_eq!(
        ServerFrame::decode(&bad_record),
        Err(ProtoError::UnknownRecord(0x7E))
    );
}

#[test]
fn a_no_player_sentinel_round_trips_as_none() {
    let record = DeltaRecord::PowerupRemove {
        id: 1,
        taken_by: None,
    };
    let mut w = Writer::new();
    record.encode(&mut w);
    let bytes = w.finish();
    assert_eq!(bytes.last(), Some(&NO_PLAYER));
    assert_eq!(DeltaRecord::decode(&mut Reader::new(&bytes)), Ok(record));
}

#[test]
fn the_packed_grid_is_two_bits_per_cell() {
    let grid = sample_grid();
    let mut w = Writer::new();
    bomber_protocol::grid::encode(&grid, &mut w);
    let bytes = w.finish();
    assert_eq!(bytes.len(), bomber_protocol::grid::packed_len(15, 13));
    assert_eq!(bytes.len(), (15 * 13usize).div_ceil(4));
    assert_eq!(
        bomber_protocol::grid::decode(&mut Reader::new(&bytes), 15, 13),
        Ok(grid)
    );
}
