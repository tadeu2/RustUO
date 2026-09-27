use rustuo_protocol::{compress_legacy_packet, encode_renaissance_supported_features};

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
    assert_eq!(compress_legacy_packet(&[]), [0xD0]);
    assert_eq!(compress_legacy_packet(&[0xB9, 0, 3]), [0xB3, 0x06, 0x9A]);
    assert_eq!(
        compress_legacy_packet(&[0xB9, 0x12, 0x34]),
        [0xB3, 0x39, 0x96, 0x1D]
    );
    assert_eq!(compress_legacy_packet(&[0xB9, 0, 3]), [0xB3, 0x06, 0x9A]);
}
