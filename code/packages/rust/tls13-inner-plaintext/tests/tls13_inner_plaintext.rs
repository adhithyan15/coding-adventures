use std::mem::size_of;

use tls13_inner_plaintext::{
    decode_tls13_inner_plaintext, encode_tls13_inner_plaintext, InnerPlaintextErrorKind,
    Tls13InnerPlaintextPolicy, TlsContentType, TLS13_MAX_INNER_PLAINTEXT_LEN,
};

fn policy(types: &[TlsContentType], maximum_encoded_length: usize) -> Tls13InnerPlaintextPolicy {
    Tls13InnerPlaintextPolicy::new(types, maximum_encoded_length).expect("valid policy")
}

#[test]
fn protocol_limit_matches_rfc_8446() {
    assert_eq!(TLS13_MAX_INNER_PLAINTEXT_LEN, (1 << 14) + 1);
}

#[test]
fn policy_is_explicit_bounded_and_duplicate_free() {
    assert_eq!(
        Tls13InnerPlaintextPolicy::new(&[], 1)
            .expect_err("empty allowlist must fail")
            .kind(),
        InnerPlaintextErrorKind::EmptyContentTypePolicy
    );
    assert_eq!(
        Tls13InnerPlaintextPolicy::new(
            &[TlsContentType::Handshake, TlsContentType::Handshake],
            16,
        )
        .expect_err("duplicate allowlist entry must fail")
        .kind(),
        InnerPlaintextErrorKind::DuplicateContentType
    );
    assert_eq!(
        Tls13InnerPlaintextPolicy::new(&[TlsContentType::ChangeCipherSpec], 16)
            .expect_err("protected ChangeCipherSpec must fail")
            .kind(),
        InnerPlaintextErrorKind::ProtectedChangeCipherSpec
    );
    for maximum in [0, TLS13_MAX_INNER_PLAINTEXT_LEN + 1] {
        assert_eq!(
            Tls13InnerPlaintextPolicy::new(&[TlsContentType::ApplicationData], maximum)
                .expect_err("invalid limit must fail")
                .kind(),
            InnerPlaintextErrorKind::EncodedLengthLimitOutOfRange
        );
    }
}

#[test]
fn each_protected_content_type_round_trips_with_padding() {
    let types = [
        TlsContentType::Alert,
        TlsContentType::Handshake,
        TlsContentType::ApplicationData,
    ];
    let policy = policy(&types, 64);

    for content_type in types {
        let content: &[u8] = if content_type == TlsContentType::ApplicationData {
            b""
        } else {
            b"message"
        };
        let mut encoded = [0xa5; 64];
        let used = encode_tls13_inner_plaintext(&policy, content_type, content, 5, &mut encoded)
            .expect("inner plaintext encodes");
        let decoded = decode_tls13_inner_plaintext(&policy, &encoded[..used])
            .expect("inner plaintext decodes");

        assert_eq!(decoded.content_type(), content_type);
        assert_eq!(decoded.content(), content);
        assert_eq!(decoded.padding_length(), 5);
        assert_eq!(&encoded[used - 5..used], &[0; 5]);
    }
}

#[test]
fn reverse_scan_preserves_zero_bytes_at_the_end_of_content() {
    let policy = policy(&[TlsContentType::ApplicationData], 16);
    let encoded = [0x41, 0x00, 23, 0, 0, 0];
    let decoded = decode_tls13_inner_plaintext(&policy, &encoded).expect("value decodes");

    assert_eq!(decoded.content(), &[0x41, 0x00]);
    assert_eq!(decoded.content_type(), TlsContentType::ApplicationData);
    assert_eq!(decoded.padding_length(), 3);
}

#[test]
fn decoder_rejects_malformed_or_disallowed_values() {
    let handshake = policy(&[TlsContentType::Handshake], 8);
    let cases: &[(&[u8], InnerPlaintextErrorKind)] = &[
        (&[], InnerPlaintextErrorKind::EmptyInnerPlaintext),
        (&[0, 0, 0], InnerPlaintextErrorKind::MissingContentType),
        (&[1, 19, 0], InnerPlaintextErrorKind::UnknownContentType),
        (
            &[1, 20, 0],
            InnerPlaintextErrorKind::ProtectedChangeCipherSpec,
        ),
        (&[1, 21, 0], InnerPlaintextErrorKind::DisallowedContentType),
        (&[22, 0], InnerPlaintextErrorKind::EmptyControlContent),
    ];

    for (encoded, expected) in cases {
        assert_eq!(
            decode_tls13_inner_plaintext(&handshake, encoded)
                .expect_err("invalid value must fail")
                .kind(),
            *expected
        );
    }
}

#[test]
fn length_limit_is_checked_before_the_reverse_scan() {
    let policy = policy(&[TlsContentType::ApplicationData], 3);
    assert_eq!(
        decode_tls13_inner_plaintext(&policy, &[0, 0, 0, 0])
            .expect_err("oversized value must fail before syntax")
            .kind(),
        InnerPlaintextErrorKind::InnerPlaintextTooLong
    );
}

#[test]
fn empty_handshake_and_alert_content_fail_for_decode_and_encode() {
    for content_type in [TlsContentType::Handshake, TlsContentType::Alert] {
        let policy = policy(&[content_type], 8);
        let encoded = [content_type.wire_value(), 0];
        assert_eq!(
            decode_tls13_inner_plaintext(&policy, &encoded)
                .expect_err("empty control content must fail")
                .kind(),
            InnerPlaintextErrorKind::EmptyControlContent
        );

        let mut output = [0xa5; 8];
        assert_eq!(
            encode_tls13_inner_plaintext(&policy, content_type, &[], 0, &mut output)
                .expect_err("empty control content must fail")
                .kind(),
            InnerPlaintextErrorKind::EmptyControlContent
        );
        assert_eq!(output, [0xa5; 8]);
    }
}

#[test]
fn encoder_preflights_every_failure_before_mutating_output() {
    let policy = policy(&[TlsContentType::ApplicationData], 4);
    let mut output = [0xa5; 8];

    let failures = [
        encode_tls13_inner_plaintext(
            &policy,
            TlsContentType::ChangeCipherSpec,
            &[1],
            0,
            &mut output,
        ),
        encode_tls13_inner_plaintext(&policy, TlsContentType::Handshake, &[1], 0, &mut output),
        encode_tls13_inner_plaintext(
            &policy,
            TlsContentType::ApplicationData,
            &[1, 2, 3, 4],
            0,
            &mut output,
        ),
        encode_tls13_inner_plaintext(
            &policy,
            TlsContentType::ApplicationData,
            &[],
            usize::MAX,
            &mut output,
        ),
    ];
    let expected = [
        InnerPlaintextErrorKind::ProtectedChangeCipherSpec,
        InnerPlaintextErrorKind::DisallowedContentType,
        InnerPlaintextErrorKind::InnerPlaintextTooLong,
        InnerPlaintextErrorKind::EncodedLengthOverflow,
    ];

    for (failure, expected_kind) in failures.into_iter().zip(expected) {
        assert_eq!(
            failure.expect_err("encoding must fail").kind(),
            expected_kind
        );
        assert_eq!(output, [0xa5; 8]);
    }

    let mut short = [0xa5; 3];
    assert_eq!(
        encode_tls13_inner_plaintext(
            &policy,
            TlsContentType::ApplicationData,
            &[1, 2, 3],
            0,
            &mut short,
        )
        .expect_err("short output must fail")
        .kind(),
        InnerPlaintextErrorKind::OutputTooSmall
    );
    assert_eq!(short, [0xa5; 3]);
}

#[test]
fn protocol_maximum_round_trips() {
    let policy = policy(
        &[TlsContentType::ApplicationData],
        TLS13_MAX_INNER_PLAINTEXT_LEN,
    );
    let content = vec![0x5a; 1 << 14];
    let mut encoded = vec![0xa5; TLS13_MAX_INNER_PLAINTEXT_LEN];
    let used = encode_tls13_inner_plaintext(
        &policy,
        TlsContentType::ApplicationData,
        &content,
        0,
        &mut encoded,
    )
    .expect("maximum inner plaintext encodes");
    assert_eq!(used, TLS13_MAX_INNER_PLAINTEXT_LEN);
    let decoded = decode_tls13_inner_plaintext(&policy, &encoded).expect("maximum decodes");
    assert_eq!(decoded.content(), content);
    assert_eq!(decoded.padding_length(), 0);
}

#[test]
fn validated_values_and_errors_remain_compact_and_redacted() {
    assert!(size_of::<tls13_inner_plaintext::Tls13InnerPlaintext<'_>>() <= 32);
    assert!(size_of::<Tls13InnerPlaintextPolicy>() <= 16);
    assert!(size_of::<tls13_inner_plaintext::InnerPlaintextError>() <= 1);

    let policy = policy(&[TlsContentType::ApplicationData], 8);
    let error = decode_tls13_inner_plaintext(&policy, &[0x73, 0x65, 0x63, 0x72, 0x65, 0x74, 19])
        .expect_err("unknown content type must fail");
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains("secret"));
    assert!(!rendered.contains("19"));
}
