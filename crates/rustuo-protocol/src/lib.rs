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

#[cfg(test)]
mod tests {
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
}
