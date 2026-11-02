use rsa_pss_encoded_message_layout::{
    decode_rsa_pss_encoded_message_layout, RsaPssEncodedMessageLayoutErrorKind,
    RsaPssEncodedMessageLayoutPolicy, RsaPssEncodedMessageLayoutPolicyErrorKind,
    RSA_PSS_TRAILER_FIELD,
};

fn encoded_message(length: usize, first_octet: u8) -> Vec<u8> {
    let mut encoded = vec![0u8; length];
    encoded[0] = first_octet;
    encoded[length - 1] = RSA_PSS_TRAILER_FIELD;
    encoded
}

#[test]
fn validates_typical_sha256_layout_and_exact_borrows() {
    let policy = RsaPssEncodedMessageLayoutPolicy::new(32, 32, 256).unwrap();
    let mut encoded = encoded_message(256, 0x7f);
    for (index, octet) in encoded[223..255].iter_mut().enumerate() {
        *octet = index as u8;
    }

    let layout = decode_rsa_pss_encoded_message_layout(&encoded, 2047, policy).unwrap();
    assert_eq!(layout.encoded_message(), encoded);
    assert_eq!(layout.masked_data_block(), &encoded[..223]);
    assert_eq!(layout.hash(), &encoded[223..255]);
    assert_eq!(layout.encoded_message_bits(), 2047);
    assert_eq!(layout.salt_length(), 32);
    assert_eq!(layout.unused_high_bits(), 1);
    assert_eq!(format!("{layout:?}"), "ValidatedRsaPssEncodedMessageLayout");
}

#[test]
fn supports_data_driven_hash_salt_and_bit_boundaries() {
    for (hash_length, salt_length, encoded_bits, first_octet, unused_bits) in [
        (48, 0, 399, 0x7f, 1),
        (64, 64, 1039, 0x7f, 1),
        (32, 32, 521, 0x01, 7),
        (32, 32, 528, 0xff, 0),
    ] {
        let encoded_length = encoded_bits / 8 + usize::from(encoded_bits % 8 != 0);
        let policy =
            RsaPssEncodedMessageLayoutPolicy::new(hash_length, salt_length, encoded_length)
                .unwrap();
        let encoded = encoded_message(encoded_length, first_octet);
        let layout = decode_rsa_pss_encoded_message_layout(&encoded, encoded_bits, policy).unwrap();
        assert_eq!(layout.hash().len(), hash_length);
        assert_eq!(layout.salt_length(), salt_length);
        assert_eq!(layout.unused_high_bits(), unused_bits);
        assert_eq!(
            layout.masked_data_block().len(),
            encoded_length - hash_length - 1
        );
    }
}

#[test]
fn policy_accepts_zero_salt_and_reports_public_bounds() {
    let policy = RsaPssEncodedMessageLayoutPolicy::new(48, 0, 512).unwrap();
    assert_eq!(policy.hash_length(), 48);
    assert_eq!(policy.salt_length(), 0);
    assert_eq!(policy.maximum_encoded_message_bytes(), 512);
    assert_eq!(policy.minimum_encoded_message_bytes(), 50);
}

#[test]
fn policy_rejects_zero_lengths_overflow_and_unusable_ceiling() {
    let zero_hash = RsaPssEncodedMessageLayoutPolicy::new(0, 0, 1).unwrap_err();
    assert_eq!(
        zero_hash.kind(),
        RsaPssEncodedMessageLayoutPolicyErrorKind::ZeroHashLength
    );

    let zero_ceiling = RsaPssEncodedMessageLayoutPolicy::new(32, 0, 0).unwrap_err();
    assert_eq!(
        zero_ceiling.kind(),
        RsaPssEncodedMessageLayoutPolicyErrorKind::ZeroMaximumEncodedMessageBytes
    );

    for (hash_length, salt_length) in [(usize::MAX, 1), (usize::MAX - 1, 0)] {
        let overflow = RsaPssEncodedMessageLayoutPolicy::new(hash_length, salt_length, usize::MAX)
            .unwrap_err();
        assert_eq!(
            overflow.kind(),
            RsaPssEncodedMessageLayoutPolicyErrorKind::LengthOverflow
        );
    }

    let too_small = RsaPssEncodedMessageLayoutPolicy::new(32, 32, 65).unwrap_err();
    assert_eq!(
        too_small.kind(),
        RsaPssEncodedMessageLayoutPolicyErrorKind::MaximumEncodedMessageTooSmall
    );
}

#[test]
fn rejects_input_over_ceiling_before_other_layout_checks() {
    let policy = RsaPssEncodedMessageLayoutPolicy::new(32, 32, 66).unwrap();
    let encoded = encoded_message(67, 0);
    let error = decode_rsa_pss_encoded_message_layout(&encoded, 0, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssEncodedMessageLayoutErrorKind::EncodedMessageTooLongForPolicy
    );
}

#[test]
fn rejects_zero_bits_and_exact_length_mismatches() {
    let policy = RsaPssEncodedMessageLayoutPolicy::new(32, 32, 128).unwrap();
    let zero_bits = decode_rsa_pss_encoded_message_layout(&[], 0, policy).unwrap_err();
    assert_eq!(
        zero_bits.kind(),
        RsaPssEncodedMessageLayoutErrorKind::ZeroEncodedMessageBits
    );

    for encoded in [encoded_message(65, 0), encoded_message(67, 0)] {
        let error = decode_rsa_pss_encoded_message_layout(&encoded, 528, policy).unwrap_err();
        assert_eq!(
            error.kind(),
            RsaPssEncodedMessageLayoutErrorKind::EncodedMessageLengthMismatch
        );
    }
}

#[test]
fn rejects_exact_width_that_cannot_hold_selected_parameters() {
    let policy = RsaPssEncodedMessageLayoutPolicy::new(32, 32, 128).unwrap();
    let encoded = encoded_message(65, 0);
    let error = decode_rsa_pss_encoded_message_layout(&encoded, 520, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssEncodedMessageLayoutErrorKind::EncodedMessageTooShortForParameters
    );
}

#[test]
fn rejects_wrong_trailer_without_returning_fields() {
    let policy = RsaPssEncodedMessageLayoutPolicy::new(32, 32, 66).unwrap();
    let mut encoded = encoded_message(66, 0);
    encoded[65] = 0xbd;
    let error = decode_rsa_pss_encoded_message_layout(&encoded, 528, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssEncodedMessageLayoutErrorKind::InvalidTrailerField
    );
}

#[test]
fn rejects_every_forbidden_high_bit_boundary() {
    let policy = RsaPssEncodedMessageLayoutPolicy::new(32, 32, 66).unwrap();
    for (encoded_bits, forbidden_octet) in [(527, 0x80), (521, 0x02), (521, 0xfe)] {
        let encoded = encoded_message(66, forbidden_octet);
        let error =
            decode_rsa_pss_encoded_message_layout(&encoded, encoded_bits, policy).unwrap_err();
        assert_eq!(
            error.kind(),
            RsaPssEncodedMessageLayoutErrorKind::NonzeroUnusedHighBits
        );
    }
}

#[test]
fn errors_are_stable_and_redacted() {
    let policy_error = RsaPssEncodedMessageLayoutPolicy::new(0, 125, 512).unwrap_err();
    assert_eq!(
        format!("{policy_error:?}"),
        "RsaPssEncodedMessageLayoutPolicyError { kind: ZeroHashLength }"
    );
    assert_eq!(
        policy_error.to_string(),
        "RSA-PSS layout policy error ZeroHashLength"
    );

    let policy = RsaPssEncodedMessageLayoutPolicy::new(32, 32, 66).unwrap();
    let error = decode_rsa_pss_encoded_message_layout(&[0x7d; 66], 528, policy).unwrap_err();
    assert_eq!(
        format!("{error:?}"),
        "RsaPssEncodedMessageLayoutError { kind: InvalidTrailerField }"
    );
    assert_eq!(
        error.to_string(),
        "RSA-PSS encoded-message layout error InvalidTrailerField"
    );
    for rendered in [
        format!("{policy_error:?}"),
        policy_error.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("125"));
        assert!(!rendered.contains("0x7d"));
    }
}
