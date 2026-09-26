#![forbid(unsafe_code)]

//! Ultima Online protocol boundary.

use rustuo_core::Serial;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityReference {
    pub serial: Serial,
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

#[cfg(test)]
mod tests {
    use super::{
        decode_renaissance_server_selection, encode_renaissance_account_login_ack,
        encode_renaissance_play_server_ack, AccountLoginAckEncodeError, RenaissanceServerListEntry,
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
}
