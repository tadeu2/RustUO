//! Transport-neutral composition of the Renaissance login and reconnect path.

use rustuo_core::ClientVersion;
use rustuo_protocol::{
    decode_renaissance_account_login, decode_renaissance_game_login,
    decode_renaissance_server_selection, encode_renaissance_account_login_ack,
    AccountLoginAckEncodeError, AccountLoginDecodeError, GameLoginDecodeError,
    RenaissanceServerListEntry, ServerSelectionDecodeError,
};
use std::sync::{Arc, Mutex};

use crate::account_repository::{
    LegacyAccountError, LegacyAccountIdentity, LegacyXmlAccountRepository,
    RepositoryCredentialVerifier,
};
use crate::{
    admit_renaissance_reconnect, AuthIdIssuer, ReconnectAdmissionError,
    ReconnectCredentialVerifier, ReconnectGrantWindow, RenaissanceLoginSession,
    RenaissanceReconnectAdmission, RenaissanceServerEndpoint, SessionSelectionError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginFlowError<E = ()> {
    AccountLoginDecode(AccountLoginDecodeError),
    AccountLoginEncode(AccountLoginAckEncodeError),
    Verification(LegacyAccountError),
    InvalidLoginPhase,
    NotAuthenticated,
    ServerSelectionDecode(ServerSelectionDecodeError),
    Selection(SessionSelectionError<E>),
    GameLoginDecode(GameLoginDecodeError),
    Reconnect(ReconnectAdmissionError<LegacyAccountError>),
    SeedMismatch { connection_seed: u32, auth_id: u32 },
    SharedStatePoisoned,
}

struct SharedLoginState {
    verifier: RepositoryCredentialVerifier<LegacyXmlAccountRepository>,
    grants: ReconnectGrantWindow,
}

/// Shared application state; a transport supplies one frame at a time and sends
/// the returned bytes on the appropriate connection.
pub struct RenaissanceLoginFlow {
    shared: Arc<Mutex<SharedLoginState>>,
    server_name: Vec<u8>,
    endpoint: RenaissanceServerEndpoint,
    client_version: ClientVersion,
    login_session: Option<RenaissanceLoginSession>,
}

impl RenaissanceLoginFlow {
    pub fn new(
        repository: LegacyXmlAccountRepository,
        server_name: Vec<u8>,
        endpoint: RenaissanceServerEndpoint,
        client_version: ClientVersion,
    ) -> Self {
        Self {
            shared: Arc::new(Mutex::new(SharedLoginState {
                verifier: RepositoryCredentialVerifier::new(repository),
                grants: ReconnectGrantWindow::new(),
            })),
            server_name,
            endpoint,
            client_version,
            login_session: None,
        }
    }

    /// Creates fresh connection-local login state while retaining process-wide
    /// credential verification and one-shot reconnect grants.
    pub fn new_connection(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            server_name: self.server_name.clone(),
            endpoint: self.endpoint,
            client_version: self.client_version,
            login_session: None,
        }
    }

    pub fn handle_account_login(&mut self, frame: &[u8]) -> Result<Vec<u8>, LoginFlowError> {
        if self.login_session.is_some() {
            return Err(LoginFlowError::InvalidLoginPhase);
        }
        let login =
            decode_renaissance_account_login(frame).map_err(LoginFlowError::AccountLoginDecode)?;
        let mut shared = self
            .shared
            .lock()
            .map_err(|_| LoginFlowError::SharedStatePoisoned)?;
        match shared.verifier.verify(login.username, login.password) {
            Ok(_) => {
                let response =
                    encode_renaissance_account_login_ack(&[RenaissanceServerListEntry {
                        name: &self.server_name,
                        full_percent: 0,
                        timezone: 0,
                        address: self.endpoint.address,
                    }])
                    .map_err(LoginFlowError::AccountLoginEncode)?;
                self.login_session = Some(RenaissanceLoginSession::new(
                    vec![self.endpoint],
                    self.client_version,
                ));
                Ok(response)
            }
            Err(LegacyAccountError::UnknownUsername) => Ok(vec![0x82, 0x00]),
            Err(LegacyAccountError::PasswordMismatch) => Ok(vec![0x82, 0x03]),
            Err(error) => Err(LoginFlowError::Verification(error)),
        }
    }

    pub fn handle_server_selection<I: AuthIdIssuer>(
        &mut self,
        frame: &[u8],
        issuer: &mut I,
    ) -> Result<Vec<u8>, LoginFlowError<I::Error>> {
        let session = self
            .login_session
            .as_mut()
            .ok_or(LoginFlowError::NotAuthenticated)?;
        let selection = decode_renaissance_server_selection(frame)
            .map_err(LoginFlowError::ServerSelectionDecode)?;
        let mut shared = self
            .shared
            .lock()
            .map_err(|_| LoginFlowError::SharedStatePoisoned)?;
        session
            .select_server(selection, issuer, &mut shared.grants)
            .map(|prepared| prepared.acknowledgement)
            .map_err(LoginFlowError::Selection)
    }

    pub fn handle_game_login(
        &mut self,
        frame: &[u8],
    ) -> Result<RenaissanceReconnectAdmission<LegacyAccountIdentity>, LoginFlowError> {
        let login =
            decode_renaissance_game_login(frame).map_err(LoginFlowError::GameLoginDecode)?;
        let mut shared = self
            .shared
            .lock()
            .map_err(|_| LoginFlowError::SharedStatePoisoned)?;
        let SharedLoginState { grants, verifier } = &mut *shared;
        admit_renaissance_reconnect(login, grants, verifier).map_err(LoginFlowError::Reconnect)
    }

    pub fn handle_game_login_for_seed(
        &mut self,
        connection_seed: u32,
        frame: &[u8],
    ) -> Result<RenaissanceReconnectAdmission<LegacyAccountIdentity>, LoginFlowError> {
        let login =
            decode_renaissance_game_login(frame).map_err(LoginFlowError::GameLoginDecode)?;
        if connection_seed == 0 || connection_seed != login.auth_id {
            return Err(LoginFlowError::SeedMismatch {
                connection_seed,
                auth_id: login.auth_id,
            });
        }
        let mut shared = self
            .shared
            .lock()
            .map_err(|_| LoginFlowError::SharedStatePoisoned)?;
        let SharedLoginState { grants, verifier } = &mut *shared;
        admit_renaissance_reconnect(login, grants, verifier).map_err(LoginFlowError::Reconnect)
    }
}
