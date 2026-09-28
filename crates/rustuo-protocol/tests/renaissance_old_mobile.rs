use rustuo_protocol::{
    encode_renaissance_login_confirm, encode_renaissance_mobile_incoming_empty,
    encode_renaissance_mobile_update_old,
};

#[test]
fn empty_mobile_incoming_has_classic_variable_layout_and_terminator() {
    assert_eq!(
        encode_renaissance_mobile_incoming_empty(
            0x0102_0304,
            0x0190,
            0x0DAF,
            0x0A0E,
            14,
            2,
            0x0456,
            0x40,
            0x01
        ),
        [
            0x78, 0, 23, 1, 2, 3, 4, 1, 0x90, 0x0D, 0xAF, 0x0A, 0x0E, 14, 2, 4, 0x56, 0x40, 1, 0,
            0, 0, 0
        ]
    );
}

#[test]
fn mobile_update_old_has_classic_fixed_layout() {
    assert_eq!(
        encode_renaissance_mobile_update_old(
            0x0102_0304,
            0x0190,
            0x0DAF,
            0x0A0E,
            14,
            2,
            0x0456,
            0x40
        ),
        [0x20, 1, 2, 3, 4, 1, 0x90, 0, 4, 0x56, 0x40, 0x0D, 0xAF, 0x0A, 0x0E, 0, 0, 2, 14]
    );
}

#[test]
fn self_mobile_fields_agree_with_login_confirm() {
    let confirm = encode_renaissance_login_confirm(1, 0x0190, 0x0DAF, 0x0A0E, 14, 2, 7168, 4096);
    let incoming =
        encode_renaissance_mobile_incoming_empty(1, 0x0190, 0x0DAF, 0x0A0E, 14, 2, 0x0456, 0x40, 1);
    let update =
        encode_renaissance_mobile_update_old(1, 0x0190, 0x0DAF, 0x0A0E, 14, 2, 0x0456, 0x40);
    assert_eq!(incoming[3..7], confirm[1..5]);
    assert_eq!(update[1..5], confirm[1..5]);
    assert_eq!(incoming[7..9], confirm[9..11]);
    assert_eq!(update[5..7], confirm[9..11]);
    assert_eq!(incoming[9..13], confirm[11..15]);
    assert_eq!(update[11..15], confirm[11..15]);
    assert_eq!(incoming[13], confirm[16]);
    assert_eq!(update[18], confirm[16]);
    assert_eq!(incoming[14], confirm[17]);
    assert_eq!(update[17], confirm[17]);
}
