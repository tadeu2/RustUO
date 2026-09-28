#![forbid(unsafe_code)]

//! Ultima Online protocol boundary.

use rustuo_core::Serial;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityReference {
    pub serial: Serial,
}

#[cfg(test)]
mod packet_input_buffer_tests {
    use super::{PacketFrameError, PacketInputBuffer, PacketLayout, PacketLayoutTable};

    fn layouts() -> PacketLayoutTable {
        let mut layouts = PacketLayoutTable::new();
        layouts
            .register(0x21, PacketLayout::Fixed { length: 2 })
            .unwrap();
        layouts.register(0xBD, PacketLayout::Variable).unwrap();
        layouts
    }

    #[test]
    fn split_fixed_frame_is_emitted_only_when_complete() {
        let mut input = PacketInputBuffer::new();
        let layouts = layouts();
        assert_eq!(input.append(&[0x21], &layouts), Ok(vec![]));
        assert_eq!(input.pending_len(), 1);
        assert_eq!(input.append(&[0xAA], &layouts), Ok(vec![vec![0x21, 0xAA]]));
        assert_eq!(input.pending_len(), 0);
    }

    #[test]
    fn split_variable_header_and_body_are_retained_exactly() {
        let mut input = PacketInputBuffer::new();
        let layouts = layouts();
        for (chunk, pending_len) in [(&[0xBD][..], 1), (&[0, 5][..], 3), (&[0xAA][..], 4)] {
            assert_eq!(input.append(chunk, &layouts), Ok(vec![]));
            assert_eq!(input.pending_len(), pending_len);
        }
        assert_eq!(
            input.append(&[0xBB], &layouts),
            Ok(vec![vec![0xBD, 0, 5, 0xAA, 0xBB]])
        );
        assert_eq!(input.pending_len(), 0);
    }

    #[test]
    fn multiple_owned_frames_leave_exact_partial_suffix_for_next_append() {
        let mut input = PacketInputBuffer::new();
        let layouts = layouts();
        assert_eq!(
            input.append(&[0x21, 0xAA, 0xBD, 0, 3, 0x21], &layouts),
            Ok(vec![vec![0x21, 0xAA], vec![0xBD, 0, 3]])
        );
        assert_eq!(input.pending_len(), 1);
        assert_eq!(input.append(&[0xBB], &layouts), Ok(vec![vec![0x21, 0xBB]]));
        assert_eq!(input.pending_len(), 0);
    }

    #[test]
    fn empty_chunks_do_not_change_pending_or_return_frames() {
        let mut input = PacketInputBuffer::new();
        let layouts = layouts();
        assert_eq!(input.append(&[], &layouts), Ok(vec![]));
        assert_eq!(input.append(&[0x21], &layouts), Ok(vec![]));
        assert_eq!(input.append(&[], &layouts), Ok(vec![]));
        assert_eq!(input.pending_len(), 1);
        assert_eq!(input.append(&[0xAA], &layouts), Ok(vec![vec![0x21, 0xAA]]));
    }

    #[test]
    fn returned_frame_owns_bytes_after_future_appends() {
        let mut input = PacketInputBuffer::new();
        let layouts = layouts();
        let first = input.append(&[0x21, 0xAA], &layouts).unwrap();
        assert_eq!(
            input.append(&[0x21, 0xBB], &layouts),
            Ok(vec![vec![0x21, 0xBB]])
        );
        assert_eq!(first, vec![vec![0x21, 0xAA]]);
    }

    #[test]
    fn unknown_packet_after_complete_prefix_rolls_back_pending_and_frames() {
        let mut input = PacketInputBuffer::new();
        let layouts = layouts();
        assert_eq!(input.append(&[0x21], &layouts), Ok(vec![]));
        assert_eq!(
            input.append(&[0xAA, 0x77], &layouts),
            Err(PacketFrameError::UnknownPacketId { packet_id: 0x77 })
        );
        assert_eq!(input.pending_len(), 1);
        assert_eq!(input.append(&[0xBB], &layouts), Ok(vec![vec![0x21, 0xBB]]));
    }

    #[test]
    fn invalid_variable_length_after_complete_prefix_rolls_back_pending_and_frames() {
        let mut input = PacketInputBuffer::new();
        let layouts = layouts();
        assert_eq!(input.append(&[0x21], &layouts), Ok(vec![]));
        assert_eq!(
            input.append(&[0xAA, 0xBD, 0, 2], &layouts),
            Err(PacketFrameError::InvalidLength {
                packet_id: 0xBD,
                length: 2,
            })
        );
        assert_eq!(input.pending_len(), 1);
        assert_eq!(input.append(&[0xBB], &layouts), Ok(vec![vec![0x21, 0xBB]]));
    }
}

#[cfg(test)]
mod packet_frame_tests {
    use super::{decode_packet_frame, PacketFrameError, PacketLayout, PacketLayoutTable};

    #[test]
    fn fixed_frame_borrows_input_and_preserves_remainder() {
        let mut layouts = PacketLayoutTable::new();
        layouts
            .register(0x21, PacketLayout::Fixed { length: 2 })
            .unwrap();
        assert_eq!(layouts.get(0x21), Some(PacketLayout::Fixed { length: 2 }));
        let input = [0x21, 0xAA, 0xFF];
        let decoded = decode_packet_frame(&input, &layouts).unwrap().unwrap();
        assert_eq!(decoded.frame, &input[..2]);
        assert_eq!(decoded.frame.as_ptr(), input.as_ptr());
        assert_eq!(decoded.remaining, &input[2..]);
    }

    #[test]
    fn variable_frame_reads_big_endian_length_and_preserves_remainder() {
        let mut layouts = PacketLayoutTable::new();
        layouts.register(0xBD, PacketLayout::Variable).unwrap();
        let input = [0xBD, 0x00, 0x05, 0xAA, 0xBB, 0xFF];
        let decoded = decode_packet_frame(&input, &layouts).unwrap().unwrap();
        assert_eq!(decoded.frame, &input[..5]);
        assert_eq!(decoded.remaining, &input[5..]);
    }

    #[test]
    fn incomplete_header_and_body_return_none() {
        let mut layouts = PacketLayoutTable::new();
        layouts
            .register(0x21, PacketLayout::Fixed { length: 3 })
            .unwrap();
        layouts.register(0xBD, PacketLayout::Variable).unwrap();
        for input in [
            &[][..],
            &[0x21, 0xAA],
            &[0xBD],
            &[0xBD, 0x00],
            &[0xBD, 0x00, 0x05, 0xAA],
        ] {
            assert_eq!(decode_packet_frame(input, &layouts), Ok(None));
        }
    }

    #[test]
    fn unknown_packet_id_is_typed_error() {
        assert_eq!(
            decode_packet_frame(&[0x77], &PacketLayoutTable::new()),
            Err(PacketFrameError::UnknownPacketId { packet_id: 0x77 })
        );
    }

    #[test]
    fn variable_lengths_below_header_are_invalid() {
        let mut layouts = PacketLayoutTable::new();
        layouts.register(0xBD, PacketLayout::Variable).unwrap();
        for length in 0..3 {
            assert_eq!(
                decode_packet_frame(&[0xBD, 0, length as u8], &layouts),
                Err(PacketFrameError::InvalidLength {
                    packet_id: 0xBD,
                    length
                })
            );
        }
    }

    #[test]
    fn zero_fixed_layout_is_rejected_without_registration() {
        let mut layouts = PacketLayoutTable::new();
        assert_eq!(
            layouts.register(0x21, PacketLayout::Fixed { length: 0 }),
            Err(PacketFrameError::InvalidLayout { packet_id: 0x21 })
        );
        assert_eq!(layouts.get(0x21), None);
    }

    #[test]
    fn duplicate_packet_id_is_rejected_without_replacement() {
        let mut layouts = PacketLayoutTable::new();
        layouts
            .register(0x21, PacketLayout::Fixed { length: 2 })
            .unwrap();
        assert_eq!(
            layouts.register(0x21, PacketLayout::Variable),
            Err(PacketFrameError::DuplicatePacketId { packet_id: 0x21 })
        );
        assert_eq!(layouts.get(0x21), Some(PacketLayout::Fixed { length: 2 }));
    }
}

#[cfg(test)]
mod renaissance_5083_login_layout_tests {
    use super::{decode_packet_frame, renaissance_5083_login_layouts, PacketFrameError};

    #[test]
    fn login_profile_decodes_exact_fixed_frames() {
        let layouts = renaissance_5083_login_layouts().unwrap();
        for (packet_id, length) in [(0x80, 62), (0xA0, 3), (0x91, 65)] {
            let mut input = vec![0; length + 1];
            input[0] = packet_id;
            input[length] = 0xFF;

            let decoded = decode_packet_frame(&input, &layouts).unwrap().unwrap();
            assert_eq!(decoded.frame, &input[..length]);
            assert_eq!(decoded.remaining, &[0xFF]);
            assert_eq!(
                decode_packet_frame(&input[..length - 1], &layouts),
                Ok(None)
            );
        }
    }

    #[test]
    fn login_profile_rejects_out_of_scope_ids() {
        let layouts = renaissance_5083_login_layouts().unwrap();
        for packet_id in [0x5D, 0xEF, 0x02, 0x22, 0x73, 0xBD] {
            assert_eq!(
                decode_packet_frame(&[packet_id], &layouts),
                Err(PacketFrameError::UnknownPacketId { packet_id })
            );
        }
    }
}

#[cfg(test)]
mod packet_frames_tests {
    use super::{decode_packet_frames, PacketFrameError, PacketLayout, PacketLayoutTable};

    fn layouts() -> PacketLayoutTable {
        let mut layouts = PacketLayoutTable::new();
        layouts
            .register(0x21, PacketLayout::Fixed { length: 2 })
            .unwrap();
        layouts.register(0xBD, PacketLayout::Variable).unwrap();
        layouts
    }

    #[test]
    fn empty_input_has_no_frames_or_remainder() {
        let input = [];
        let decoded = decode_packet_frames(&input, &layouts()).unwrap();
        assert!(decoded.frames.is_empty());
        assert_eq!(decoded.remaining, &input);
    }

    #[test]
    fn consecutive_fixed_frames_are_borrowed_in_order() {
        let input = [0x21, 0xAA, 0x21, 0xBB];
        let decoded = decode_packet_frames(&input, &layouts()).unwrap();
        assert_eq!(decoded.frames, vec![&input[..2], &input[2..]]);
        assert_eq!(decoded.frames[1].as_ptr(), input[2..].as_ptr());
        assert!(decoded.remaining.is_empty());
    }

    #[test]
    fn variable_frames_are_decoded_in_order() {
        let input = [0xBD, 0, 3, 0xBD, 0, 5, 0xAA, 0xBB];
        let decoded = decode_packet_frames(&input, &layouts()).unwrap();
        assert_eq!(decoded.frames, vec![&input[..3], &input[3..]]);
        assert!(decoded.remaining.is_empty());
    }

    #[test]
    fn incomplete_trailing_header_or_body_is_untouched() {
        for suffix in [
            &[0xBD][..],
            &[0xBD, 0][..],
            &[0xBD, 0, 5, 0xAA][..],
            &[0x21][..],
        ] {
            let mut input = vec![0x21, 0xAA];
            input.extend_from_slice(suffix);
            let decoded = decode_packet_frames(&input, &layouts()).unwrap();
            assert_eq!(decoded.frames, vec![&input[..2]]);
            assert_eq!(decoded.remaining, &input[2..]);
            assert_eq!(decoded.remaining.as_ptr(), input[2..].as_ptr());
        }
    }

    #[test]
    fn error_after_complete_prefix_propagates_unchanged() {
        let input = [0x21, 0xAA, 0x77];
        assert_eq!(
            decode_packet_frames(&input, &layouts()),
            Err(PacketFrameError::UnknownPacketId { packet_id: 0x77 })
        );
        let input = [0x21, 0xAA, 0xBD, 0, 2];
        assert_eq!(
            decode_packet_frames(&input, &layouts()),
            Err(PacketFrameError::InvalidLength {
                packet_id: 0xBD,
                length: 2,
            })
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketLayout {
    Fixed { length: usize },
    Variable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketFrameError {
    UnknownPacketId { packet_id: u8 },
    InvalidLayout { packet_id: u8 },
    DuplicatePacketId { packet_id: u8 },
    InvalidLength { packet_id: u8, length: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketLayoutTable {
    layouts: [Option<PacketLayout>; 256],
}

impl Default for PacketLayoutTable {
    fn default() -> Self {
        Self {
            layouts: [None; 256],
        }
    }
}

impl PacketLayoutTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        packet_id: u8,
        layout: PacketLayout,
    ) -> Result<(), PacketFrameError> {
        if matches!(layout, PacketLayout::Fixed { length: 0 }) {
            return Err(PacketFrameError::InvalidLayout { packet_id });
        }
        if self.get(packet_id).is_some() {
            return Err(PacketFrameError::DuplicatePacketId { packet_id });
        }
        self.layouts[packet_id as usize] = Some(layout);
        Ok(())
    }

    pub fn get(&self, packet_id: u8) -> Option<PacketLayout> {
        self.layouts[packet_id as usize]
    }
}

/// Fixed client login packets registered by ServUO for the Renaissance 5.0.8.3 path.
/// The four-byte connection seed is decoded separately, before packet framing.
pub fn renaissance_5083_login_layouts() -> Result<PacketLayoutTable, PacketFrameError> {
    let mut layouts = PacketLayoutTable::new();
    for (packet_id, length) in [(0x80, 62), (0xA0, 3), (0x91, 65)] {
        layouts.register(packet_id, PacketLayout::Fixed { length })?;
    }
    Ok(layouts)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketFrame<'a> {
    pub frame: &'a [u8],
    pub remaining: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketFrames<'a> {
    pub frames: Vec<&'a [u8]>,
    pub remaining: &'a [u8],
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PacketInputBuffer {
    pending: Vec<u8>,
}

impl PacketInputBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn append(
        &mut self,
        bytes: &[u8],
        layouts: &PacketLayoutTable,
    ) -> Result<Vec<Vec<u8>>, PacketFrameError> {
        let mut combined = Vec::with_capacity(self.pending.len() + bytes.len());
        combined.extend_from_slice(&self.pending);
        combined.extend_from_slice(bytes);

        let decoded = decode_packet_frames(&combined, layouts)?;
        let frames = decoded.frames.into_iter().map(<[u8]>::to_vec).collect();
        self.pending = decoded.remaining.to_vec();
        Ok(frames)
    }
}

pub fn decode_packet_frame<'a>(
    bytes: &'a [u8],
    layouts: &PacketLayoutTable,
) -> Result<Option<PacketFrame<'a>>, PacketFrameError> {
    let Some(&packet_id) = bytes.first() else {
        return Ok(None);
    };
    let layout = layouts
        .get(packet_id)
        .ok_or(PacketFrameError::UnknownPacketId { packet_id })?;
    let frame_length = match layout {
        PacketLayout::Fixed { length } => length,
        PacketLayout::Variable => {
            if bytes.len() < 3 {
                return Ok(None);
            }
            let length = u16::from_be_bytes([bytes[1], bytes[2]]) as usize;
            if length < 3 {
                return Err(PacketFrameError::InvalidLength { packet_id, length });
            }
            length
        }
    };
    if bytes.len() < frame_length {
        return Ok(None);
    }
    let (frame, remaining) = bytes.split_at(frame_length);
    Ok(Some(PacketFrame { frame, remaining }))
}

pub fn decode_packet_frames<'a>(
    bytes: &'a [u8],
    layouts: &PacketLayoutTable,
) -> Result<PacketFrames<'a>, PacketFrameError> {
    let mut frames = Vec::new();
    let mut remaining = bytes;
    while let Some(decoded) = decode_packet_frame(remaining, layouts)? {
        frames.push(decoded.frame);
        remaining = decoded.remaining;
    }
    Ok(PacketFrames { frames, remaining })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedDecodeError {
    ZeroSeed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenaissanceSeed<'a> {
    pub seed: u32,
    pub remaining: &'a [u8],
}

pub fn decode_renaissance_5083_seed(
    bytes: &[u8],
) -> Result<Option<RenaissanceSeed<'_>>, SeedDecodeError> {
    if bytes.len() < 4 {
        return Ok(None);
    }

    let seed = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if seed == 0 {
        return Err(SeedDecodeError::ZeroSeed);
    }

    Ok(Some(RenaissanceSeed {
        seed,
        remaining: &bytes[4..],
    }))
}

const RENAISSANCE_ACCOUNT_LOGIN_PACKET_ID: u8 = 0x80;
const ACCOUNT_LOGIN_FIELD_LENGTH: usize = 30;
const ACCOUNT_LOGIN_FRAME_LENGTH: usize = 1 + 2 * ACCOUNT_LOGIN_FIELD_LENGTH + 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountLoginDecodeError {
    WrongPacketId { packet_id: u8 },
    Truncated { length: usize },
    InvalidLength { length: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenaissanceAccountLogin<'a> {
    pub username: &'a [u8],
    pub password: &'a [u8],
}

pub fn decode_renaissance_account_login(
    bytes: &[u8],
) -> Result<RenaissanceAccountLogin<'_>, AccountLoginDecodeError> {
    let Some(&packet_id) = bytes.first() else {
        return Err(AccountLoginDecodeError::Truncated { length: 0 });
    };
    if packet_id != RENAISSANCE_ACCOUNT_LOGIN_PACKET_ID {
        return Err(AccountLoginDecodeError::WrongPacketId { packet_id });
    }
    if bytes.len() < ACCOUNT_LOGIN_FRAME_LENGTH {
        return Err(AccountLoginDecodeError::Truncated {
            length: bytes.len(),
        });
    }
    if bytes.len() > ACCOUNT_LOGIN_FRAME_LENGTH {
        return Err(AccountLoginDecodeError::InvalidLength {
            length: bytes.len(),
        });
    }

    let username_field = &bytes[1..1 + ACCOUNT_LOGIN_FIELD_LENGTH];
    let password_start = 1 + ACCOUNT_LOGIN_FIELD_LENGTH;
    let password_field = &bytes[password_start..password_start + ACCOUNT_LOGIN_FIELD_LENGTH];

    Ok(RenaissanceAccountLogin {
        username: bytes_before_nul(username_field),
        password: bytes_before_nul(password_field),
    })
}

fn bytes_before_nul(field: &[u8]) -> &[u8] {
    let length = field
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(field.len());
    &field[..length]
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PacketWriter {
    bytes: Vec<u8>,
}

impl PacketWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn write_i8(&mut self, value: i8) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

const RENAISSANCE_GAME_LOGIN_PACKET_ID: u8 = 0x91;
const GAME_LOGIN_AUTH_ID_LENGTH: usize = 4;
const GAME_LOGIN_FIELD_LENGTH: usize = 30;
const GAME_LOGIN_FRAME_LENGTH: usize = 1 + GAME_LOGIN_AUTH_ID_LENGTH + 2 * GAME_LOGIN_FIELD_LENGTH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameLoginDecodeError {
    WrongPacketId { packet_id: u8 },
    Truncated { length: usize },
    InvalidLength { length: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenaissanceGameLogin<'a> {
    pub auth_id: u32,
    pub username: &'a [u8],
    pub password: &'a [u8],
}

pub fn decode_renaissance_game_login(
    bytes: &[u8],
) -> Result<RenaissanceGameLogin<'_>, GameLoginDecodeError> {
    let Some(&packet_id) = bytes.first() else {
        return Err(GameLoginDecodeError::Truncated { length: 0 });
    };
    if packet_id != RENAISSANCE_GAME_LOGIN_PACKET_ID {
        return Err(GameLoginDecodeError::WrongPacketId { packet_id });
    }
    if bytes.len() < GAME_LOGIN_FRAME_LENGTH {
        return Err(GameLoginDecodeError::Truncated {
            length: bytes.len(),
        });
    }
    if bytes.len() > GAME_LOGIN_FRAME_LENGTH {
        return Err(GameLoginDecodeError::InvalidLength {
            length: bytes.len(),
        });
    }

    let username_start = 1 + GAME_LOGIN_AUTH_ID_LENGTH;
    let password_start = username_start + GAME_LOGIN_FIELD_LENGTH;
    let username_field = &bytes[username_start..password_start];
    let password_field = &bytes[password_start..GAME_LOGIN_FRAME_LENGTH];

    Ok(RenaissanceGameLogin {
        auth_id: u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]),
        username: bytes_before_nul(username_field),
        password: bytes_before_nul(password_field),
    })
}

const RENAISSANCE_ACCOUNT_LOGIN_ACK_PACKET_ID: u8 = 0xA8;
const ACCOUNT_LOGIN_ACK_UNKNOWN: u8 = 0x5D;
const ACCOUNT_LOGIN_ACK_HEADER_LENGTH: usize = 6;
const SERVER_LIST_ENTRY_LENGTH: usize = 40;
const SERVER_LIST_NAME_LENGTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountLoginAckEncodeError {
    CountOverflow { count: usize },
    LengthOverflow { length: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenaissanceServerListEntry<'a> {
    pub name: &'a [u8],
    pub full_percent: u8,
    pub timezone: i8,
    pub address: u32,
}

pub fn encode_renaissance_account_login_ack(
    entries: &[RenaissanceServerListEntry<'_>],
) -> Result<Vec<u8>, AccountLoginAckEncodeError> {
    let count =
        u16::try_from(entries.len()).map_err(|_| AccountLoginAckEncodeError::CountOverflow {
            count: entries.len(),
        })?;
    let length = entries
        .len()
        .checked_mul(SERVER_LIST_ENTRY_LENGTH)
        .and_then(|entries_length| entries_length.checked_add(ACCOUNT_LOGIN_ACK_HEADER_LENGTH))
        .ok_or(AccountLoginAckEncodeError::LengthOverflow { length: usize::MAX })?;
    let encoded_length =
        u16::try_from(length).map_err(|_| AccountLoginAckEncodeError::LengthOverflow { length })?;

    let mut writer = PacketWriter::new();
    writer.write_u8(RENAISSANCE_ACCOUNT_LOGIN_ACK_PACKET_ID);
    writer.write_u16(encoded_length);
    writer.write_u8(ACCOUNT_LOGIN_ACK_UNKNOWN);
    writer.write_u16(count);

    let name_padding = [0; SERVER_LIST_NAME_LENGTH];
    for (index, entry) in (0..count).zip(entries) {
        writer.write_u16(index);

        let name_length = entry.name.len().min(SERVER_LIST_NAME_LENGTH);
        writer.write_bytes(&entry.name[..name_length]);
        writer.write_bytes(&name_padding[..SERVER_LIST_NAME_LENGTH - name_length]);

        writer.write_u8(entry.full_percent);
        writer.write_i8(entry.timezone);
        writer.write_u32(entry.address);
    }

    Ok(writer.into_inner())
}

const RENAISSANCE_SERVER_SELECTION_PACKET_ID: u8 = 0xA0;
const SERVER_SELECTION_FRAME_LENGTH: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerSelectionDecodeError {
    WrongPacketId { packet_id: u8 },
    Truncated { length: usize },
    InvalidLength { length: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenaissanceServerSelection {
    pub index: i16,
}

pub fn decode_renaissance_server_selection(
    bytes: &[u8],
) -> Result<RenaissanceServerSelection, ServerSelectionDecodeError> {
    let Some(&packet_id) = bytes.first() else {
        return Err(ServerSelectionDecodeError::Truncated { length: 0 });
    };
    if packet_id != RENAISSANCE_SERVER_SELECTION_PACKET_ID {
        return Err(ServerSelectionDecodeError::WrongPacketId { packet_id });
    }
    if bytes.len() < SERVER_SELECTION_FRAME_LENGTH {
        return Err(ServerSelectionDecodeError::Truncated {
            length: bytes.len(),
        });
    }
    if bytes.len() > SERVER_SELECTION_FRAME_LENGTH {
        return Err(ServerSelectionDecodeError::InvalidLength {
            length: bytes.len(),
        });
    }

    Ok(RenaissanceServerSelection {
        index: i16::from_be_bytes([bytes[1], bytes[2]]),
    })
}

pub fn encode_renaissance_play_server_ack(address: u32, port: u16, auth_id: u32) -> Vec<u8> {
    let mut writer = PacketWriter::new();
    writer.write_u8(0x8C);
    writer.write_bytes(&address.to_le_bytes());
    writer.write_u16(port);
    writer.write_u32(auth_id);
    writer.into_inner()
}

/// Encodes the fixed LoginConfirm frame from already-resolved world-entry fields.
pub fn encode_renaissance_login_confirm(
    serial: u32,
    body: u16,
    x: u16,
    y: u16,
    z: i16,
    direction: u8,
    map_width: u16,
    map_height: u16,
) -> Vec<u8> {
    let mut writer = PacketWriter::new();
    writer.write_u8(0x1B);
    writer.write_u32(serial);
    writer.write_u32(0);
    writer.write_u16(body);
    writer.write_u16(x);
    writer.write_u16(y);
    writer.write_bytes(&z.to_be_bytes());
    writer.write_u8(direction);
    writer.write_u8(0);
    writer.write_u32(u32::MAX);
    writer.write_u16(0);
    writer.write_u16(0);
    writer.write_u16(map_width);
    writer.write_u16(map_height);
    writer.write_bytes(&[0; 6]);
    writer.into_inner()
}

/// Classic 0xB9 uses the configured 16-bit feature mask, not an expansion default.
pub fn encode_renaissance_supported_features(feature_flags: u16) -> [u8; 3] {
    let [high, low] = feature_flags.to_be_bytes();
    [0xB9, high, low]
}

/// The 0xBF/0x0008 MapChange packet for an explicit facet ID.
pub fn encode_renaissance_map_change(map_id: u8) -> [u8; 6] {
    [0xBF, 0, 6, 0, 8, map_id]
}

/// The 0xBF/0x0018 MapPatches packet. Pairs are static then land blocks,
/// ordered Felucca, Trammel, Ilshenar, Malas as in ServUO.
pub fn encode_renaissance_map_patches(patch_counts: [(i32, i32); 4]) -> [u8; 41] {
    let mut packet = [0; 41];
    packet[..9].copy_from_slice(&[0xBF, 0, 0x29, 0, 0x18, 0, 0, 0, 4]);
    for (index, (static_blocks, land_blocks)) in patch_counts.into_iter().enumerate() {
        let offset = 9 + index * 8;
        packet[offset..offset + 4].copy_from_slice(&static_blocks.to_be_bytes());
        packet[offset + 4..offset + 8].copy_from_slice(&land_blocks.to_be_bytes());
    }
    packet
}

// ServUO Compression.cs: pairs are MSB-first bit length and code; index 256 is EOF.
#[rustfmt::skip]
const LEGACY_HUFFMAN: [(u8, u16); 257] = [
    (2, 0x000), (5, 0x01F), (6, 0x022), (7, 0x034),
    (7, 0x075), (6, 0x028), (6, 0x03B), (7, 0x032),
    (8, 0x0E0), (8, 0x062), (7, 0x056), (8, 0x079),
    (9, 0x19D), (8, 0x097), (6, 0x02A), (7, 0x057),
    (8, 0x071), (8, 0x05B), (9, 0x1CC), (8, 0x0A7),
    (7, 0x025), (7, 0x04F), (8, 0x066), (8, 0x07D),
    (9, 0x191), (9, 0x1CE), (7, 0x03F), (9, 0x090),
    (8, 0x059), (8, 0x07B), (8, 0x091), (8, 0x0C6),
    (6, 0x02D), (9, 0x186), (8, 0x06F), (9, 0x093),
    (10, 0x1CC), (8, 0x05A), (10, 0x1AE), (10, 0x1C0),
    (9, 0x148), (9, 0x14A), (9, 0x082), (10, 0x19F),
    (9, 0x171), (9, 0x120), (9, 0x0E7), (10, 0x1F3),
    (9, 0x14B), (9, 0x100), (9, 0x190), (6, 0x013),
    (9, 0x161), (9, 0x125), (9, 0x133), (9, 0x195),
    (9, 0x173), (9, 0x1CA), (9, 0x086), (9, 0x1E9),
    (9, 0x0DB), (9, 0x1EC), (9, 0x08B), (9, 0x085),
    (5, 0x00A), (8, 0x096), (8, 0x09C), (9, 0x1C3),
    (9, 0x19C), (9, 0x08F), (9, 0x18F), (9, 0x091),
    (9, 0x087), (9, 0x0C6), (9, 0x177), (9, 0x089),
    (9, 0x0D6), (9, 0x08C), (9, 0x1EE), (9, 0x1EB),
    (9, 0x084), (9, 0x164), (9, 0x175), (9, 0x1CD),
    (8, 0x05E), (9, 0x088), (9, 0x12B), (9, 0x172),
    (9, 0x10A), (9, 0x08D), (9, 0x13A), (9, 0x11C),
    (10, 0x1E1), (10, 0x1E0), (9, 0x187), (10, 0x1DC),
    (10, 0x1DF), (7, 0x074), (9, 0x19F), (8, 0x08D),
    (8, 0x0E4), (7, 0x079), (9, 0x0EA), (9, 0x0E1),
    (8, 0x040), (7, 0x041), (9, 0x10B), (9, 0x0B0),
    (8, 0x06A), (8, 0x0C1), (7, 0x071), (7, 0x078),
    (8, 0x0B1), (9, 0x14C), (7, 0x043), (8, 0x076),
    (7, 0x066), (7, 0x04D), (9, 0x08A), (6, 0x02F),
    (8, 0x0C9), (9, 0x0CE), (9, 0x149), (9, 0x160),
    (10, 0x1BA), (10, 0x19E), (10, 0x39F), (9, 0x0E5),
    (9, 0x194), (9, 0x184), (9, 0x126), (7, 0x030),
    (8, 0x06C), (9, 0x121), (9, 0x1E8), (10, 0x1C1),
    (10, 0x11D), (10, 0x163), (10, 0x385), (10, 0x3DB),
    (10, 0x17D), (10, 0x106), (10, 0x397), (10, 0x24E),
    (7, 0x02E), (8, 0x098), (10, 0x33C), (10, 0x32E),
    (10, 0x1E9), (9, 0x0BF), (10, 0x3DF), (10, 0x1DD),
    (10, 0x32D), (10, 0x2ED), (10, 0x30B), (10, 0x107),
    (10, 0x2E8), (10, 0x3DE), (10, 0x125), (10, 0x1E8),
    (9, 0x0E9), (10, 0x1CD), (10, 0x1B5), (9, 0x165),
    (10, 0x232), (10, 0x2E1), (11, 0x3AE), (11, 0x3C6),
    (11, 0x3E2), (10, 0x205), (10, 0x29A), (10, 0x248),
    (10, 0x2CD), (10, 0x23B), (11, 0x3C5), (10, 0x251),
    (10, 0x2E9), (10, 0x252), (9, 0x1EA), (11, 0x3A0),
    (11, 0x391), (10, 0x23C), (11, 0x392), (11, 0x3D5),
    (10, 0x233), (10, 0x2CC), (11, 0x390), (10, 0x1BB),
    (11, 0x3A1), (11, 0x3C4), (10, 0x211), (10, 0x203),
    (9, 0x12A), (10, 0x231), (11, 0x3E0), (10, 0x29B),
    (11, 0x3D7), (10, 0x202), (11, 0x3AD), (10, 0x213),
    (10, 0x253), (10, 0x32C), (10, 0x23D), (10, 0x23F),
    (10, 0x32F), (10, 0x11C), (10, 0x384), (10, 0x31C),
    (10, 0x17C), (10, 0x30A), (10, 0x2E0), (10, 0x276),
    (10, 0x250), (11, 0x3E3), (10, 0x396), (10, 0x18F),
    (10, 0x204), (10, 0x206), (10, 0x230), (10, 0x265),
    (10, 0x212), (10, 0x23E), (11, 0x3AC), (11, 0x393),
    (11, 0x3E1), (10, 0x1DE), (11, 0x3D6), (10, 0x31D),
    (11, 0x3E5), (11, 0x3E4), (10, 0x207), (11, 0x3C7),
    (10, 0x277), (11, 0x3D4), (8, 0x0C0), (10, 0x162),
    (10, 0x3DA), (10, 0x124), (10, 0x1B4), (10, 0x264),
    (10, 0x33D), (10, 0x1D1), (10, 0x1AF), (10, 0x39E),
    (10, 0x24F), (11, 0x373), (10, 0x249), (11, 0x372),
    (9, 0x167), (10, 0x210), (10, 0x23A), (10, 0x1B8),
    (11, 0x3AF), (10, 0x18E), (10, 0x2EC), (7, 0x062),
    (4, 0x00D),
];

const LEGACY_COMPRESSED_PACKET_LIMIT: usize = 0x10000;
const LEGACY_DEFINITE_OVERFLOW_INPUT: usize = (LEGACY_COMPRESSED_PACKET_LIMIT * 8 - 4) / 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionError {
    OutputOverflow,
}

/// Compresses one complete outbound packet; the terminal code closes this packet.
pub fn compress_legacy_packet(packet: &[u8]) -> Result<Vec<u8>, CompressionError> {
    if packet.len() > LEGACY_DEFINITE_OVERFLOW_INPUT {
        return Err(CompressionError::OutputOverflow);
    }
    let mut output = Vec::new();
    let mut pending = 0u8;
    let mut count = 0u8;
    for symbol in packet
        .iter()
        .map(|byte| usize::from(*byte))
        .chain(std::iter::once(256))
    {
        let (length, code) = LEGACY_HUFFMAN[symbol];
        for shift in (0..length).rev() {
            pending = (pending << 1) | ((code >> shift) as u8 & 1);
            count += 1;
            if count == 8 {
                if output.len() == LEGACY_COMPRESSED_PACKET_LIMIT {
                    return Err(CompressionError::OutputOverflow);
                }
                output.push(pending);
                pending = 0;
                count = 0;
            }
        }
    }
    if count != 0 {
        if output.len() == LEGACY_COMPRESSED_PACKET_LIMIT {
            return Err(CompressionError::OutputOverflow);
        }
        output.push(pending << (8 - count));
    }
    Ok(output)
}

const OLD_CHARACTER_LIST_FRAME_LENGTH: u16 = 309;
const OLD_CHARACTER_NAME_LENGTH: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OldCharacterListEncodeError {
    NameTooLong { length: usize },
}

/// Encodes the old five-slot list for one pre-seeded character in slot zero.
pub fn encode_renaissance_old_character_list(
    name: &[u8],
) -> Result<Vec<u8>, OldCharacterListEncodeError> {
    if name.len() > OLD_CHARACTER_NAME_LENGTH {
        return Err(OldCharacterListEncodeError::NameTooLong { length: name.len() });
    }

    let mut writer = PacketWriter::new();
    writer.write_u8(0xA9);
    writer.write_u16(OLD_CHARACTER_LIST_FRAME_LENGTH);
    writer.write_u8(5);
    writer.write_bytes(name);
    writer.write_bytes(&[0; OLD_CHARACTER_NAME_LENGTH][..OLD_CHARACTER_NAME_LENGTH - name.len()]);
    writer.write_bytes(&[0; 30]); // Slot-zero password field.
    writer.write_bytes(&[0; 4 * 60]); // Four empty slots.
    writer.write_u8(0); // No cities.
    writer.write_u32(0x0000_0014); // SlotLimit | OneCharacterSlot.
    Ok(writer.into_inner())
}

const PLAY_CHARACTER_FRAME_LENGTH: usize = 73;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayCharacterSlotDecodeError {
    WrongPacketId { packet_id: u8 },
    Truncated { length: usize },
    InvalidLength { length: usize },
}

/// Reads the raw signed slot only; authorization belongs to the session layer.
pub fn decode_renaissance_play_character_slot(
    bytes: &[u8],
) -> Result<i32, PlayCharacterSlotDecodeError> {
    let Some(&packet_id) = bytes.first() else {
        return Err(PlayCharacterSlotDecodeError::Truncated { length: 0 });
    };
    if packet_id != 0x5D {
        return Err(PlayCharacterSlotDecodeError::WrongPacketId { packet_id });
    }
    if bytes.len() < PLAY_CHARACTER_FRAME_LENGTH {
        return Err(PlayCharacterSlotDecodeError::Truncated {
            length: bytes.len(),
        });
    }
    if bytes.len() > PLAY_CHARACTER_FRAME_LENGTH {
        return Err(PlayCharacterSlotDecodeError::InvalidLength {
            length: bytes.len(),
        });
    }

    Ok(i32::from_be_bytes(bytes[65..69].try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::{
        decode_renaissance_game_login, decode_renaissance_server_selection,
        encode_renaissance_account_login_ack, encode_renaissance_play_server_ack,
        AccountLoginAckEncodeError, GameLoginDecodeError, RenaissanceServerListEntry,
        RenaissanceServerSelection, ServerSelectionDecodeError,
    };

    #[test]
    fn account_login_ack_encoder_encodes_empty_server_list_header() {
        assert_eq!(
            encode_renaissance_account_login_ack(&[]),
            Ok(vec![0xA8, 0x00, 0x06, 0x5D, 0x00, 0x00])
        );
    }

    #[test]
    fn account_login_ack_encoder_encodes_single_server_entry() {
        let entry = RenaissanceServerListEntry {
            name: b"Renaissance",
            full_percent: 42,
            timezone: -5,
            address: 0x1234_5678,
        };

        let encoded = encode_renaissance_account_login_ack(&[entry]).unwrap();

        assert_eq!(&encoded[..6], &[0xA8, 0x00, 0x2E, 0x5D, 0x00, 0x01]);
        assert_eq!(&encoded[6..8], &[0x00, 0x00]);
        assert_eq!(
            &encoded[8..40],
            b"Renaissance\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"
        );
        assert_eq!(&encoded[40..42], &[42, 0xFB]);
        assert_eq!(&encoded[42..46], &[0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn account_login_ack_encoder_preserves_multiple_entry_order_and_indexes() {
        let entries = [
            RenaissanceServerListEntry {
                name: b"First",
                full_percent: 1,
                timezone: 2,
                address: 0x0102_0304,
            },
            RenaissanceServerListEntry {
                name: b"Second",
                full_percent: 3,
                timezone: -4,
                address: 0xA0B0_C0D0,
            },
        ];

        let encoded = encode_renaissance_account_login_ack(&entries).unwrap();

        assert_eq!(&encoded[..6], &[0xA8, 0x00, 0x56, 0x5D, 0x00, 0x02]);
        assert_eq!(&encoded[6..8], &[0x00, 0x00]);
        assert_eq!(&encoded[46..48], &[0x00, 0x01]);
        assert_eq!(&encoded[48..54], b"Second");
        assert_eq!(&encoded[80..82], &[3, 0xFC]);
        assert_eq!(&encoded[82..86], &[0xA0, 0xB0, 0xC0, 0xD0]);
    }

    #[test]
    fn account_login_ack_encoder_truncates_and_pads_raw_names() {
        let long_name = [0xE9; 40];
        let entries = [
            RenaissanceServerListEntry {
                name: &long_name,
                full_percent: 0,
                timezone: 0,
                address: 0,
            },
            RenaissanceServerListEntry {
                name: &[0xFF, 0x80],
                full_percent: 0,
                timezone: 0,
                address: 0,
            },
        ];

        let encoded = encode_renaissance_account_login_ack(&entries).unwrap();

        assert_eq!(&encoded[8..40], &[0xE9; 32]);
        assert_eq!(&encoded[48..50], &[0xFF, 0x80]);
        assert_eq!(&encoded[50..80], &[0; 30]);
    }

    #[test]
    fn account_login_ack_encoder_rejects_unrepresentable_lengths_and_counts() {
        let entry = RenaissanceServerListEntry {
            name: b"",
            full_percent: 0,
            timezone: 0,
            address: 0,
        };
        let too_long = vec![entry; (u16::MAX as usize - 6) / 40 + 1];

        assert_eq!(
            encode_renaissance_account_login_ack(&too_long),
            Err(AccountLoginAckEncodeError::LengthOverflow {
                length: 6 + 40 * too_long.len(),
            })
        );

        let too_many = vec![entry; u16::MAX as usize + 1];
        assert_eq!(
            encode_renaissance_account_login_ack(&too_many),
            Err(AccountLoginAckEncodeError::CountOverflow {
                count: too_many.len(),
            })
        );
    }

    #[test]
    fn renaissance_seed_decoder_returns_incomplete_without_consuming_input() {
        let input = [0x01, 0x02, 0x03];

        assert_eq!(super::decode_renaissance_5083_seed(&input), Ok(None));
        assert_eq!(input, [0x01, 0x02, 0x03]);
    }

    #[test]
    fn renaissance_seed_decoder_reads_nonzero_big_endian_seed() {
        let input = [0x12, 0x34, 0x56, 0x78];

        let decoded = super::decode_renaissance_5083_seed(&input)
            .unwrap()
            .unwrap();

        assert_eq!(decoded.seed, 0x1234_5678);
        assert!(decoded.remaining.is_empty());
    }

    #[test]
    fn renaissance_seed_decoder_rejects_zero_seed() {
        let input = [0x00, 0x00, 0x00, 0x00];

        assert_eq!(
            super::decode_renaissance_5083_seed(&input),
            Err(super::SeedDecodeError::ZeroSeed)
        );
    }

    #[test]
    fn renaissance_seed_decoder_preserves_trailing_bytes() {
        let input = [0x00, 0x00, 0x00, 0x01, 0xEF, 0xAA, 0x55];

        let decoded = super::decode_renaissance_5083_seed(&input)
            .unwrap()
            .unwrap();

        assert_eq!(decoded.seed, 1);
        assert_eq!(decoded.remaining, &[0xEF, 0xAA, 0x55]);
    }
    use super::{decode_renaissance_account_login, AccountLoginDecodeError};

    #[test]
    fn account_login_decoder_reads_raw_fixed_width_fields() {
        let mut frame = vec![0; 62];
        frame[0] = 0x80;
        frame[1..31].copy_from_slice(&[0xE9; 30]);
        frame[31..61].copy_from_slice(&[0xFE; 30]);

        let decoded = decode_renaissance_account_login(&frame).unwrap();

        assert_eq!(decoded.username, &[0xE9; 30]);
        assert_eq!(decoded.password, &[0xFE; 30]);
    }

    #[test]
    fn account_login_decoder_stops_fields_at_nul_but_keeps_fixed_offsets() {
        let mut frame = vec![0xAA; 62];
        frame[0] = 0x80;
        frame[1] = b'u';
        frame[2] = b's';
        frame[3] = 0;
        frame[31] = b'p';
        frame[32] = 0;

        let decoded = decode_renaissance_account_login(&frame).unwrap();

        assert_eq!(decoded.username, b"us");
        assert_eq!(decoded.password, b"p");
    }

    #[test]
    fn account_login_decoder_reports_truncated_frames() {
        assert_eq!(
            decode_renaissance_account_login(&[]),
            Err(AccountLoginDecodeError::Truncated { length: 0 })
        );
        assert_eq!(
            decode_renaissance_account_login(&[0x80; 61]),
            Err(AccountLoginDecodeError::Truncated { length: 61 })
        );
    }

    #[test]
    fn account_login_decoder_rejects_wrong_packet_id_and_length() {
        assert_eq!(
            decode_renaissance_account_login(&[0x81; 62]),
            Err(AccountLoginDecodeError::WrongPacketId { packet_id: 0x81 })
        );

        let mut oversized = vec![0; 63];
        oversized[0] = 0x80;
        assert_eq!(
            decode_renaissance_account_login(&oversized),
            Err(AccountLoginDecodeError::InvalidLength { length: 63 })
        );
    }

    #[test]
    fn account_login_decoder_accepts_any_final_frame_byte() {
        let mut frame = vec![0; 62];
        frame[0] = 0x80;

        frame[61] = 0;
        assert!(decode_renaissance_account_login(&frame).is_ok());
        frame[61] = 0xFF;
        assert!(decode_renaissance_account_login(&frame).is_ok());
    }

    #[test]
    fn server_selection_decoder_preserves_signed_big_endian_indexes() {
        assert_eq!(
            decode_renaissance_server_selection(&[0xA0, 0x12, 0x34]),
            Ok(RenaissanceServerSelection { index: 0x1234 })
        );
        assert_eq!(
            decode_renaissance_server_selection(&[0xA0, 0xFF, 0xFE]),
            Ok(RenaissanceServerSelection { index: -2 })
        );
    }

    #[test]
    fn server_selection_decoder_rejects_wrong_packet_id() {
        assert_eq!(
            decode_renaissance_server_selection(&[0xA1, 0x00, 0x01]),
            Err(ServerSelectionDecodeError::WrongPacketId { packet_id: 0xA1 })
        );
    }

    #[test]
    fn server_selection_decoder_reports_truncated_and_extra_bytes() {
        assert_eq!(
            decode_renaissance_server_selection(&[]),
            Err(ServerSelectionDecodeError::Truncated { length: 0 })
        );
        assert_eq!(
            decode_renaissance_server_selection(&[0xA0, 0x01]),
            Err(ServerSelectionDecodeError::Truncated { length: 2 })
        );
        assert_eq!(
            decode_renaissance_server_selection(&[0xA0, 0x00, 0x01, 0xFF]),
            Err(ServerSelectionDecodeError::InvalidLength { length: 4 })
        );
    }

    #[test]
    fn play_server_ack_encoder_writes_exact_eleven_byte_frame() {
        let frame = encode_renaissance_play_server_ack(0x1122_3344, 0x5566, 0x7788_99AA);

        assert_eq!(frame.len(), 11);
        assert_eq!(
            frame,
            [0x8C, 0x44, 0x33, 0x22, 0x11, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA]
        );
    }

    #[test]
    fn game_login_decoder_reads_auth_id_and_raw_fields_from_exact_offsets() {
        let mut frame = [0xCC; 65];
        frame[0] = 0x91;
        frame[1..5].copy_from_slice(&[0x80, 0x12, 0x34, 0x56]);
        frame[5..35].fill(0x81);
        frame[35..65].fill(0xFE);

        let decoded = decode_renaissance_game_login(&frame).unwrap();

        assert_eq!(decoded.auth_id, 0x8012_3456);
        assert_eq!(decoded.username, &[0x81; 30]);
        assert_eq!(decoded.password, &[0xFE; 30]);
        assert_eq!(decoded.username.as_ptr(), frame[5..].as_ptr());
        assert_eq!(decoded.password.as_ptr(), frame[35..].as_ptr());
    }

    #[test]
    fn game_login_decoder_preserves_zero_auth_id_and_truncates_each_field_at_nul() {
        let mut frame = [0; 65];
        frame[0] = 0x91;
        frame[5..35].fill(0x66);
        frame[5..8].copy_from_slice(&[0x55, 0xFF, 0]);
        frame[35..65].fill(0xFE);
        frame[35..37].copy_from_slice(&[0x80, 0]);

        let decoded = decode_renaissance_game_login(&frame).unwrap();

        assert_eq!(decoded.auth_id, 0);
        assert_eq!(decoded.username, &[0x55, 0xFF]);
        assert_eq!(decoded.password, &[0x80]);
    }

    #[test]
    fn game_login_decoder_accepts_empty_fields_and_reports_every_short_length() {
        let mut frame = [0; 65];
        frame[0] = 0x91;
        let decoded = decode_renaissance_game_login(&frame).unwrap();
        assert_eq!(decoded.username, b"");
        assert_eq!(decoded.password, b"");

        for length in 0..65 {
            let error = decode_renaissance_game_login(&frame[..length]).unwrap_err();
            assert_eq!(error, GameLoginDecodeError::Truncated { length });
        }
    }

    #[test]
    fn game_login_decoder_rejects_wrong_packet_id_and_oversized_frame() {
        let mut frame = [0; 65];
        frame[0] = 0x90;
        assert_eq!(
            decode_renaissance_game_login(&frame),
            Err(GameLoginDecodeError::WrongPacketId { packet_id: 0x90 })
        );

        frame[0] = 0x91;
        let mut oversized = [0; 66];
        oversized[..65].copy_from_slice(&frame);
        assert_eq!(
            decode_renaissance_game_login(&oversized),
            Err(GameLoginDecodeError::InvalidLength { length: 66 })
        );
    }
}

#[cfg(test)]
mod renaissance_old_character_entry_tests {
    use super::{
        decode_renaissance_play_character_slot, encode_renaissance_old_character_list,
        OldCharacterListEncodeError, PlayCharacterSlotDecodeError,
    };

    #[test]
    fn renaissance_old_character_entry_encodes_exact_five_slot_fixture() {
        let frame = encode_renaissance_old_character_list(&[0x80, b'A']).unwrap();
        let mut expected = vec![0xA9, 0x01, 0x35, 0x05];
        expected.extend_from_slice(&[0x80, b'A']);
        expected.extend_from_slice(&[0; 58]);
        expected.extend_from_slice(&[0; 240]);
        expected.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x14]);
        assert_eq!(frame, expected);
        assert_eq!(frame.len(), 309);
    }

    #[test]
    fn renaissance_old_character_entry_accepts_thirty_bytes_and_rejects_more() {
        let name = [0xFF; 30];
        let frame = encode_renaissance_old_character_list(&name).unwrap();
        assert_eq!(&frame[4..34], &name);
        assert_eq!(&frame[34..64], &[0; 30]);
        assert_eq!(
            encode_renaissance_old_character_list(&[0xFF; 31]),
            Err(OldCharacterListEncodeError::NameTooLong { length: 31 })
        );
    }

    #[test]
    fn renaissance_old_character_entry_preserves_signed_slot_at_fixed_offset() {
        let mut frame = [0xAA; 73];
        frame[0] = 0x5D;
        frame[65..69].copy_from_slice(&0i32.to_be_bytes());
        assert_eq!(decode_renaissance_play_character_slot(&frame), Ok(0));
        frame[65..69].copy_from_slice(&(-2i32).to_be_bytes());
        assert_eq!(decode_renaissance_play_character_slot(&frame), Ok(-2));
    }

    #[test]
    fn renaissance_old_character_entry_rejects_wrong_id_and_nonexact_lengths() {
        let mut frame = [0; 73];
        frame[0] = 0x5C;
        assert_eq!(
            decode_renaissance_play_character_slot(&frame),
            Err(PlayCharacterSlotDecodeError::WrongPacketId { packet_id: 0x5C })
        );
        frame[0] = 0x5D;
        assert_eq!(
            decode_renaissance_play_character_slot(&frame[..72]),
            Err(PlayCharacterSlotDecodeError::Truncated { length: 72 })
        );
        assert_eq!(
            decode_renaissance_play_character_slot(&[]),
            Err(PlayCharacterSlotDecodeError::Truncated { length: 0 })
        );
        assert_eq!(
            decode_renaissance_play_character_slot(&[&frame[..], &[0xFF]].concat()),
            Err(PlayCharacterSlotDecodeError::InvalidLength { length: 74 })
        );
    }
}

#[cfg(test)]
mod renaissance_login_confirm_encoder_tests {
    use super::encode_renaissance_login_confirm;

    #[test]
    fn login_confirm_encoder_writes_exact_thirty_seven_byte_frame_with_signed_z() {
        let frame = encode_renaissance_login_confirm(
            0x1234_5678,
            0x09AB,
            0x1357,
            0x2468,
            -2,
            0x87,
            0x1800,
            0x1000,
        );

        assert_eq!(frame.len(), 37);
        assert_eq!(
            frame,
            [
                0x1B, 0x12, 0x34, 0x56, 0x78, 0, 0, 0, 0, 0x09, 0xAB, 0x13, 0x57, 0x24, 0x68, 0xFF,
                0xFE, 0x87, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0x18, 0, 0x10, 0, 0, 0, 0, 0, 0,
                0,
            ]
        );
    }
}
