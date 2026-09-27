use rustuo_protocol::{
    compress_legacy_packet, encode_renaissance_supported_features, CompressionError,
};

#[test]
fn classic_features_preserve_explicit_mask_in_big_endian_frame() {
    assert_eq!(
        encode_renaissance_supported_features(0x1234),
        [0xB9, 0x12, 0x34]
    );
    assert_eq!(encode_renaissance_supported_features(0x0003), [0xB9, 0, 3]);
}

#[test]
fn legacy_huffman_uses_terminal_and_resets_for_each_packet() {
    assert_eq!(compress_legacy_packet(&[]).unwrap(), [0xD0]);
    assert_eq!(
        compress_legacy_packet(&[0xB9, 0, 3]).unwrap(),
        [0xB3, 0x06, 0x9A]
    );
    assert_eq!(
        compress_legacy_packet(&[0xB9, 0x12, 0x34]).unwrap(),
        [0xB3, 0x39, 0x96, 0x1D]
    );
    assert_eq!(
        compress_legacy_packet(&[0xB9, 0, 3]).unwrap(),
        [0xB3, 0x06, 0x9A]
    );
}

#[test]
fn legacy_huffman_rejects_output_over_64_kib_without_truncation() {
    assert_eq!(
        compress_legacy_packet(&vec![0xA6; 65_535]),
        Err(CompressionError::OutputOverflow)
    );
    assert_eq!(
        compress_legacy_packet(&vec![0xA6; 47_663]),
        Err(CompressionError::OutputOverflow)
    );
}

#[test]
fn legacy_huffman_accepts_exact_64_kib_output() {
    let compressed = compress_legacy_packet(&vec![0xA6; 47_662]).unwrap();
    assert_eq!(compressed.len(), 65_536);
}
