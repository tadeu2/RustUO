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
}
