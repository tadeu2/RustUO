//! Transport-neutral composition of the Renaissance login and reconnect path.

use rustuo_core::ClientVersion;
use rustuo_protocol::{
    decode_renaissance_account_login, decode_renaissance_game_login,
    decode_renaissance_server_selection, encode_renaissance_account_login_ack,
    AccountLoginAckEncodeError, AccountLoginDecodeError, GameLoginDecodeError,
    RenaissanceServerListEntry, ServerSelectionDecodeError,
};

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
}

/// Shared application state; a transport supplies one frame at a time and sends
/// the returned bytes on the appropriate connection.
pub struct RenaissanceLoginFlow {
    verifier: RepositoryCredentialVerifier<LegacyXmlAccountRepository>,
    server_name: Vec<u8>,
    endpoint: RenaissanceServerEndpoint,
    client_version: ClientVersion,
    login_session: Option<RenaissanceLoginSession>,
    grants: ReconnectGrantWindow,
}

impl RenaissanceLoginFlow {
    pub fn new(
        repository: LegacyXmlAccountRepository,
        server_name: Vec<u8>,
        endpoint: RenaissanceServerEndpoint,
        client_version: ClientVersion,
    ) -> Self {
        Self {
            verifier: RepositoryCredentialVerifier::new(repository),
            server_name,
            endpoint,
            client_version,
            login_session: None,
            grants: ReconnectGrantWindow::new(),
        }
    }

    pub fn handle_account_login(&mut self, frame: &[u8]) -> Result<Vec<u8>, LoginFlowError> {
        if self.login_session.is_some() {
            return Err(LoginFlowError::InvalidLoginPhase);
        }
        let login =
            decode_renaissance_account_login(frame).map_err(LoginFlowError::AccountLoginDecode)?;
        match self.verifier.verify(login.username, login.password) {
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
        session
            .select_server(selection, issuer, &mut self.grants)
            .map(|prepared| prepared.acknowledgement)
            .map_err(LoginFlowError::Selection)
    }

    pub fn handle_game_login(
        &mut self,
        frame: &[u8],
    ) -> Result<RenaissanceReconnectAdmission<LegacyAccountIdentity>, LoginFlowError> {
        let login =
            decode_renaissance_game_login(frame).map_err(LoginFlowError::GameLoginDecode)?;
        admit_renaissance_reconnect(login, &mut self.grants, &mut self.verifier)
            .map_err(LoginFlowError::Reconnect)
    }
}
