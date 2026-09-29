use rustuo_protocol::{
    decode_renaissance_ping_request, encode_renaissance_ping_ack, RenaissancePingDecodeError,
    RenaissancePingRequest,
};

#[test]
fn ping_request_preserves_the_client_sequence_and_ack_echoes_it() {
    assert_eq!(
        decode_renaissance_ping_request(&[0x73, 0xA5]),
        Ok(RenaissancePingRequest { sequence: 0xA5 })
    );
    assert_eq!(encode_renaissance_ping_ack(0xA5), [0x73, 0xA5]);
}

#[test]
fn ping_request_rejects_wrong_id_and_nonexact_length() {
    assert_eq!(
        decode_renaissance_ping_request(&[]),
        Err(RenaissancePingDecodeError::Truncated { length: 0 })
    );
    assert_eq!(
        decode_renaissance_ping_request(&[0x72, 0xA5]),
        Err(RenaissancePingDecodeError::WrongPacketId { packet_id: 0x72 })
    );
    assert_eq!(
        decode_renaissance_ping_request(&[0x73]),
        Err(RenaissancePingDecodeError::Truncated { length: 1 })
    );
    assert_eq!(
        decode_renaissance_ping_request(&[0x73, 0xA5, 0x5A]),
        Err(RenaissancePingDecodeError::InvalidLength { length: 3 })
    );
}
