use forme_plugin_runner_rs::{
    decode_wire_value, encode_frame, FrameDecoder, ProtocolError, WireValue,
};

#[test]
fn bytes_and_reserved_objects_round_trip_without_colliding() {
    let value = WireValue::object([
        ("bytes", WireValue::Bytes(vec![0, 1, 2, 255])),
        ("$forme", WireValue::String("caller-owned-value".to_owned())),
    ]);
    let frame = encode_frame(&value, 4096).unwrap();
    let mut decoder = FrameDecoder::new(4096, 512).unwrap();
    assert_eq!(decoder.push(&frame).unwrap(), vec![value]);
    decoder.finish().unwrap();
}

#[test]
fn decoder_rejects_noncanonical_and_oversized_headers_before_payload_allocation() {
    let mut decoder = FrameDecoder::new(8, 64).unwrap();
    let noncanonical = decoder.push(b"Content-Length: 01\r\n\r\n{}").unwrap_err();
    assert!(matches!(noncanonical, ProtocolError::InvalidFrame(_)));

    let mut decoder = FrameDecoder::new(8, 64).unwrap();
    let oversized = decoder.push(b"Content-Length: 9\r\n\r\n").unwrap_err();
    assert!(matches!(oversized, ProtocolError::ResourceLimit(_)));
}

#[test]
fn malformed_binary_and_escaped_object_envelopes_fail_closed() {
    let malformed_bytes = serde_json::json!({"$forme": "bytes", "base64": "!"});
    assert!(decode_wire_value(malformed_bytes).is_err());
    let duplicate = serde_json::json!({
        "$forme": "escaped-object",
        "entries": [["x", 1], ["x", 2]]
    });
    assert!(decode_wire_value(duplicate).is_err());
}
