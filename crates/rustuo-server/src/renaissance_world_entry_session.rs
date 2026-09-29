//! Per-reconnect, transport-neutral first-packet world-entry composition.

use rustuo_core::ClientVersion;
use rustuo_core::Direction;
use rustuo_protocol::{
    compress_legacy_packet, decode_renaissance_movement_request,
    decode_renaissance_play_character_slot, encode_renaissance_aos_mobile_status,
    encode_renaissance_current_time, encode_renaissance_global_light,
    encode_renaissance_login_complete, encode_renaissance_login_confirm,
    encode_renaissance_map_change, encode_renaissance_map_patches,
    encode_renaissance_mobile_incoming_empty, encode_renaissance_mobile_update_old,
    encode_renaissance_movement_ack, encode_renaissance_movement_rejection,
    encode_renaissance_old_character_list, encode_renaissance_personal_light,
    encode_renaissance_season_change, encode_renaissance_supported_features,
    encode_renaissance_war_mode, CompressionError, OldCharacterListEncodeError,
    PlayCharacterSlotDecodeError, RenaissanceMovementDecodeError,
};
use rustuo_world::World;

use crate::account_repository::LegacyAccountIdentity;
use crate::RenaissanceReconnectAdmission;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarPresentation {
    pub name: Vec<u8>,
    pub body: u16,
    pub direction: u8,
    pub hue: u16,
    pub old_flags: u8,
    pub notoriety: u8,
}

/// Explicit login-tail values absent from the one-mobile world fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenaissanceLoginTailFixture {
    pub global_light: u8,
    pub personal_light: u8,
    pub status: rustuo_protocol::RenaissanceAosMobileStatus,
    pub war_mode: bool,
    pub season: u8,
    /// UTC hour, minute, second, supplied by the caller rather than read here.
    pub current_time: (u8, u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldEntryError {
    InvalidPhase,
    CharacterList(OldCharacterListEncodeError),
    Compression(CompressionError),
    Decode(PlayCharacterSlotDecodeError),
    MovementDecode(RenaissanceMovementDecodeError),
    UnsupportedSlot(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    AwaitingList,
    AwaitingSlot,
    Confirmed,
    MapSetupSent,
    InitialMobileSent,
    FirstEverythingSent,
    LoginTailSent,
}

/// The admission, avatar presentation, and seeded world share one session lifetime.
pub struct RenaissanceWorldEntrySession {
    admission: RenaissanceReconnectAdmission<LegacyAccountIdentity>,
    world: World,
    avatar: AvatarPresentation,
    feature_flags: u16,
    phase: Phase,
    movement_sequence: u8,
}

impl RenaissanceWorldEntrySession {
    pub fn new(
        admission: RenaissanceReconnectAdmission<LegacyAccountIdentity>,
        feature_flags: u16,
        avatar: AvatarPresentation,
    ) -> Self {
        Self {
            admission,
            world: World::renaissance_client_fixture(),
            avatar,
            feature_flags,
            phase: Phase::AwaitingList,
            movement_sequence: 0,
        }
    }

    pub fn account(&self) -> &LegacyAccountIdentity {
        &self.admission.account
    }

    pub fn client_version(&self) -> ClientVersion {
        self.admission.client_version
    }

    pub fn reconnect_packets(&mut self) -> Result<[Vec<u8>; 2], WorldEntryError> {
        if self.phase != Phase::AwaitingList {
            return Err(WorldEntryError::InvalidPhase);
        }
        let frame = encode_renaissance_old_character_list(&self.avatar.name)
            .map_err(WorldEntryError::CharacterList)?;
        let features = encode_renaissance_supported_features(self.feature_flags);
        let features = compress_legacy_packet(&features).map_err(WorldEntryError::Compression)?;
        let frame = compress_legacy_packet(&frame).map_err(WorldEntryError::Compression)?;
        self.phase = Phase::AwaitingSlot;
        Ok([features, frame])
    }

    pub fn play_character(&mut self, frame: &[u8]) -> Result<Vec<u8>, WorldEntryError> {
        if self.phase != Phase::AwaitingSlot {
            return Err(WorldEntryError::InvalidPhase);
        }
        let slot =
            decode_renaissance_play_character_slot(frame).map_err(WorldEntryError::Decode)?;
        if slot != 0 {
            return Err(WorldEntryError::UnsupportedSlot(slot));
        }

        let player = self.world.player();
        let position = player.position();
        let (width, height) = self.world.map_size();
        let confirmation = encode_renaissance_login_confirm(
            player.id().serial().0,
            self.avatar.body,
            u16::try_from(position.x).expect("seeded player x fits wire"),
            u16::try_from(position.y).expect("seeded player y fits wire"),
            i16::try_from(position.z).expect("seeded player z fits wire"),
            self.avatar.direction,
            u16::try_from(width).expect("seeded map width fits wire"),
            u16::try_from(height).expect("seeded map height fits wire"),
        );
        let confirmation =
            compress_legacy_packet(&confirmation).map_err(WorldEntryError::Compression)?;
        self.phase = Phase::Confirmed;
        Ok(confirmation)
    }

    /// Emits only the post-LoginConfirm map setup prefix for the seeded fixture.
    pub fn map_setup_packets(&mut self) -> Result<[Vec<u8>; 3], WorldEntryError> {
        if self.phase != Phase::Confirmed {
            return Err(WorldEntryError::InvalidPhase);
        }
        let map_change = encode_renaissance_map_change(self.world.map_id().raw());
        // The seeded world has no patch store or loaded MUL patch files. These
        // zero counts describe this fixture only, not a general server default.
        let map_patches = encode_renaissance_map_patches([(0, 0); 4]);
        let features = encode_renaissance_supported_features(self.feature_flags);
        let frames = [
            compress_legacy_packet(&map_change).map_err(WorldEntryError::Compression)?,
            compress_legacy_packet(&map_patches).map_err(WorldEntryError::Compression)?,
            compress_legacy_packet(&features).map_err(WorldEntryError::Compression)?,
        ];
        self.phase = Phase::MapSetupSent;
        Ok(frames)
    }

    /// Emits only the first self-mobile prefix, before SendEverything and light updates.
    pub fn initial_self_mobile_packets(&mut self) -> Result<[Vec<u8>; 2], WorldEntryError> {
        if self.phase != Phase::MapSetupSent {
            return Err(WorldEntryError::InvalidPhase);
        }
        let player = self.world.player();
        let position = player.position();
        let serial = player.id().serial().0;
        let x = u16::try_from(position.x).expect("seeded player x fits wire");
        let y = u16::try_from(position.y).expect("seeded player y fits wire");
        let z = i8::try_from(position.z).expect("seeded player z fits old wire");
        let incoming = self.encode_fixture_player_incoming()?;
        let update = encode_renaissance_mobile_update_old(
            serial,
            self.avatar.body,
            x,
            y,
            z,
            self.avatar.direction,
            self.avatar.hue,
            self.avatar.old_flags,
        );
        let frames = [
            incoming,
            compress_legacy_packet(&update).map_err(WorldEntryError::Compression)?,
        ];
        self.phase = Phase::InitialMobileSent;
        Ok(frames)
    }

    /// Emits the first SendEverything pass for the one-mobile, zero-item fixture.
    /// The legacy range enumeration includes the player itself.
    pub fn first_send_everything_packets(&mut self) -> Result<[Vec<u8>; 1], WorldEntryError> {
        if self.phase != Phase::InitialMobileSent {
            return Err(WorldEntryError::InvalidPhase);
        }
        let incoming = self.encode_fixture_player_incoming()?;
        self.phase = Phase::FirstEverythingSent;
        Ok([incoming])
    }

    /// Completes the bounded fixture's DoLogin tail, including second SendEverything.
    /// Light and status values are fixture inputs, not derived gameplay behavior.
    pub fn post_first_send_everything_packets(
        &mut self,
        fixture: &RenaissanceLoginTailFixture,
    ) -> Result<[Vec<u8>; 10], WorldEntryError> {
        if self.phase != Phase::FirstEverythingSent {
            return Err(WorldEntryError::InvalidPhase);
        }
        let serial = self.world.player().id().serial().0;
        let status = encode_renaissance_aos_mobile_status(
            serial,
            &String::from_utf8_lossy(&self.avatar.name),
            &fixture.status,
        );
        let (hour, minute, second) = fixture.current_time;
        let compress =
            |frame: &[u8]| compress_legacy_packet(frame).map_err(WorldEntryError::Compression);
        // PacketHandlers.DoLogin: forced light, complete, self, status, war,
        // season, UTC time, map, then second SendEverything. The fixture has
        // no login-event side effects, items, equipment, or fastwalk stack.
        let frames = [
            compress(&encode_renaissance_global_light(fixture.global_light))?,
            compress(&encode_renaissance_personal_light(
                serial,
                fixture.personal_light,
            ))?,
            compress(&encode_renaissance_login_complete())?,
            self.encode_fixture_player_incoming()?,
            compress(&status)?,
            compress(&encode_renaissance_war_mode(fixture.war_mode))?,
            compress(&encode_renaissance_season_change(fixture.season))?,
            compress(&encode_renaissance_current_time(hour, minute, second))?,
            compress(&encode_renaissance_map_change(self.world.map_id().raw()))?,
            self.encode_fixture_player_incoming()?,
        ];
        self.phase = Phase::LoginTailSent;
        Ok(frames)
    }

    /// Handles one 0x02 movement request for the bounded, map-bounds-only fixture.
    pub fn movement_request(&mut self, frame: &[u8]) -> Result<Vec<u8>, WorldEntryError> {
        if self.phase != Phase::LoginTailSent {
            return Err(WorldEntryError::InvalidPhase);
        }
        let request =
            decode_renaissance_movement_request(frame).map_err(WorldEntryError::MovementDecode)?;
        let sequence_rejected = self.movement_sequence == 0 && request.sequence != 0;
        let same_facing = self.avatar.direction & 0x07 == request.direction & 0x07;
        let moved = if sequence_rejected || !same_facing {
            !sequence_rejected
        } else {
            let direction = Direction::from_raw(request.direction & 0x07)
                .expect("masked Renaissance direction is valid");
            let player_id = self.world.player().id();
            self.world
                .decide_move(player_id, direction)
                .ok()
                .and_then(|decision| self.world.apply_move(decision).ok())
                .is_some()
        };

        if !moved {
            self.movement_sequence = 0;
            let player = self.world.player();
            let position = player.position();
            let packet = encode_renaissance_movement_rejection(
                request.sequence,
                u16::try_from(position.x).expect("fixture player x fits wire"),
                u16::try_from(position.y).expect("fixture player y fits wire"),
                self.avatar.direction,
                i8::try_from(position.z).expect("fixture player z fits wire"),
            );
            return compress_legacy_packet(&packet).map_err(WorldEntryError::Compression);
        }

        self.avatar.direction = request.direction;
        let acknowledgment =
            encode_renaissance_movement_ack(self.movement_sequence, self.avatar.notoriety);
        self.movement_sequence = if request.sequence == u8::MAX {
            1
        } else {
            request.sequence + 1
        };
        compress_legacy_packet(&acknowledgment).map_err(WorldEntryError::Compression)
    }

    fn encode_fixture_player_incoming(&self) -> Result<Vec<u8>, WorldEntryError> {
        let player = self.world.player();
        let position = player.position();
        let incoming = encode_renaissance_mobile_incoming_empty(
            player.id().serial().0,
            self.avatar.body,
            u16::try_from(position.x).expect("seeded player x fits wire"),
            u16::try_from(position.y).expect("seeded player y fits wire"),
            i8::try_from(position.z).expect("seeded player z fits old wire"),
            self.avatar.direction,
            self.avatar.hue,
            self.avatar.old_flags,
            self.avatar.notoriety,
        );
        compress_legacy_packet(&incoming).map_err(WorldEntryError::Compression)
    }
}
