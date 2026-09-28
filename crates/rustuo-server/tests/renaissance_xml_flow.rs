use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use rustuo_core::ClientVersion;
use rustuo_server::account_repository::{LegacyAccountError, LegacyXmlAccountRepository};
use rustuo_server::renaissance_login_flow::{LoginFlowError, RenaissanceLoginFlow};
use rustuo_server::{
    AuthIdIssuer, ReconnectAdmissionError, ReconnectGrantError, RenaissanceServerEndpoint,
    SessionSelectionError,
};

static NEXT_XML_FILE: AtomicUsize = AtomicUsize::new(0);

struct FixedAuthId(u32);

impl AuthIdIssuer for FixedAuthId {
    type Error = ();

    fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
        Ok(self.0)
    }
}

struct FailsOnceAuthId(bool);

impl AuthIdIssuer for FailsOnceAuthId {
    type Error = &'static str;

    fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
        if !self.0 {
            self.0 = true;
            Err("issuer unavailable")
        } else {
            Ok(0x1122_3344)
        }
    }
}

fn repository() -> LegacyXmlAccountRepository {
    let path = std::env::temp_dir().join(format!(
        "rustuo-renaissance-flow-{}-{}.xml",
        std::process::id(),
        NEXT_XML_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(
        &path,
        "<accounts><account><username>alice</username><password>secret</password></account></accounts>",
    )
    .unwrap();
    let loaded = LegacyXmlAccountRepository::open(&path).unwrap();
    fs::remove_file(path).unwrap();
    loaded
}

fn flow() -> RenaissanceLoginFlow {
    RenaissanceLoginFlow::new(
        repository(),
        b"Shard".to_vec(),
        RenaissanceServerEndpoint {
            address: 0x0102_0304,
            port: 2593,
        },
        ClientVersion::new(5, 0, 8, 3),
    )
}

fn account_login(password: &[u8]) -> [u8; 62] {
    let mut frame = [0; 62];
    frame[0] = 0x80;
    frame[1..6].copy_from_slice(b"alice");
    frame[31..31 + password.len()].copy_from_slice(password);
    frame
}

fn game_login(password: &[u8]) -> [u8; 65] {
    let mut frame = [0; 65];
    frame[..5].copy_from_slice(&[0x91, 0x11, 0x22, 0x33, 0x44]);
    frame[5..10].copy_from_slice(b"alice");
    frame[35..35 + password.len()].copy_from_slice(password);
    frame
}

#[test]
fn xml_account_login_emits_one_legacy_server_offer() {
    let mut flow = self::flow();
    let response = flow
        .handle_account_login(&account_login(b"secret"))
        .unwrap();

    assert_eq!(
        &response[..8],
        &[0xA8, 0x00, 0x2E, 0x5D, 0x00, 0x01, 0x00, 0x00]
    );
    assert_eq!(&response[8..13], b"Shard");
    assert_eq!(&response[13..40], &[0; 27]);
    assert_eq!(&response[40..], &[0, 0, 0x01, 0x02, 0x03, 0x04]);
}

#[test]
fn bad_xml_credentials_reject_without_authenticating_or_issuing_grant() {
    let mut flow = flow();
    assert_eq!(
        flow.handle_account_login(&account_login(b"wrong")).unwrap(),
        [0x82, 0x03]
    );
    assert_eq!(
        flow.handle_server_selection(&[0xA0, 0, 0], &mut FixedAuthId(0x1122_3344)),
        Err(LoginFlowError::NotAuthenticated)
    );
    assert_eq!(
        flow.handle_game_login(&game_login(b"secret")),
        Err(LoginFlowError::Reconnect(
            ReconnectAdmissionError::UnknownAuthId {
                auth_id: 0x1122_3344,
            }
        ))
    );
}

#[test]
fn server_selection_emits_exact_redirect_only_for_offered_index() {
    let mut flow = flow();
    flow.handle_account_login(&account_login(b"secret"))
        .unwrap();
    assert_eq!(
        flow.handle_server_selection(&[0xA0, 0, 1], &mut FixedAuthId(0x1122_3344)),
        Err(LoginFlowError::Selection(
            SessionSelectionError::ServerIndexOutOfRange {
                index: 1,
                server_count: 1,
            }
        ))
    );
    assert_eq!(
        flow.handle_server_selection(&[0xA0, 0, 0], &mut FixedAuthId(0x1122_3344))
            .unwrap(),
        [0x8C, 0x04, 0x03, 0x02, 0x01, 0x0A, 0x21, 0x11, 0x22, 0x33, 0x44]
    );
}

#[test]
fn issuer_failure_preserves_typed_error_and_allows_later_selection() {
    let mut flow = flow();
    flow.handle_account_login(&account_login(b"secret"))
        .unwrap();
    let mut issuer = FailsOnceAuthId(false);

    assert_eq!(
        flow.handle_server_selection(&[0xA0, 0, 0], &mut issuer),
        Err(LoginFlowError::Selection(
            SessionSelectionError::ReconnectGrantFailed(ReconnectGrantError::AuthIdIssuanceFailed(
                "issuer unavailable"
            ))
        ))
    );
    assert_eq!(
        flow.handle_game_login(&game_login(b"secret")),
        Err(LoginFlowError::Reconnect(
            ReconnectAdmissionError::UnknownAuthId {
                auth_id: 0x1122_3344,
            }
        ))
    );
    assert_eq!(
        flow.handle_server_selection(&[0xA0, 0, 0], &mut issuer)
            .unwrap(),
        [0x8C, 0x04, 0x03, 0x02, 0x01, 0x0A, 0x21, 0x11, 0x22, 0x33, 0x44]
    );
}

#[test]
fn game_reconnect_reauthenticates_xml_credentials_and_consumes_grant_once() {
    let mut flow = flow();
    flow.handle_account_login(&account_login(b"secret"))
        .unwrap();
    flow.handle_server_selection(&[0xA0, 0, 0], &mut FixedAuthId(0x1122_3344))
        .unwrap();
    assert_eq!(
        flow.handle_game_login(&game_login(b"wrong")),
        Err(LoginFlowError::Reconnect(
            ReconnectAdmissionError::VerifierFailed(LegacyAccountError::PasswordMismatch,)
        ))
    );
    assert_eq!(
        flow.handle_game_login(&game_login(b"secret")),
        Err(LoginFlowError::Reconnect(
            ReconnectAdmissionError::UnknownAuthId {
                auth_id: 0x1122_3344,
            }
        ))
    );

    let mut flow = self::flow();
    flow.handle_account_login(&account_login(b"secret"))
        .unwrap();
    flow.handle_server_selection(&[0xA0, 0, 0], &mut FixedAuthId(0x1122_3344))
        .unwrap();
    let admitted = flow.handle_game_login(&game_login(b"secret")).unwrap();
    assert_eq!(admitted.account.username(), "alice");
    assert_eq!(admitted.client_version, ClientVersion::new(5, 0, 8, 3));
    assert_eq!(
        flow.handle_game_login(&game_login(b"secret")),
        Err(LoginFlowError::Reconnect(
            ReconnectAdmissionError::UnknownAuthId {
                auth_id: 0x1122_3344,
            }
        ))
    );
}

#[test]
fn unknown_xml_account_uses_invalid_username_rejection() {
    let mut flow = flow();
    let mut frame = account_login(b"secret");
    frame[1..6].copy_from_slice(b"other");
    assert_eq!(flow.handle_account_login(&frame).unwrap(), [0x82, 0x00]);
    assert_eq!(
        flow.handle_server_selection(&[0xA0, 0, 0], &mut FixedAuthId(0x1122_3344)),
        Err(LoginFlowError::NotAuthenticated)
    );
}
