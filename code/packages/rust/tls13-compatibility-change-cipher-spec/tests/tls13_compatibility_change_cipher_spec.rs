use tls13_compatibility_change_cipher_spec::{
    decode_compatibility_change_cipher_spec, encode_compatibility_change_cipher_spec,
    CompatibilityChangeCipherSpecErrorKind, COMPATIBILITY_CHANGE_CIPHER_SPEC_BODY_LENGTH,
};

#[test]
fn canonical_body_matches_rfc_8446() {
    assert_eq!(COMPATIBILITY_CHANGE_CIPHER_SPEC_BODY_LENGTH, 1);

    let message = decode_compatibility_change_cipher_spec(&[1]).expect("canonical body decodes");
    assert_eq!(message.wire_value(), 1);
    assert_eq!(format!("{message:?}"), "CompatibilityChangeCipherSpec");
}

#[test]
fn exact_decoder_rejects_truncation_and_trailing_bytes() {
    for body in [&[][..], &[1, 0][..], &[1, 1, 1][..]] {
        assert_eq!(
            decode_compatibility_change_cipher_spec(body)
                .expect_err("non-exact body must fail")
                .kind(),
            CompatibilityChangeCipherSpecErrorKind::InvalidBodyLength
        );
    }
}

#[test]
fn every_noncanonical_single_octet_fails_closed() {
    for value in 0..=u8::MAX {
        if value == 1 {
            continue;
        }
        assert_eq!(
            decode_compatibility_change_cipher_spec(&[value])
                .expect_err("noncanonical body must fail")
                .kind(),
            CompatibilityChangeCipherSpecErrorKind::InvalidBody
        );
    }
}

#[test]
fn encoder_emits_canonical_body_and_leaves_extra_capacity_untouched() {
    let message = decode_compatibility_change_cipher_spec(&[1]).expect("canonical body decodes");
    let mut output = [0xa5; 3];

    let used = encode_compatibility_change_cipher_spec(message, &mut output)
        .expect("canonical body encodes");

    assert_eq!(used, COMPATIBILITY_CHANGE_CIPHER_SPEC_BODY_LENGTH);
    assert_eq!(output, [1, 0xa5, 0xa5]);
    assert_eq!(
        decode_compatibility_change_cipher_spec(&output[..used])
            .expect("encoded body decodes")
            .wire_value(),
        1
    );
}

#[test]
fn empty_output_fails_before_any_write() {
    let message = decode_compatibility_change_cipher_spec(&[1]).expect("canonical body decodes");
    let mut storage = [0xa5; 1];

    assert_eq!(
        encode_compatibility_change_cipher_spec(message, &mut storage[..0])
            .expect_err("empty output must fail")
            .kind(),
        CompatibilityChangeCipherSpecErrorKind::OutputTooSmall
    );
    assert_eq!(storage, [0xa5]);
}

#[test]
fn errors_are_stable_and_retain_no_input_payload() {
    let invalid =
        decode_compatibility_change_cipher_spec(&[255]).expect_err("noncanonical body must fail");
    assert_eq!(
        invalid.to_string(),
        "TLS 1.3 compatibility ChangeCipherSpec error InvalidBody"
    );
    assert!(!format!("{invalid:?}").contains("255"));

    let length = decode_compatibility_change_cipher_spec(&[]).expect_err("empty body must fail");
    assert_eq!(
        length.to_string(),
        "TLS 1.3 compatibility ChangeCipherSpec error InvalidBodyLength"
    );
}
