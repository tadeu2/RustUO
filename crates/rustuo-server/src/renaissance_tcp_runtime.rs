use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use rustuo_core::ClientVersion;
use rustuo_protocol::{
    compress_legacy_packet, decode_renaissance_ping_request,
    decode_renaissance_update_range_request, encode_renaissance_ping_ack,
    encode_renaissance_update_range_response, RenaissancePingDecodeError,
    RenaissanceUpdateRangeDecodeError,
};

use crate::account_repository::LegacyXmlAccountRepository;
use crate::renaissance_login_flow::{LoginFlowError, RenaissanceLoginFlow};
use crate::renaissance_world_entry_session::{
    AvatarPresentation, RenaissanceLoginTailFixture, RenaissanceWorldEntrySession, WorldEntryError,
};
use crate::{AuthIdIssuer, RenaissanceReconnectAdmission, RenaissanceServerEndpoint};

const SOCKET_TIMEOUT: Duration = Duration::from_secs(5);
const ACCOUNT_LOGIN_LENGTH: usize = 62;
const GAME_LOGIN_LENGTH: usize = 65;
const PLAY_CHARACTER_LENGTH: usize = 73;
const MOVEMENT_LENGTH: usize = 7;
const PING_LENGTH: usize = 2;
const UPDATE_RANGE_LENGTH: usize = 2;
const DEFAULT_UPDATE_RANGE: u8 = 18;
const MAX_UPDATE_RANGE: u8 = 24;

#[derive(Debug)]
pub enum TcpRuntimeError<E> {
    Accept(io::Error),
    Io(io::Error),
    InvalidSeed,
    UnsupportedFirstPacket(u8),
    AccountLogin(LoginFlowError<()>),
    ServerSelection(LoginFlowError<E>),
    GameLogin(LoginFlowError<()>),
    WorldEntry(WorldEntryError),
    PingDecode(RenaissancePingDecodeError),
    UpdateRangeDecode(RenaissanceUpdateRangeDecodeError),
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
        let (stream, _) = self.listener.accept().map_err(TcpRuntimeError::Accept)?;
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

    /// Completes reconnect, character selection, seeded-world login, and one movement request.
    /// Login and redirect connections return `None`; successful world-entry answers ping frames
    /// until the first complete movement frame, then returns the same open game socket.
    pub fn serve_world_entry_next<I: AuthIdIssuer>(
        &self,
        issuer: &mut I,
        feature_flags: u16,
        avatar: AvatarPresentation,
        login_tail: &RenaissanceLoginTailFixture,
    ) -> Result<Option<(TcpStream, RenaissanceWorldEntrySession)>, TcpRuntimeError<I::Error>> {
        let Some((mut stream, admission)) = self.serve_next(issuer)? else {
            return Ok(None);
        };
        let mut session = RenaissanceWorldEntrySession::new(admission, feature_flags, avatar);
        for packet in session
            .reconnect_packets()
            .map_err(TcpRuntimeError::WorldEntry)?
        {
            stream.write_all(&packet).map_err(TcpRuntimeError::Io)?;
        }

        let mut play_character = [0; PLAY_CHARACTER_LENGTH];
        stream
            .read_exact(&mut play_character)
            .map_err(TcpRuntimeError::Io)?;
        let confirmation = session
            .play_character(&play_character)
            .map_err(TcpRuntimeError::WorldEntry)?;
        stream
            .write_all(&confirmation)
            .map_err(TcpRuntimeError::Io)?;

        for packet in session
            .map_setup_packets()
            .map_err(TcpRuntimeError::WorldEntry)?
        {
            stream.write_all(&packet).map_err(TcpRuntimeError::Io)?;
        }
        for packet in session
            .initial_self_mobile_packets()
            .map_err(TcpRuntimeError::WorldEntry)?
        {
            stream.write_all(&packet).map_err(TcpRuntimeError::Io)?;
        }
        for packet in session
            .first_send_everything_packets()
            .map_err(TcpRuntimeError::WorldEntry)?
        {
            stream.write_all(&packet).map_err(TcpRuntimeError::Io)?;
        }
        for packet in session
            .post_first_send_everything_packets(login_tail)
            .map_err(TcpRuntimeError::WorldEntry)?
        {
            stream.write_all(&packet).map_err(TcpRuntimeError::Io)?;
        }

        let mut update_range = DEFAULT_UPDATE_RANGE;
        loop {
            let mut packet_id = [0];
            stream
                .read_exact(&mut packet_id)
                .map_err(TcpRuntimeError::Io)?;

            if packet_id[0] == 0x73 {
                let mut ping = [0; PING_LENGTH];
                ping[0] = packet_id[0];
                stream
                    .read_exact(&mut ping[1..])
                    .map_err(TcpRuntimeError::Io)?;
                let request =
                    decode_renaissance_ping_request(&ping).map_err(TcpRuntimeError::PingDecode)?;
                let reply = compress_legacy_packet(&encode_renaissance_ping_ack(request.sequence))
                    .map_err(|error| {
                        TcpRuntimeError::WorldEntry(WorldEntryError::Compression(error))
                    })?;
                stream.write_all(&reply).map_err(TcpRuntimeError::Io)?;
                continue;
            }

            if packet_id[0] == 0xC8 {
                let mut update_range_packet = [0; UPDATE_RANGE_LENGTH];
                update_range_packet[0] = packet_id[0];
                stream
                    .read_exact(&mut update_range_packet[1..])
                    .map_err(TcpRuntimeError::Io)?;
                let request = decode_renaissance_update_range_request(&update_range_packet)
                    .map_err(TcpRuntimeError::UpdateRangeDecode)?;
                let effective_range = request.range.clamp(DEFAULT_UPDATE_RANGE, MAX_UPDATE_RANGE);
                if effective_range == update_range {
                    continue;
                }

                update_range = effective_range;
                let response = encode_renaissance_update_range_response(update_range);
                let reply = compress_legacy_packet(&response).map_err(|error| {
                    TcpRuntimeError::WorldEntry(WorldEntryError::Compression(error))
                })?;
                stream.write_all(&reply).map_err(TcpRuntimeError::Io)?;
                continue;
            }

            let mut movement = [0; MOVEMENT_LENGTH];
            movement[0] = packet_id[0];
            stream
                .read_exact(&mut movement[1..])
                .map_err(TcpRuntimeError::Io)?;
            let reply = session
                .movement_request(&movement)
                .map_err(TcpRuntimeError::WorldEntry)?;
            stream.write_all(&reply).map_err(TcpRuntimeError::Io)?;
            break;
        }

        Ok(Some((stream, session)))
    }
}
