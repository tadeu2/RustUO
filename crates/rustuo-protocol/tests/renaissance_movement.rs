use rustuo_protocol::{
    decode_renaissance_movement_request, encode_renaissance_movement_ack,
    encode_renaissance_movement_rejection, RenaissanceMovementDecodeError,
    RenaissanceMovementRequest,
};

#[test]
fn movement_request_preserves_raw_direction_sequence_and_signed_big_endian_key() {
    assert_eq!(
        decode_renaissance_movement_request(&[0x02, 0x87, 0xFE, 0xFF, 0xFF, 0xFF, 0x80]),
        Ok(RenaissanceMovementRequest {
            direction: 0x87,
            sequence: 0xFE,
            fastwalk_key: -128,
        })
    );
}

#[test]
fn movement_request_rejects_wrong_id_and_nonexact_length() {
    assert_eq!(
        decode_renaissance_movement_request(&[]),
        Err(RenaissanceMovementDecodeError::Truncated { length: 0 })
    );
    assert_eq!(
        decode_renaissance_movement_request(&[0x03, 0, 0, 0, 0, 0, 0]),
        Err(RenaissanceMovementDecodeError::WrongPacketId { packet_id: 0x03 })
    );
    assert_eq!(
        decode_renaissance_movement_request(&[0x02, 0, 0, 0, 0, 0]),
        Err(RenaissanceMovementDecodeError::Truncated { length: 6 })
    );
    assert_eq!(
        decode_renaissance_movement_request(&[0x02, 0, 0, 0, 0, 0, 0, 0]),
        Err(RenaissanceMovementDecodeError::InvalidLength { length: 8 })
    );
}

#[test]
fn movement_ack_writes_sequence_and_notoriety() {
    assert_eq!(
        encode_renaissance_movement_ack(0xFE, 0x07),
        [0x22, 0xFE, 0x07]
    );
}

#[test]
fn movement_rejection_writes_big_endian_position_raw_facing_and_signed_z() {
    assert_eq!(
        encode_renaissance_movement_rejection(0xFE, 0x1234, 0xABCD, 0x87, -5),
        [0x21, 0xFE, 0x12, 0x34, 0xAB, 0xCD, 0x87, 0xFB]
    );
}
