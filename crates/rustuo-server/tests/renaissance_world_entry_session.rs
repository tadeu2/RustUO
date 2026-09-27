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

fn admitted_session() -> RenaissanceWorldEntrySession {
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
    let mut session = admitted_session();
    assert_eq!(session.account().username(), "alice");
    assert_eq!(session.client_version(), ClientVersion::new(5, 0, 8, 3));
    let list = session.character_list().unwrap();
    assert_eq!(list.len(), 309);
    assert_eq!(
        &list[..9],
        &[0xA9, 0x01, 0x35, 5, b'A', b'l', b'i', b'c', b'e']
    );
    assert_eq!(&list[9..304], &[0; 295]);
    assert_eq!(&list[304..], &[0, 0, 0, 0, 0x14]);

    assert_eq!(
        session.play_character(&slot_request(0)).unwrap(),
        [
            0x1B, 0, 0, 0, 1, 0, 0, 0, 0, 0x01, 0x90, 0x0D, 0xAF, 0x0A, 0x0E, 0, 0x0E, 2, 0, 0xFF,
            0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0x1C, 0, 0x10, 0, 0, 0, 0, 0, 0, 0,
        ]
    );
}

#[test]
fn invalid_slot_and_frame_do_not_advance_session() {
    let mut session = admitted_session();
    assert_eq!(
        session.play_character(&slot_request(0)),
        Err(WorldEntryError::InvalidPhase)
    );
    session.character_list().unwrap();
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
    assert_eq!(session.play_character(&slot_request(0)).unwrap()[0], 0x1B);
}

#[test]
fn list_and_play_replays_are_rejected() {
    let mut session = admitted_session();
    session.character_list().unwrap();
    assert_eq!(session.character_list(), Err(WorldEntryError::InvalidPhase));
    session.play_character(&slot_request(0)).unwrap();
    assert_eq!(
        session.play_character(&slot_request(0)),
        Err(WorldEntryError::InvalidPhase)
    );
    assert_eq!(session.character_list(), Err(WorldEntryError::InvalidPhase));
}
