#![forbid(unsafe_code)]

pub mod account_repository;

use std::collections::VecDeque;

use rustuo_core::ClientVersion;
use rustuo_protocol::{encode_renaissance_play_server_ack, RenaissanceServerSelection};

pub const RECONNECT_GRANT_WINDOW_CAPACITY: usize = 128;
const MAX_AUTH_ID_ISSUANCE_ATTEMPTS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenaissanceServerEndpoint {
    pub address: u32,
    pub port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenaissanceSessionPhase {
    Authenticated,
    AwaitingGameReconnect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenaissanceLoginSession {
    offered_servers: Vec<RenaissanceServerEndpoint>,
    client_version: ClientVersion,
    phase: RenaissanceSessionPhase,
}

impl RenaissanceLoginSession {
    pub fn new(
        offered_servers: Vec<RenaissanceServerEndpoint>,
        client_version: ClientVersion,
    ) -> Self {
        Self {
            offered_servers,
            client_version,
            phase: RenaissanceSessionPhase::Authenticated,
        }
    }

    pub fn phase(&self) -> RenaissanceSessionPhase {
        self.phase
    }

    pub fn select_server<I: AuthIdIssuer>(
        &mut self,
        selection: RenaissanceServerSelection,
        issuer: &mut I,
        grants: &mut ReconnectGrantWindow,
    ) -> Result<PreparedPlayServerAck, SessionSelectionError<I::Error>> {
        if self.phase != RenaissanceSessionPhase::Authenticated {
            return Err(SessionSelectionError::InvalidSessionPhase { phase: self.phase });
        }
        if selection.index < 0 {
            return Err(SessionSelectionError::NegativeIndex {
                index: selection.index,
            });
        }
        let Some(&endpoint) = self.offered_servers.get(selection.index as usize) else {
            return Err(SessionSelectionError::ServerIndexOutOfRange {
                index: selection.index,
                server_count: self.offered_servers.len(),
            });
        };

        let auth_id = grants
            .issue(self.client_version, issuer)
            .map_err(SessionSelectionError::ReconnectGrantFailed)?;
        let acknowledgement =
            encode_renaissance_play_server_ack(endpoint.address, endpoint.port, auth_id);
        self.phase = RenaissanceSessionPhase::AwaitingGameReconnect;

        Ok(PreparedPlayServerAck {
            endpoint,
            auth_id,
            acknowledgement,
        })
    }
}

pub trait AuthIdIssuer {
    type Error;

    fn issue_auth_id(&mut self) -> Result<u32, Self::Error>;
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReconnectGrantWindow {
    grants: VecDeque<(u32, ClientVersion)>,
}

impl ReconnectGrantWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn issue<I: AuthIdIssuer>(
        &mut self,
        client_version: ClientVersion,
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

    pub fn consume(&mut self, auth_id: u32) -> Option<ClientVersion> {
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
pub struct PreparedPlayServerAck {
    pub endpoint: RenaissanceServerEndpoint,
    pub auth_id: u32,
    pub acknowledgement: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionSelectionError<E> {
    InvalidSessionPhase { phase: RenaissanceSessionPhase },
    NegativeIndex { index: i16 },
    ServerIndexOutOfRange { index: i16, server_count: usize },
    ReconnectGrantFailed(ReconnectGrantError<E>),
}

/// Application port for account credential verification.
///
/// Implementations receive borrowed raw bytes and are independent of storage
/// and transport concerns.
pub trait ReconnectCredentialVerifier {
    type Account;
    type Error;

    fn verify(&mut self, username: &[u8], password: &[u8]) -> Result<Self::Account, Self::Error>;
}

#[cfg(test)]
mod session_selection_tests {
    use rustuo_core::ClientVersion;
    use rustuo_protocol::RenaissanceServerSelection;

    use super::{
        AuthIdIssuer, ReconnectGrantError, ReconnectGrantWindow, RenaissanceLoginSession,
        RenaissanceServerEndpoint, RenaissanceSessionPhase, SessionSelectionError,
    };

    struct FixedIssuer {
        result: Result<u32, &'static str>,
        calls: usize,
    }

    impl AuthIdIssuer for FixedIssuer {
        type Error = &'static str;

        fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
            self.calls += 1;
            self.result
        }
    }

    fn session() -> RenaissanceLoginSession {
        RenaissanceLoginSession::new(
            vec![
                RenaissanceServerEndpoint {
                    address: 0x1122_3344,
                    port: 0x5566,
                },
                RenaissanceServerEndpoint {
                    address: 0xAABB_CCDD,
                    port: 0x7788,
                },
            ],
            ClientVersion::new(5, 0, 8, 3),
        )
    }

    #[test]
    fn session_selection_returns_endpoint_auth_id_ack_and_advances_phase() {
        let mut session = session();
        let mut issuer = FixedIssuer {
            result: Ok(0x7788_99AA),
            calls: 0,
        };

        let mut grants = ReconnectGrantWindow::new();
        let prepared = session
            .select_server(
                RenaissanceServerSelection { index: 1 },
                &mut issuer,
                &mut grants,
            )
            .unwrap();

        assert_eq!(
            prepared.endpoint,
            RenaissanceServerEndpoint {
                address: 0xAABB_CCDD,
                port: 0x7788
            }
        );
        assert_eq!(prepared.auth_id, 0x7788_99AA);
        assert_eq!(
            prepared.acknowledgement,
            [0x8C, 0xDD, 0xCC, 0xBB, 0xAA, 0x77, 0x88, 0x77, 0x88, 0x99, 0xAA]
        );
        assert_eq!(
            session.phase(),
            RenaissanceSessionPhase::AwaitingGameReconnect
        );
        assert_eq!(issuer.calls, 1);
        assert_eq!(
            grants.consume(0x7788_99AA),
            Some(ClientVersion::new(5, 0, 8, 3))
        );
    }

    #[test]
    fn session_selection_rejects_invalid_indexes_without_issuing_auth() {
        let mut session = session();
        for (index, expected) in [
            (-1, SessionSelectionError::NegativeIndex { index: -1 }),
            (
                2,
                SessionSelectionError::ServerIndexOutOfRange {
                    index: 2,
                    server_count: 2,
                },
            ),
        ] {
            let mut issuer = FixedIssuer {
                result: Ok(123),
                calls: 0,
            };
            let mut grants = ReconnectGrantWindow::new();
            assert_eq!(
                session.select_server(
                    RenaissanceServerSelection { index },
                    &mut issuer,
                    &mut grants
                ),
                Err(expected)
            );
            assert_eq!(session.phase(), RenaissanceSessionPhase::Authenticated);
            assert_eq!(issuer.calls, 0);
            assert_eq!(grants.consume(123), None);
        }
    }

    #[test]
    fn session_selection_issuer_failure_keeps_session_authenticated() {
        let mut session = session();
        let mut issuer = FixedIssuer {
            result: Err("unavailable"),
            calls: 0,
        };

        let mut grants = ReconnectGrantWindow::new();
        assert_eq!(
            session.select_server(
                RenaissanceServerSelection { index: 0 },
                &mut issuer,
                &mut grants
            ),
            Err(SessionSelectionError::ReconnectGrantFailed(
                ReconnectGrantError::AuthIdIssuanceFailed("unavailable")
            ))
        );
        assert_eq!(session.phase(), RenaissanceSessionPhase::Authenticated);
        assert_eq!(issuer.calls, 1);
        assert_eq!(grants.consume(0), None);
    }

    #[test]
    fn session_selection_rejects_a_second_attempt_without_reissuing_auth() {
        let mut session = session();
        let mut issuer = FixedIssuer {
            result: Ok(456),
            calls: 0,
        };
        let mut grants = ReconnectGrantWindow::new();
        session
            .select_server(
                RenaissanceServerSelection { index: 0 },
                &mut issuer,
                &mut grants,
            )
            .unwrap();

        assert_eq!(
            session.select_server(
                RenaissanceServerSelection { index: 1 },
                &mut issuer,
                &mut grants
            ),
            Err(SessionSelectionError::InvalidSessionPhase {
                phase: RenaissanceSessionPhase::AwaitingGameReconnect,
            })
        );
        assert_eq!(
            session.phase(),
            RenaissanceSessionPhase::AwaitingGameReconnect
        );
        assert_eq!(issuer.calls, 1);
        assert_eq!(grants.consume(456), Some(ClientVersion::new(5, 0, 8, 3)));
    }
}

#[cfg(test)]
mod reconnect_grant_window_tests {
    use rustuo_core::ClientVersion;
    use rustuo_protocol::RenaissanceServerSelection;

    use super::{
        AuthIdIssuer, ReconnectGrantError, ReconnectGrantWindow, RenaissanceLoginSession,
        RenaissanceServerEndpoint, RenaissanceSessionPhase, SessionSelectionError,
    };

    struct Issuer {
        ids: Vec<Result<u32, &'static str>>,
        calls: usize,
    }

    impl AuthIdIssuer for Issuer {
        type Error = &'static str;

        fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
            self.calls += 1;
            self.ids.remove(0)
        }
    }

    fn version() -> ClientVersion {
        ClientVersion::new(5, 0, 8, 3)
    }

    #[test]
    fn reconnect_grant_window_consumes_once_and_rejects_unknown_ids() {
        let mut grants = ReconnectGrantWindow::new();
        let mut issuer = Issuer {
            ids: vec![Ok(7)],
            calls: 0,
        };
        assert_eq!(grants.consume(404), None);
        assert_eq!(grants.issue(version(), &mut issuer), Ok(7));
        assert_eq!(grants.consume(7), Some(version()));
        assert_eq!(grants.consume(7), None);
    }

    #[test]
    fn reconnect_grant_window_retries_collision_without_overwriting_live_version() {
        let mut grants = ReconnectGrantWindow::new();
        let mut issuer = Issuer {
            ids: vec![Ok(7), Ok(7), Ok(8)],
            calls: 0,
        };
        let other = ClientVersion::new(6, 0, 0, 0);
        assert_eq!(grants.issue(version(), &mut issuer), Ok(7));
        assert_eq!(grants.issue(other, &mut issuer), Ok(8));
        assert_eq!(issuer.calls, 3);
        assert_eq!(grants.consume(7), Some(version()));
        assert_eq!(grants.consume(8), Some(other));
    }

    #[test]
    fn reconnect_grant_window_evicts_oldest_at_capacity() {
        let mut grants = ReconnectGrantWindow::new();
        let mut issuer = Issuer {
            ids: (1..=129).map(Ok).collect(),
            calls: 0,
        };
        for id in 1..=129 {
            assert_eq!(grants.issue(version(), &mut issuer), Ok(id));
        }
        assert_eq!(grants.consume(1), None);
        assert_eq!(grants.consume(2), Some(version()));
        assert_eq!(grants.consume(129), Some(version()));
    }

    #[test]
    fn reconnect_grant_window_issuer_error_preserves_live_grants() {
        let mut grants = ReconnectGrantWindow::new();
        let mut issuer = Issuer {
            ids: vec![Ok(7), Err("offline")],
            calls: 0,
        };
        grants.issue(version(), &mut issuer).unwrap();
        assert_eq!(
            grants.issue(version(), &mut issuer),
            Err(ReconnectGrantError::AuthIdIssuanceFailed("offline"))
        );
        assert_eq!(grants.consume(7), Some(version()));
    }

    #[test]
    fn reconnect_grant_window_exhaustion_preserves_live_grants() {
        let mut grants = ReconnectGrantWindow::new();
        let mut issuer = Issuer {
            ids: vec![Ok(7)],
            calls: 0,
        };
        grants.issue(version(), &mut issuer).unwrap();
        struct CollisionIssuer {
            calls: usize,
        }
        impl AuthIdIssuer for CollisionIssuer {
            type Error = ();
            fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
                self.calls += 1;
                Ok(7)
            }
        }
        let mut collision = CollisionIssuer { calls: 0 };
        assert_eq!(
            grants.issue(version(), &mut collision),
            Err(ReconnectGrantError::CollisionExhausted { attempts: 128 })
        );
        assert_eq!(collision.calls, 128);
        assert_eq!(grants.consume(7), Some(version()));
    }

    #[test]
    fn reconnect_grant_window_selection_registers_version_before_transition() {
        let mut session = RenaissanceLoginSession::new(
            vec![RenaissanceServerEndpoint {
                address: 0x1122_3344,
                port: 0x5566,
            }],
            version(),
        );
        let mut grants = ReconnectGrantWindow::new();
        let mut issuer = Issuer {
            ids: vec![Ok(7)],
            calls: 0,
        };
        let ack = session
            .select_server(
                RenaissanceServerSelection { index: 0 },
                &mut issuer,
                &mut grants,
            )
            .unwrap();
        assert_eq!(
            ack.acknowledgement,
            [0x8C, 0x44, 0x33, 0x22, 0x11, 0x55, 0x66, 0, 0, 0, 7]
        );
        assert_eq!(
            session.phase(),
            RenaissanceSessionPhase::AwaitingGameReconnect
        );
        assert_eq!(grants.consume(7), Some(version()));
    }

    #[test]
    fn reconnect_grant_window_selection_failure_preserves_phase_and_grants() {
        let mut session = RenaissanceLoginSession::new(
            vec![RenaissanceServerEndpoint {
                address: 1,
                port: 2,
            }],
            version(),
        );
        let mut grants = ReconnectGrantWindow::new();
        let mut issuer = Issuer {
            ids: vec![Ok(7)],
            calls: 0,
        };
        grants.issue(version(), &mut issuer).unwrap();
        struct CollisionIssuer;
        impl AuthIdIssuer for CollisionIssuer {
            type Error = ();
            fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
                Ok(7)
            }
        }
        assert_eq!(
            session.select_server(
                RenaissanceServerSelection { index: 0 },
                &mut CollisionIssuer,
                &mut grants
            ),
            Err(SessionSelectionError::ReconnectGrantFailed(
                ReconnectGrantError::CollisionExhausted { attempts: 128 }
            ))
        );
        assert_eq!(session.phase(), RenaissanceSessionPhase::Authenticated);
        assert_eq!(grants.consume(7), Some(version()));
    }
}
