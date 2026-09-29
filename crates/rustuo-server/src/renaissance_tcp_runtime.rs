use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use rustuo_core::ClientVersion;

use crate::account_repository::LegacyXmlAccountRepository;
use crate::renaissance_login_flow::{LoginFlowError, RenaissanceLoginFlow};
use crate::{AuthIdIssuer, RenaissanceReconnectAdmission, RenaissanceServerEndpoint};

const SOCKET_TIMEOUT: Duration = Duration::from_secs(5);
const ACCOUNT_LOGIN_LENGTH: usize = 62;
const GAME_LOGIN_LENGTH: usize = 65;

#[derive(Debug)]
pub enum TcpRuntimeError<E> {
    Io(io::Error),
    InvalidSeed,
    UnsupportedFirstPacket(u8),
    AccountLogin(LoginFlowError<()>),
    ServerSelection(LoginFlowError<E>),
    GameLogin(LoginFlowError<()>),
}

pub struct RenaissanceTcpRuntime {
    listener: TcpListener,
    flow: RenaissanceLoginFlow,
}

impl RenaissanceTcpRuntime {
    pub fn bind(
        address: SocketAddr,
        repository: LegacyXmlAccountRepository,
        server_name: Vec<u8>,
        advertised_address: u32,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        let endpoint = RenaissanceServerEndpoint {
            address: advertised_address,
            port: listener.local_addr()?.port(),
        };
        Ok(Self {
            listener,
            flow: RenaissanceLoginFlow::new(
                repository,
                server_name,
                endpoint,
                ClientVersion::new(5, 0, 8, 3),
            ),
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Accepts and processes one client. Successful game admission returns the
    /// still-open game socket; login and redirect sockets return `None`.
    pub fn serve_next<I: AuthIdIssuer>(
        &self,
        issuer: &mut I,
    ) -> Result<
        Option<(
            TcpStream,
            RenaissanceReconnectAdmission<crate::account_repository::LegacyAccountIdentity>,
        )>,
        TcpRuntimeError<I::Error>,
    > {
        let (stream, _) = self.listener.accept().map_err(TcpRuntimeError::Io)?;
        stream
            .set_read_timeout(Some(SOCKET_TIMEOUT))
            .map_err(TcpRuntimeError::Io)?;
        stream
            .set_write_timeout(Some(SOCKET_TIMEOUT))
            .map_err(TcpRuntimeError::Io)?;
        let mut connection = self.flow.new_connection();
        let mut seed = [0; 4];
        let mut stream = stream;
        stream.read_exact(&mut seed).map_err(TcpRuntimeError::Io)?;
        let seed = u32::from_be_bytes(seed);
        if seed == 0 {
            return Err(TcpRuntimeError::InvalidSeed);
        }
        let mut packet_id = [0];
        stream
            .read_exact(&mut packet_id)
            .map_err(TcpRuntimeError::Io)?;
        match packet_id[0] {
            0x80 => {
                let mut frame = [0; ACCOUNT_LOGIN_LENGTH];
                frame[0] = packet_id[0];
                stream
                    .read_exact(&mut frame[1..])
                    .map_err(TcpRuntimeError::Io)?;
                let response = connection
                    .handle_account_login(&frame)
                    .map_err(TcpRuntimeError::AccountLogin)?;
                stream.write_all(&response).map_err(TcpRuntimeError::Io)?;
                if response.first() == Some(&0x82) {
                    return Ok(None);
                }
                let mut selection = [0; 3];
                stream
                    .read_exact(&mut selection)
                    .map_err(TcpRuntimeError::Io)?;
                let redirect = connection
                    .handle_server_selection(&selection, issuer)
                    .map_err(TcpRuntimeError::ServerSelection)?;
                stream.write_all(&redirect).map_err(TcpRuntimeError::Io)?;
                Ok(None)
            }
            0x91 => {
                let mut frame = [0; GAME_LOGIN_LENGTH];
                frame[0] = packet_id[0];
                stream
                    .read_exact(&mut frame[1..])
                    .map_err(TcpRuntimeError::Io)?;
                let admission = connection
                    .handle_game_login_for_seed(seed, &frame)
                    .map_err(TcpRuntimeError::GameLogin)?;
                Ok(Some((stream, admission)))
            }
            id => Err(TcpRuntimeError::UnsupportedFirstPacket(id)),
        }
    }
}
