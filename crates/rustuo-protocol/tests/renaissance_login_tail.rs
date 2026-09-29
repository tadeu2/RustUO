use rustuo_protocol::{
    encode_renaissance_aos_mobile_status, encode_renaissance_current_time,
    encode_renaissance_global_light, encode_renaissance_login_complete,
    encode_renaissance_personal_light, encode_renaissance_season_change,
    encode_renaissance_war_mode, RenaissanceAosMobileStatus,
};

#[test]
fn aos_mobile_status_has_pre_ml_type_four_and_88_byte_layout() {
    let status = RenaissanceAosMobileStatus {
        hits: (0x0102, 0x0304),
        can_rename: false,
        female: true,
        attributes: [5, 6, 7],
        stamina: (8, 9),
        mana: (10, 11),
        gold: 12,
        physical_resistance: 13,
        weight: 14,
        stat_cap: 15,
        followers: (2, 5),
        elemental_resistances: [16, 17, 18, 19],
        luck: 20,
        damage: (21, 22),
        tithing_points: 23,
    };
    let frame = encode_renaissance_aos_mobile_status(0x1122_3344, "Alice", &status);
    let mut expected = vec![0x11, 0, 88, 0x11, 0x22, 0x33, 0x44];
    expected.extend_from_slice(b"Alice");
    expected.extend_from_slice(&[0; 25]);
    expected.extend_from_slice(&[
        1, 2, 3, 4, 0, 4, 1, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9, 0, 10, 0, 11, 0, 0, 0, 12, 0, 13, 0, 14,
        0, 15, 2, 5, 0, 16, 0, 17, 0, 18, 0, 19, 0, 20, 0, 21, 0, 22, 0, 0, 0, 23,
    ]);
    assert_eq!(frame.len(), 88);
    assert_eq!(frame, expected);
}

#[test]
fn login_tail_fixed_packets_match_legacy_layout() {
    assert_eq!(encode_renaissance_global_light(7), [0x4F, 7]);
    assert_eq!(
        encode_renaissance_personal_light(0x1122_3344, 9),
        [0x4E, 0x11, 0x22, 0x33, 0x44, 9]
    );
    assert_eq!(encode_renaissance_login_complete(), [0x55]);
    assert_eq!(encode_renaissance_war_mode(false), [0x72, 0, 0, 0x32, 0]);
    assert_eq!(encode_renaissance_season_change(2), [0xBC, 2, 1]);
    assert_eq!(
        encode_renaissance_current_time(12, 34, 56),
        [0x5B, 12, 34, 56]
    );
}
