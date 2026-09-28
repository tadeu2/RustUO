use rustuo_protocol::{encode_renaissance_map_change, encode_renaissance_map_patches};

#[test]
fn map_change_uses_exact_extended_packet_layout() {
    assert_eq!(encode_renaissance_map_change(1), [0xBF, 0, 6, 0, 8, 1]);
}

#[test]
fn map_patches_writes_static_then_land_counts_in_legacy_facet_order() {
    let packet = encode_renaissance_map_patches([(1, 2), (3, 4), (5, 6), (7, 8)]);
    let mut expected = vec![0xBF, 0, 0x29, 0, 0x18, 0, 0, 0, 4];
    for count in 1_i32..=8 {
        expected.extend_from_slice(&count.to_be_bytes());
    }
    assert_eq!(packet.as_slice(), expected);
    assert_eq!(packet.len(), 41);
}
