use tls13_key_update_message::{
    decode_key_update, encode_key_update, KeyUpdateErrorKind, KeyUpdateRequest,
    KEY_UPDATE_BODY_LENGTH,
};

#[test]
fn protocol_constants_and_response_requirements_match_rfc_8446() {
    assert_eq!(KEY_UPDATE_BODY_LENGTH, 1);
    assert_eq!(KeyUpdateRequest::UpdateNotRequested.wire_value(), 0);
    assert_eq!(KeyUpdateRequest::UpdateRequested.wire_value(), 1);

    assert!(!KeyUpdateRequest::UpdateNotRequested.requires_response());
    assert_eq!(
        KeyUpdateRequest::UpdateNotRequested.required_response(),
        None
    );
    assert!(KeyUpdateRequest::UpdateRequested.requires_response());
    assert_eq!(
        KeyUpdateRequest::UpdateRequested.required_response(),
        Some(KeyUpdateRequest::UpdateNotRequested)
    );
}

#[test]
fn both_valid_requests_decode_exactly() {
    assert_eq!(
        decode_key_update(&[0]).expect("non-requesting update decodes"),
        KeyUpdateRequest::UpdateNotRequested
    );
    assert_eq!(
        decode_key_update(&[1]).expect("requesting update decodes"),
        KeyUpdateRequest::UpdateRequested
    );
}

#[test]
fn exact_decoder_rejects_truncation_and_padding() {
    for body in [&[][..], &[0, 0][..], &[1, 0, 0][..]] {
        assert_eq!(
            decode_key_update(body)
                .expect_err("non-exact body must fail")
                .kind(),
            KeyUpdateErrorKind::InvalidBodyLength
        );
    }
}

#[test]
fn every_out_of_range_request_fails_closed() {
    for value in 2..=u8::MAX {
        assert_eq!(
            decode_key_update(&[value])
                .expect_err("out-of-range request must fail")
                .kind(),
            KeyUpdateErrorKind::InvalidRequest
        );
    }
}

#[test]
fn encoder_round_trips_both_requests_and_leaves_extra_capacity() {
    for request in [
        KeyUpdateRequest::UpdateNotRequested,
        KeyUpdateRequest::UpdateRequested,
    ] {
        let mut output = [0xa5; 3];
        let used = encode_key_update(request, &mut output).expect("request encodes");
        assert_eq!(used, KEY_UPDATE_BODY_LENGTH);
        assert_eq!(output[0], request.wire_value());
        assert_eq!(&output[used..], &[0xa5, 0xa5]);
        assert_eq!(
            decode_key_update(&output[..used]).expect("encoded request decodes"),
            request
        );
    }
}

#[test]
fn empty_output_fails_without_an_output_write() {
    let mut storage = [0xa5; 1];
    assert_eq!(
        encode_key_update(KeyUpdateRequest::UpdateRequested, &mut storage[..0])
            .expect_err("empty output must fail")
            .kind(),
        KeyUpdateErrorKind::OutputTooSmall
    );
    assert_eq!(storage, [0xa5]);
}

#[test]
fn errors_are_stable_and_do_not_retain_request_values() {
    let invalid = decode_key_update(&[255]).expect_err("invalid request must fail");
    assert_eq!(
        invalid.to_string(),
        "TLS 1.3 KeyUpdate error InvalidRequest"
    );
    assert!(!format!("{invalid:?}").contains("255"));

    let length = decode_key_update(&[]).expect_err("empty body must fail");
    assert_eq!(
        length.to_string(),
        "TLS 1.3 KeyUpdate error InvalidBodyLength"
    );
}
