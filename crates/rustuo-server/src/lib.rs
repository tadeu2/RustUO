#![forbid(unsafe_code)]

pub mod account_repository;

use rustuo_protocol::{encode_renaissance_play_server_ack, RenaissanceServerSelection};

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
    phase: RenaissanceSessionPhase,
}

impl RenaissanceLoginSession {
    pub fn new(offered_servers: Vec<RenaissanceServerEndpoint>) -> Self {
        Self {
            offered_servers,
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

        let auth_id = issuer
            .issue_auth_id()
            .map_err(SessionSelectionError::AuthIdIssuanceFailed)?;
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
    AuthIdIssuanceFailed(E),
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
    use rustuo_protocol::RenaissanceServerSelection;

    use super::{
        AuthIdIssuer, RenaissanceLoginSession, RenaissanceServerEndpoint, RenaissanceSessionPhase,
        SessionSelectionError,
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
        RenaissanceLoginSession::new(vec![
            RenaissanceServerEndpoint {
                address: 0x1122_3344,
                port: 0x5566,
            },
            RenaissanceServerEndpoint {
                address: 0xAABB_CCDD,
                port: 0x7788,
            },
        ])
    }

    #[test]
    fn session_selection_returns_endpoint_auth_id_ack_and_advances_phase() {
        let mut session = session();
        let mut issuer = FixedIssuer {
            result: Ok(0x7788_99AA),
            calls: 0,
        };

        let prepared = session
            .select_server(RenaissanceServerSelection { index: 1 }, &mut issuer)
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
            assert_eq!(
                session.select_server(RenaissanceServerSelection { index }, &mut issuer),
                Err(expected)
            );
            assert_eq!(session.phase(), RenaissanceSessionPhase::Authenticated);
            assert_eq!(issuer.calls, 0);
        }
    }

    #[test]
    fn session_selection_issuer_failure_keeps_session_authenticated() {
        let mut session = session();
        let mut issuer = FixedIssuer {
            result: Err("unavailable"),
            calls: 0,
        };

        assert_eq!(
            session.select_server(RenaissanceServerSelection { index: 0 }, &mut issuer),
            Err(SessionSelectionError::AuthIdIssuanceFailed("unavailable"))
        );
        assert_eq!(session.phase(), RenaissanceSessionPhase::Authenticated);
        assert_eq!(issuer.calls, 1);
    }

    #[test]
    fn session_selection_rejects_a_second_attempt_without_reissuing_auth() {
        let mut session = session();
        let mut issuer = FixedIssuer {
            result: Ok(456),
            calls: 0,
        };
        session
            .select_server(RenaissanceServerSelection { index: 0 }, &mut issuer)
            .unwrap();

        assert_eq!(
            session.select_server(RenaissanceServerSelection { index: 1 }, &mut issuer),
            Err(SessionSelectionError::InvalidSessionPhase {
                phase: RenaissanceSessionPhase::AwaitingGameReconnect,
            })
        );
        assert_eq!(
            session.phase(),
            RenaissanceSessionPhase::AwaitingGameReconnect
        );
        assert_eq!(issuer.calls, 1);
    }
}
