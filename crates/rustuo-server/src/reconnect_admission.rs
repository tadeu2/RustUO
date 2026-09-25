use std::collections::VecDeque;

use rustuo_protocol::RenaissanceGameLogin;

use crate::ReconnectCredentialVerifier;

pub const RECONNECT_GRANT_WINDOW_CAPACITY: usize = 128;
const MAX_AUTH_ID_ISSUANCE_ATTEMPTS: usize = 128;

pub trait AuthIdIssuer {
    type Error;

    fn issue_auth_id(&mut self) -> Result<u32, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectGrantWindow<Version> {
    grants: VecDeque<(u32, Version)>,
}

impl<Version> Default for ReconnectGrantWindow<Version> {
    fn default() -> Self {
        Self {
            grants: VecDeque::new(),
        }
    }
}

impl<Version> ReconnectGrantWindow<Version> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn issue<I: AuthIdIssuer>(
        &mut self,
        client_version: Version,
        issuer: &mut I,
    ) -> Result<u32, ReconnectGrantError<I::Error>> {
        for _ in 0..MAX_AUTH_ID_ISSUANCE_ATTEMPTS {
            let auth_id = issuer
                .issue_auth_id()
                .map_err(ReconnectGrantError::AuthIdIssuanceFailed)?;
            if self.grants.iter().any(|(live_id, _)| *live_id == auth_id) {
                continue;
            }

            if self.grants.len() == RECONNECT_GRANT_WINDOW_CAPACITY {
                self.grants.pop_front();
            }
            self.grants.push_back((auth_id, client_version));
            return Ok(auth_id);
        }

        Err(ReconnectGrantError::CollisionExhausted {
            attempts: MAX_AUTH_ID_ISSUANCE_ATTEMPTS,
        })
    }

    pub fn consume(&mut self, auth_id: u32) -> Option<Version> {
        let position = self
            .grants
            .iter()
            .position(|(live_id, _)| *live_id == auth_id)?;
        self.grants.remove(position).map(|(_, version)| version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectGrantError<E> {
    AuthIdIssuanceFailed(E),
    CollisionExhausted { attempts: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenaissanceReconnectAdmission<Account, Version> {
    pub client_version: Version,
    pub account: Account,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectAdmissionError<E> {
    UnknownAuthId { auth_id: u32 },
    VerifierFailed(E),
}

pub fn admit_renaissance_reconnect<V, Version>(
    login: RenaissanceGameLogin<'_>,
    grants: &mut ReconnectGrantWindow<Version>,
    verifier: &mut V,
) -> Result<RenaissanceReconnectAdmission<V::Account, Version>, ReconnectAdmissionError<V::Error>>
where
    V: ReconnectCredentialVerifier,
{
    let client_version =
        grants
            .consume(login.auth_id)
            .ok_or(ReconnectAdmissionError::UnknownAuthId {
                auth_id: login.auth_id,
            })?;
    let account = verifier
        .verify(login.username, login.password)
        .map_err(ReconnectAdmissionError::VerifierFailed)?;

    Ok(RenaissanceReconnectAdmission {
        client_version,
        account,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use rustuo_protocol::RenaissanceGameLogin;

    use crate::account_repository::{
        LegacyAccountError, LegacyAccountIdentity, LegacyXmlAccountRepository,
        RepositoryCredentialVerifier,
    };
    use crate::ReconnectCredentialVerifier;

    use super::{
        admit_renaissance_reconnect, AuthIdIssuer, ReconnectAdmissionError, ReconnectGrantWindow,
        RenaissanceReconnectAdmission,
    };

    static NEXT_XML_FILE: AtomicUsize = AtomicUsize::new(0);

    struct XmlFile(PathBuf);

    impl XmlFile {
        fn new(contents: &str) -> Self {
            let sequence = NEXT_XML_FILE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rustuo-reconnect-admission-{}-{sequence}.xml",
                std::process::id()
            ));
            fs::write(&path, contents).unwrap();
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for XmlFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    struct FixedIssuer {
        ids: VecDeque<u32>,
    }

    impl AuthIdIssuer for FixedIssuer {
        type Error = &'static str;

        fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
            self.ids.pop_front().ok_or("no auth IDs remaining")
        }
    }

    struct TestVerifier {
        result: Result<u8, &'static str>,
        calls: usize,
    }

    impl ReconnectCredentialVerifier for TestVerifier {
        type Account = u8;
        type Error = &'static str;

        fn verify(
            &mut self,
            _username: &[u8],
            _password: &[u8],
        ) -> Result<Self::Account, Self::Error> {
            self.calls += 1;
            self.result
        }
    }

    fn game_login(auth_id: u32) -> RenaissanceGameLogin<'static> {
        RenaissanceGameLogin {
            auth_id,
            username: b"legacy-user",
            password: b"secret",
        }
    }

    fn grant(version: u32, auth_id: u32) -> ReconnectGrantWindow<u32> {
        let mut grants = ReconnectGrantWindow::new();
        let mut issuer = FixedIssuer {
            ids: [auth_id].into_iter().collect(),
        };
        assert_eq!(grants.issue(version, &mut issuer), Ok(auth_id));
        grants
    }

    fn legacy_repository_verifier(
        xml: &str,
    ) -> (
        XmlFile,
        RepositoryCredentialVerifier<LegacyXmlAccountRepository>,
    ) {
        let file = XmlFile::new(xml);
        let repository = LegacyXmlAccountRepository::open(file.path()).unwrap();
        (file, RepositoryCredentialVerifier::new(repository))
    }

    #[test]
    fn accepted_grant_is_consumed_once_and_returns_its_version() {
        let mut grants = grant(5083, 42);
        let mut verifier = TestVerifier {
            result: Ok(7),
            calls: 0,
        };

        let admission = admit_renaissance_reconnect(game_login(42), &mut grants, &mut verifier);

        assert_eq!(admission.unwrap().client_version, 5083);
        assert_eq!(verifier.calls, 1);
        assert_eq!(
            admit_renaissance_reconnect(game_login(42), &mut grants, &mut verifier),
            Err(ReconnectAdmissionError::UnknownAuthId { auth_id: 42 })
        );
        assert_eq!(verifier.calls, 1);
    }

    #[test]
    fn unknown_grant_does_not_invoke_credential_verification() {
        let mut grants = ReconnectGrantWindow::<u32>::new();
        let mut verifier = TestVerifier {
            result: Ok(7),
            calls: 0,
        };

        assert_eq!(
            admit_renaissance_reconnect(game_login(99), &mut grants, &mut verifier),
            Err(ReconnectAdmissionError::UnknownAuthId { auth_id: 99 })
        );
        assert_eq!(verifier.calls, 0);
    }

    #[test]
    fn failed_credentials_consume_the_grant_and_replay_is_rejected() {
        let mut grants = grant(5083, 42);
        let mut verifier = TestVerifier {
            result: Err("rejected"),
            calls: 0,
        };

        assert_eq!(
            admit_renaissance_reconnect(game_login(42), &mut grants, &mut verifier),
            Err(ReconnectAdmissionError::VerifierFailed("rejected"))
        );
        assert_eq!(verifier.calls, 1);
        assert_eq!(
            admit_renaissance_reconnect(game_login(42), &mut grants, &mut verifier),
            Err(ReconnectAdmissionError::UnknownAuthId { auth_id: 42 })
        );
        assert_eq!(verifier.calls, 1);
    }

    #[test]
    fn legacy_xml_account_is_admitted_with_stored_identity_and_exact_grant_version() {
        let (file, mut verifier) = legacy_repository_verifier(
            "<accounts><account><username>Alice</username><password>secret</password></account></accounts>",
        );
        let original_xml = fs::read(file.path()).unwrap();
        let version = 0x0508_0003;
        let mut grants = grant(version, 42);
        let login = RenaissanceGameLogin {
            auth_id: 42,
            username: b"Alice",
            password: b"secret",
        };

        let admission = admit_renaissance_reconnect(login, &mut grants, &mut verifier);

        assert_eq!(
            admission,
            Ok(RenaissanceReconnectAdmission {
                client_version: version,
                account: LegacyAccountIdentity::new("Alice"),
            })
        );
        assert_eq!(fs::read(file.path()).unwrap(), original_xml);
    }

    #[test]
    fn legacy_xml_credential_failure_consumes_grant_and_replay_is_rejected() {
        let (file, mut verifier) = legacy_repository_verifier(
            "<accounts><account><username>Alice</username><password>secret</password></account></accounts>",
        );
        let original_xml = fs::read(file.path()).unwrap();
        let mut grants = grant(0x0508_0003, 42);
        let login = RenaissanceGameLogin {
            auth_id: 42,
            username: b"Alice",
            password: b"incorrect",
        };

        assert_eq!(
            admit_renaissance_reconnect(login, &mut grants, &mut verifier),
            Err(ReconnectAdmissionError::VerifierFailed(
                LegacyAccountError::PasswordMismatch
            ))
        );
        assert_eq!(
            admit_renaissance_reconnect(login, &mut grants, &mut verifier),
            Err(ReconnectAdmissionError::UnknownAuthId { auth_id: 42 })
        );
        assert_eq!(fs::read(file.path()).unwrap(), original_xml);
    }
}
