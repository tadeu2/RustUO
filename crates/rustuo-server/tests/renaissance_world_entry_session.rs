use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use rustuo_core::ClientVersion;
use rustuo_server::account_repository::LegacyXmlAccountRepository;
use rustuo_server::renaissance_login_flow::RenaissanceLoginFlow;
use rustuo_server::renaissance_world_entry_session::{
    AvatarPresentation, RenaissanceLoginTailFixture, RenaissanceWorldEntrySession, WorldEntryError,
};
use rustuo_server::{AuthIdIssuer, RenaissanceServerEndpoint};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

struct FixedAuthId;

impl AuthIdIssuer for FixedAuthId {
    type Error = ();

    fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
        Ok(0x1122_3344)
    }
}

fn admitted_session(feature_flags: u16) -> RenaissanceWorldEntrySession {
    let path = std::env::temp_dir().join(format!(
        "rustuo-world-entry-{}-{}.xml",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, "<accounts><account><username>alice</username><password>secret</password></account></accounts>").unwrap();
    let repository = LegacyXmlAccountRepository::open(&path).unwrap();
    fs::remove_file(path).unwrap();
    let mut flow = RenaissanceLoginFlow::new(
        repository,
        b"Shard".to_vec(),
        RenaissanceServerEndpoint {
            address: 0x0102_0304,
            port: 2593,
        },
        ClientVersion::new(5, 0, 8, 3),
    );
    let mut account = [0; 62];
    account[0] = 0x80;
    account[1..6].copy_from_slice(b"alice");
    account[31..37].copy_from_slice(b"secret");
    flow.handle_account_login(&account).unwrap();
    flow.handle_server_selection(&[0xA0, 0, 0], &mut FixedAuthId)
        .unwrap();
    let mut game = [0; 65];
    game[..5].copy_from_slice(&[0x91, 0x11, 0x22, 0x33, 0x44]);
    game[5..10].copy_from_slice(b"alice");
    game[35..41].copy_from_slice(b"secret");
    let admission = flow.handle_game_login(&game).unwrap();
    RenaissanceWorldEntrySession::new(
        admission,
        feature_flags,
        AvatarPresentation {
            name: b"Alice".to_vec(),
            body: 0x0190,
            direction: 2,
            hue: 0x0456,
            old_flags: 0x40,
            notoriety: 1,
        },
    )
}

fn slot_request(slot: i32) -> [u8; 73] {
    let mut frame = [0; 73];
    frame[0] = 0x5D;
    frame[65..69].copy_from_slice(&slot.to_be_bytes());
    frame
}

#[test]
fn xml_reconnect_lists_one_avatar_then_confirms_seeded_player() {
    let mut session = admitted_session(0x0003);
    assert_eq!(session.account().username(), "alice");
    assert_eq!(session.client_version(), ClientVersion::new(5, 0, 8, 3));
    let [features, list] = session.reconnect_packets().unwrap();
    assert_eq!(features, [0xB3, 0x06, 0x9A]);
    assert_eq!(
        list,
        [
            0x81, 0x7F, 0x25, 0xA2, 0x59, 0xAA, 0x0C, 0x6F, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12,
            0xE8,
        ]
    );

    assert_eq!(
        session.play_character(&slot_request(0)).unwrap(),
        [
            0x48, 0x01, 0xF0, 0x0F, 0xAE, 0x97, 0x94, 0x6B, 0x54, 0x55, 0x11, 0x8B, 0x16, 0x2C,
            0x40, 0x0B, 0x23, 0x88, 0x00, 0x1A,
        ]
    );
}

#[test]
fn invalid_slot_and_frame_do_not_advance_session() {
    let mut session = admitted_session(0x0003);
    assert_eq!(
        session.play_character(&slot_request(0)),
        Err(WorldEntryError::InvalidPhase)
    );
    session.reconnect_packets().unwrap();
    assert_eq!(
        session.play_character(&slot_request(-1)),
        Err(WorldEntryError::UnsupportedSlot(-1))
    );
    assert_eq!(
        session.play_character(&slot_request(1)),
        Err(WorldEntryError::UnsupportedSlot(1))
    );
    assert!(matches!(
        session.play_character(&slot_request(0)[..72]),
        Err(WorldEntryError::Decode(_))
    ));
    assert_eq!(session.play_character(&slot_request(0)).unwrap()[0], 0x48);
}

#[test]
fn list_and_play_replays_are_rejected() {
    let mut session = admitted_session(0x0003);
    session.reconnect_packets().unwrap();
    assert_eq!(
        session.reconnect_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    session.play_character(&slot_request(0)).unwrap();
    assert_eq!(
        session.play_character(&slot_request(0)),
        Err(WorldEntryError::InvalidPhase)
    );
    assert_eq!(
        session.reconnect_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
}

#[test]
fn configured_feature_mask_changes_only_first_reconnect_packet() {
    let [features, list] = admitted_session(0x1234).reconnect_packets().unwrap();
    assert_eq!(features, [0xB3, 0x39, 0x96, 0x1D]);
    assert_eq!(list.len(), 85);
}

#[test]
fn map_setup_follows_confirm_once_and_preserves_frame_boundaries() {
    let mut session = admitted_session(0x0003);
    assert_eq!(
        session.map_setup_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    session.reconnect_packets().unwrap();
    assert_eq!(
        session.map_setup_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    session.play_character(&slot_request(0)).unwrap();

    assert_eq!(
        session.map_setup_packets().unwrap(),
        [
            vec![0x80, 0xCE, 0xCE, 0x0F, 0xE8],
            vec![0x80, 0xCA, 0x51, 0x91, 0x03, 0xA8, 0, 0, 0, 0, 0, 0, 0, 0x06, 0x80,],
            vec![0xB3, 0x06, 0x9A],
        ]
    );
    assert_eq!(
        session.map_setup_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    assert_eq!(
        session.play_character(&slot_request(0)),
        Err(WorldEntryError::InvalidPhase)
    );
}

#[test]
fn map_setup_reuses_explicit_session_feature_mask() {
    let mut session = admitted_session(0x1234);
    session.reconnect_packets().unwrap();
    session.play_character(&slot_request(0)).unwrap();
    assert_eq!(
        session.map_setup_packets().unwrap()[2],
        [0xB3, 0x39, 0x96, 0x1D]
    );
}

#[test]
fn initial_mobile_prefix_requires_map_setup_and_rejects_replay() {
    let mut session = admitted_session(0x1234);
    assert_eq!(
        session.initial_self_mobile_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    session.reconnect_packets().unwrap();
    session.play_character(&slot_request(0)).unwrap();
    assert_eq!(
        session.initial_self_mobile_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    session.map_setup_packets().unwrap();
    let packets = session.initial_self_mobile_packets().unwrap();
    // Literal vectors independently packed from legacy/Server/Network/Compression.cs.
    assert_eq!(
        packets,
        [
            vec![
                0xC9, 0x1F, 0x40, 0xFF, 0xD7, 0x4B, 0xCA, 0x35, 0xAA, 0xAA, 0x2E, 0xB2, 0xB5, 0x7C,
                0x03, 0x40
            ],
            vec![0xB4, 0x0F, 0xFD, 0x71, 0xD6, 0x56, 0xA9, 0x79, 0x46, 0xB5, 0x41, 0x15, 0x5A],
        ]
    );
    assert_eq!(
        session.initial_self_mobile_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    assert_eq!(
        session.map_setup_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
}

#[test]
fn first_send_everything_repeats_visible_fixture_player_once() {
    let mut session = admitted_session(0x0003);
    assert_eq!(
        session.first_send_everything_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    session.reconnect_packets().unwrap();
    session.play_character(&slot_request(0)).unwrap();
    session.map_setup_packets().unwrap();
    let [initial_incoming, _] = session.initial_self_mobile_packets().unwrap();
    assert_eq!(
        session.first_send_everything_packets().unwrap(),
        [initial_incoming]
    );
    assert_eq!(
        session.first_send_everything_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    assert_eq!(
        session.initial_self_mobile_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
}

fn login_tail_fixture() -> RenaissanceLoginTailFixture {
    RenaissanceLoginTailFixture {
        global_light: 7,
        personal_light: 9,
        status: rustuo_protocol::RenaissanceAosMobileStatus {
            hits: (0x0102, 0x0304),
            can_rename: false,
            female: true,
            attributes: [5, 6, 7],
            stamina: (8, 9),
            mana: (10, 11),
            gold: 12,
            physical_resistance: 13,
            weight: 14,
            stat_cap: 15,
            followers: (2, 5),
            elemental_resistances: [16, 17, 18, 19],
            luck: 20,
            damage: (21, 22),
            tithing_points: 23,
        },
        war_mode: true,
        season: 2,
        current_time: (12, 34, 56),
    }
}

fn login_tail_session() -> RenaissanceWorldEntrySession {
    let mut session = admitted_session(0x0003);
    session.reconnect_packets().unwrap();
    session.play_character(&slot_request(0)).unwrap();
    session.map_setup_packets().unwrap();
    session.initial_self_mobile_packets().unwrap();
    session.first_send_everything_packets().unwrap();
    session
        .post_first_send_everything_packets(&login_tail_fixture())
        .unwrap();
    session
}

fn movement_request(direction: u8, sequence: u8) -> [u8; 7] {
    [0x02, direction, sequence, 0, 0, 0, 0]
}

fn compressed(frame: &[u8]) -> Vec<u8> {
    rustuo_protocol::compress_legacy_packet(frame).unwrap()
}

#[test]
fn movement_requires_login_tail_and_malformed_requests_do_not_advance() {
    let mut session = admitted_session(0x0003);
    let request = movement_request(2, 0);
    assert_eq!(
        session.movement_request(&request),
        Err(WorldEntryError::InvalidPhase)
    );
    session.reconnect_packets().unwrap();
    session.play_character(&slot_request(0)).unwrap();
    session.map_setup_packets().unwrap();
    session.initial_self_mobile_packets().unwrap();
    session.first_send_everything_packets().unwrap();
    session
        .post_first_send_everything_packets(&login_tail_fixture())
        .unwrap();

    assert!(matches!(
        session.movement_request(&request[..6]),
        Err(WorldEntryError::MovementDecode(_))
    ));
    assert!(matches!(
        session.movement_request(&[0x02, 2, 0, 0, 0, 0, 0, 0]),
        Err(WorldEntryError::MovementDecode(_))
    ));
    assert_eq!(
        session.movement_request(&request).unwrap(),
        compressed(&[0x22, 0, 1])
    );
    assert_eq!(
        session.movement_request(&request).unwrap(),
        compressed(&[0x22, 1, 1])
    );
}

#[test]
fn movement_turn_does_not_step_but_same_facing_steps_with_run_bit() {
    let mut session = login_tail_session();
    assert_eq!(
        session
            .movement_request(&movement_request(0x84, 0))
            .unwrap(),
        compressed(&[0x22, 0, 1])
    );
    // A run-bit change alone keeps the same low-three-bit facing and steps south.
    assert_eq!(
        session
            .movement_request(&movement_request(0x04, 1))
            .unwrap(),
        compressed(&[0x22, 1, 1])
    );
}

#[test]
fn movement_boundary_rejection_returns_unchanged_position_and_retries_from_zero() {
    let mut session = login_tail_session();
    // The first north request turns; following running requests reach y=1.
    for sequence in 0..2574_u16 {
        let request = movement_request(0x80, sequence as u8);
        let acknowledgment_sequence = if sequence == 0 {
            0
        } else if sequence % 256 == 0 {
            1
        } else {
            sequence as u8
        };
        assert_eq!(
            session.movement_request(&request).unwrap(),
            compressed(&[0x22, acknowledgment_sequence, 1])
        );
    }
    assert_eq!(
        session.movement_request(&movement_request(0, 14)).unwrap(),
        compressed(&[0x22, 14, 1])
    );
    assert_eq!(
        session.movement_request(&movement_request(0, 15)).unwrap(),
        compressed(&[0x21, 15, 0x0D, 0xAF, 0, 0, 0, 0x0E])
    );
    assert_eq!(
        session.movement_request(&movement_request(2, 0)).unwrap(),
        compressed(&[0x22, 0, 1])
    );
}

#[test]
fn movement_rejects_nonzero_initial_sequence_and_accepts_later_mismatch() {
    let mut session = login_tail_session();
    assert_eq!(
        session.movement_request(&movement_request(2, 7)).unwrap(),
        compressed(&[0x21, 7, 0x0D, 0xAF, 0x0A, 0x0E, 2, 0x0E])
    );
    assert_eq!(
        session.movement_request(&movement_request(2, 0)).unwrap(),
        compressed(&[0x22, 0, 1])
    );
    // The legacy handler checks sequence equality only while the server sequence is zero.
    assert_eq!(
        session.movement_request(&movement_request(2, 42)).unwrap(),
        compressed(&[0x22, 1, 1])
    );
}

#[test]
fn movement_ack_uses_pre_advance_sequence_and_wraps_request_255_to_one() {
    let mut session = login_tail_session();
    session.movement_request(&movement_request(2, 0)).unwrap();
    assert_eq!(
        session.movement_request(&movement_request(2, 255)).unwrap(),
        compressed(&[0x22, 1, 1])
    );
    assert_eq!(
        session.movement_request(&movement_request(2, 1)).unwrap(),
        compressed(&[0x22, 1, 1])
    );
}

#[test]
fn login_tail_uses_explicit_fixed_time_in_legacy_order_once() {
    let mut session = admitted_session(0x0003);
    let fixture = login_tail_fixture();
    // Every earlier phase rejects the tail without advancing the session.
    assert_eq!(
        session.post_first_send_everything_packets(&fixture),
        Err(WorldEntryError::InvalidPhase)
    );
    session.reconnect_packets().unwrap();
    assert_eq!(
        session.post_first_send_everything_packets(&fixture),
        Err(WorldEntryError::InvalidPhase)
    );
    session.play_character(&slot_request(0)).unwrap();
    assert_eq!(
        session.post_first_send_everything_packets(&fixture),
        Err(WorldEntryError::InvalidPhase)
    );
    session.map_setup_packets().unwrap();
    assert_eq!(
        session.post_first_send_everything_packets(&fixture),
        Err(WorldEntryError::InvalidPhase)
    );
    session.initial_self_mobile_packets().unwrap();
    assert_eq!(
        session.post_first_send_everything_packets(&fixture),
        Err(WorldEntryError::InvalidPhase)
    );
    session.first_send_everything_packets().unwrap();

    // Raw fixtures independently follow legacy DoLogin and the type-four status layout.
    let incoming = vec![
        0x78, 0, 23, 0, 0, 0, 1, 1, 0x90, 0x0D, 0xAF, 0x0A, 0x0E, 14, 2, 4, 0x56, 0x40, 1, 0, 0, 0,
        0,
    ];
    let mut status = vec![0x11, 0, 88, 0, 0, 0, 1];
    status.extend_from_slice(b"Alice");
    status.extend_from_slice(&[0; 25]);
    status.extend_from_slice(&[
        1, 2, 3, 4, 0, 4, 1, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9, 0, 10, 0, 11, 0, 0, 0, 12, 0, 13, 0, 14,
        0, 15, 2, 5, 0, 16, 0, 17, 0, 18, 0, 19, 0, 20, 0, 21, 0, 22, 0, 0, 0, 23,
    ]);
    let raw = [
        vec![0x4F, 7],
        vec![0x4E, 0, 0, 0, 1, 9],
        vec![0x55],
        incoming.clone(),
        status,
        vec![0x72, 1, 0, 0x32, 0],
        vec![0xBC, 2, 1],
        vec![0x5B, 12, 34, 56],
        vec![0xBF, 0, 6, 0, 8, 1],
        incoming,
    ];
    let expected = raw.map(|frame| rustuo_protocol::compress_legacy_packet(&frame).unwrap());
    assert_eq!(
        session
            .post_first_send_everything_packets(&fixture)
            .unwrap(),
        expected
    );
    assert_eq!(
        session.post_first_send_everything_packets(&fixture),
        Err(WorldEntryError::InvalidPhase)
    );
    assert_eq!(
        session.first_send_everything_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    assert_eq!(
        session.initial_self_mobile_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    assert_eq!(
        session.map_setup_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
    assert_eq!(
        session.play_character(&slot_request(0)),
        Err(WorldEntryError::InvalidPhase)
    );
    assert_eq!(
        session.reconnect_packets(),
        Err(WorldEntryError::InvalidPhase)
    );
}
