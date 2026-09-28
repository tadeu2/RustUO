use rustuo_core::ClientVersion;
use rustuo_protocol::{
    decode_renaissance_5083_seed, decode_renaissance_account_login, decode_renaissance_game_login,
    decode_renaissance_server_selection, encode_renaissance_account_login_ack,
    RenaissanceServerListEntry,
};
use rustuo_server::{
    admit_renaissance_reconnect, AuthIdIssuer, InMemoryAccountRecord, InMemoryAccountVerifier,
    ReconnectAdmissionError, ReconnectGrantWindow, RenaissanceLoginSession,
    RenaissanceServerEndpoint,
};

struct FixedAuthId(u32);

impl AuthIdIssuer for FixedAuthId {
    type Error = ();

    fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
        Ok(self.0)
    }
}

#[test]
fn old_seed_and_account_login_preserve_wire_credentials() {
    let mut input = [0; 66];
    input[..4].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    input[4] = 0x80;
    input[5..10].copy_from_slice(b"alice");
    input[35..41].copy_from_slice(b"secret");

    let seed = decode_renaissance_5083_seed(&input).unwrap().unwrap();
    assert_eq!(seed.seed, 0x1122_3344);
    let login = decode_renaissance_account_login(seed.remaining).unwrap();
    assert_eq!(login.username, b"alice");
    assert_eq!(login.password, b"secret");
}

#[test]
fn server_list_and_selection_match_legacy_wire_layout() {
    let list = encode_renaissance_account_login_ack(&[RenaissanceServerListEntry {
        name: b"Shard",
        full_percent: 42,
        timezone: -5,
        address: 0x0102_0304,
    }])
    .unwrap();
    assert_eq!(
        &list[..8],
        &[0xA8, 0x00, 0x2E, 0x5D, 0x00, 0x01, 0x00, 0x00]
    );
    assert_eq!(&list[8..13], b"Shard");
    assert_eq!(&list[13..40], &[0; 27]);
    assert_eq!(&list[40..], &[42, 0xFB, 0x01, 0x02, 0x03, 0x04]);

    let selection = decode_renaissance_server_selection(&[0xA0, 0x00, 0x00]).unwrap();
    let mut session = RenaissanceLoginSession::new(
        vec![RenaissanceServerEndpoint {
            address: 0x0102_0304,
            port: 2593,
        }],
        ClientVersion::new(5, 0, 8, 3),
    );
    let mut grants = ReconnectGrantWindow::new();
    let prepared = session
        .select_server(selection, &mut FixedAuthId(0x1122_3344), &mut grants)
        .unwrap();
    assert_eq!(
        prepared.acknowledgement,
        [0x8C, 0x04, 0x03, 0x02, 0x01, 0x0A, 0x21, 0x11, 0x22, 0x33, 0x44]
    );
    assert_eq!(
        grants.consume(0x1122_3344),
        Some(ClientVersion::new(5, 0, 8, 3))
    );
}

#[test]
fn game_reconnect_credentials_and_auth_id_are_consumed_once() {
    let mut frame = [0; 65];
    frame[..5].copy_from_slice(&[0x91, 0x11, 0x22, 0x33, 0x44]);
    frame[5..10].copy_from_slice(b"alice");
    frame[35..41].copy_from_slice(b"secret");
    let login = decode_renaissance_game_login(&frame).unwrap();
    assert_eq!(login.auth_id, 0x1122_3344);
    assert_eq!(login.username, b"alice");
    assert_eq!(login.password, b"secret");

    let mut grants = ReconnectGrantWindow::new();
    grants
        .issue(
            ClientVersion::new(5, 0, 8, 3),
            &mut FixedAuthId(0x1122_3344),
        )
        .unwrap();
    let mut verifier = InMemoryAccountVerifier::new([InMemoryAccountRecord::new(
        rustuo_server::AccountId::new(7),
        b"alice",
        b"secret",
    )]);
    let admitted = admit_renaissance_reconnect(login, &mut grants, &mut verifier).unwrap();
    assert_eq!(admitted.client_version, ClientVersion::new(5, 0, 8, 3));
    assert_eq!(admitted.account.value(), 7);
    assert_eq!(
        admit_renaissance_reconnect(login, &mut grants, &mut verifier),
        Err(ReconnectAdmissionError::UnknownAuthId {
            auth_id: 0x1122_3344
        })
    );
}
