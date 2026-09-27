use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use rustuo_core::ClientVersion;
use rustuo_server::account_repository::LegacyXmlAccountRepository;
use rustuo_server::renaissance_login_flow::RenaissanceLoginFlow;
use rustuo_server::renaissance_world_entry_session::{
    AvatarPresentation, RenaissanceWorldEntrySession, WorldEntryError,
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
