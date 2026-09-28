//! Per-reconnect, transport-neutral first-packet world-entry composition.

use rustuo_core::ClientVersion;
use rustuo_protocol::{
    compress_legacy_packet, decode_renaissance_play_character_slot,
    encode_renaissance_login_confirm, encode_renaissance_map_change,
    encode_renaissance_map_patches, encode_renaissance_mobile_incoming_empty,
    encode_renaissance_mobile_update_old, encode_renaissance_old_character_list,
    encode_renaissance_supported_features, CompressionError, OldCharacterListEncodeError,
    PlayCharacterSlotDecodeError,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldEntryError {
    InvalidPhase,
    CharacterList(OldCharacterListEncodeError),
    Compression(CompressionError),
    Decode(PlayCharacterSlotDecodeError),
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
}

/// The admission, avatar presentation, and seeded world share one session lifetime.
pub struct RenaissanceWorldEntrySession {
    admission: RenaissanceReconnectAdmission<LegacyAccountIdentity>,
    world: World,
    avatar: AvatarPresentation,
    feature_flags: u16,
    phase: Phase,
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
