use std::mem::size_of;

use tls13_record::{
    decode_tls13_ciphertext, decode_tls13_ciphertext_prefix, decode_tls13_plaintext,
    decode_tls13_plaintext_prefix, encode_tls13_ciphertext, encode_tls13_plaintext,
    RecordErrorKind, Tls13CiphertextPolicy, Tls13PlaintextPolicy, TlsContentType,
    TLS13_LEGACY_RECORD_VERSION, TLS13_MAX_CIPHERTEXT_FRAGMENT_LEN,
    TLS13_MAX_PLAINTEXT_FRAGMENT_LEN, TLS_RECORD_HEADER_LEN,
};

fn plaintext_policy(
    types: &[TlsContentType],
    maximum_fragment_length: usize,
) -> Tls13PlaintextPolicy {
    Tls13PlaintextPolicy::new(types, maximum_fragment_length).expect("valid plaintext policy")
}

fn ciphertext_policy(maximum_fragment_length: usize) -> Tls13CiphertextPolicy {
    Tls13CiphertextPolicy::new(maximum_fragment_length).expect("valid ciphertext policy")
}

#[test]
fn constants_match_rfc_8446_record_limits() {
    assert_eq!(TLS_RECORD_HEADER_LEN, 5);
    assert_eq!(TLS13_LEGACY_RECORD_VERSION, 0x0303);
    assert_eq!(TLS13_MAX_PLAINTEXT_FRAGMENT_LEN, 1 << 14);
    assert_eq!(TLS13_MAX_CIPHERTEXT_FRAGMENT_LEN, (1 << 14) + 256);
}

#[test]
fn plaintext_policy_is_explicit_non_empty_and_duplicate_free() {
    assert_eq!(
        Tls13PlaintextPolicy::new(&[], 1)
            .expect_err("empty allowlist must fail")
            .kind(),
        RecordErrorKind::EmptyContentTypePolicy
    );
    assert_eq!(
        Tls13PlaintextPolicy::new(&[TlsContentType::Handshake, TlsContentType::Handshake], 1,)
            .expect_err("duplicates must fail")
            .kind(),
        RecordErrorKind::DuplicateContentType
    );
    for maximum in [0, TLS13_MAX_PLAINTEXT_FRAGMENT_LEN + 1] {
        assert_eq!(
            Tls13PlaintextPolicy::new(&[TlsContentType::Handshake], maximum)
                .expect_err("invalid maximum must fail")
                .kind(),
            RecordErrorKind::FragmentLimitOutOfRange
        );
    }
}

#[test]
fn ciphertext_policy_enforces_the_protocol_ceiling() {
    for maximum in [0, TLS13_MAX_CIPHERTEXT_FRAGMENT_LEN + 1] {
        assert_eq!(
            Tls13CiphertextPolicy::new(maximum)
                .expect_err("invalid maximum must fail")
                .kind(),
            RecordErrorKind::FragmentLimitOutOfRange
        );
    }
}

#[test]
fn plaintext_round_trips_each_known_content_type() {
    let types = [
        TlsContentType::ChangeCipherSpec,
        TlsContentType::Alert,
        TlsContentType::Handshake,
        TlsContentType::ApplicationData,
    ];
    let policy = plaintext_policy(&types, 64);

    for content_type in types {
        let fragment: &[u8] = if content_type == TlsContentType::ApplicationData {
            b""
        } else {
            b"record"
        };
        let mut encoded = [0xa5; 64];
        let encoded_length = encode_tls13_plaintext(&policy, content_type, fragment, &mut encoded)
            .expect("record encodes");
        let decoded =
            decode_tls13_plaintext(&policy, &encoded[..encoded_length]).expect("record decodes");

        assert_eq!(decoded.content_type(), content_type);
        assert_eq!(decoded.legacy_record_version(), TLS13_LEGACY_RECORD_VERSION);
        assert_eq!(decoded.fragment(), fragment);
    }
}

#[test]
fn plaintext_prefix_decode_returns_the_exact_stream_tail() {
    let policy = plaintext_policy(&[TlsContentType::Handshake], 16);
    let bytes = [22, 3, 3, 0, 3, 1, 2, 3, 21, 3, 3];
    let (record, remainder) =
        decode_tls13_plaintext_prefix(&policy, &bytes).expect("prefix decodes");

    assert_eq!(record.fragment(), &[1, 2, 3]);
    assert_eq!(remainder, &[21, 3, 3]);
    assert_eq!(
        decode_tls13_plaintext(&policy, &bytes)
            .expect_err("exact mode rejects tail")
            .kind(),
        RecordErrorKind::TrailingData
    );
}

#[test]
fn plaintext_ignores_and_retains_every_legacy_version_value() {
    let policy = plaintext_policy(&[TlsContentType::Handshake], 16);

    for legacy_version in [0x0301_u16, 0x0303, 0xffff] {
        let [version_high, version_low] = legacy_version.to_be_bytes();
        let bytes = [22, version_high, version_low, 0, 1, 1];
        let record = decode_tls13_plaintext(&policy, &bytes)
            .expect("plaintext legacy version must be ignored");
        assert_eq!(record.legacy_record_version(), legacy_version);
        assert_eq!(record.fragment(), &[1]);
    }
}

#[test]
fn plaintext_rejects_bad_headers_lengths_and_empty_control_records() {
    let policy = plaintext_policy(&[TlsContentType::Handshake], 3);
    let cases: &[(&[u8], RecordErrorKind)] = &[
        (&[22, 3, 3, 0], RecordErrorKind::TruncatedHeader),
        (&[19, 3, 3, 0, 1, 0], RecordErrorKind::UnknownContentType),
        (&[21, 3, 3, 0, 1, 0], RecordErrorKind::DisallowedContentType),
        (
            &[22, 3, 3, 0, 4, 0, 0, 0, 0],
            RecordErrorKind::FragmentTooLong,
        ),
        (&[22, 3, 3, 0, 2, 0], RecordErrorKind::TruncatedFragment),
        (&[22, 3, 3, 0, 0], RecordErrorKind::EmptyControlFragment),
    ];

    for (bytes, expected) in cases {
        assert_eq!(
            decode_tls13_plaintext(&policy, bytes)
                .expect_err("malformed record must fail")
                .kind(),
            *expected
        );
    }
}

#[test]
fn ciphertext_round_trips_and_prefix_decode_preserves_remainder() {
    let policy = ciphertext_policy(32);
    let fragment = [0x11, 0x22, 0x33, 0x44];
    let mut encoded = [0xa5; 32];
    let encoded_length =
        encode_tls13_ciphertext(&policy, &fragment, &mut encoded).expect("ciphertext encodes");
    encoded[encoded_length] = 0xee;

    let (decoded, remainder) =
        decode_tls13_ciphertext_prefix(&policy, &encoded[..encoded_length + 1])
            .expect("prefix decodes");
    assert_eq!(
        decoded.outer_content_type(),
        TlsContentType::ApplicationData
    );
    assert_eq!(decoded.legacy_record_version(), TLS13_LEGACY_RECORD_VERSION);
    assert_eq!(decoded.fragment(), fragment);
    assert_eq!(remainder, &[0xee]);
    assert_eq!(
        decode_tls13_ciphertext(&policy, &encoded[..encoded_length + 1])
            .expect_err("exact mode rejects tail")
            .kind(),
        RecordErrorKind::TrailingData
    );
}

#[test]
fn ciphertext_rejects_bad_outer_framing() {
    let policy = ciphertext_policy(3);
    let cases: &[(&[u8], RecordErrorKind)] = &[
        (&[23, 3, 3, 0], RecordErrorKind::TruncatedHeader),
        (
            &[22, 3, 3, 0, 1, 0],
            RecordErrorKind::InvalidCiphertextContentType,
        ),
        (
            &[23, 3, 1, 0, 1, 0],
            RecordErrorKind::InvalidLegacyRecordVersion,
        ),
        (&[23, 3, 3, 0, 0], RecordErrorKind::EmptyCiphertextFragment),
        (
            &[23, 3, 3, 0, 4, 0, 0, 0, 0],
            RecordErrorKind::FragmentTooLong,
        ),
        (&[23, 3, 3, 0, 2, 0], RecordErrorKind::TruncatedFragment),
    ];

    for (bytes, expected) in cases {
        assert_eq!(
            decode_tls13_ciphertext(&policy, bytes)
                .expect_err("malformed record must fail")
                .kind(),
            *expected
        );
    }
}

#[test]
fn protocol_maximum_fragments_round_trip_with_exact_u16_lengths() {
    let plaintext_policy = plaintext_policy(
        &[TlsContentType::ApplicationData],
        TLS13_MAX_PLAINTEXT_FRAGMENT_LEN,
    );
    let plaintext_fragment = vec![0x5a; TLS13_MAX_PLAINTEXT_FRAGMENT_LEN];
    let mut plaintext_record = vec![0xa5; TLS_RECORD_HEADER_LEN + TLS13_MAX_PLAINTEXT_FRAGMENT_LEN];
    let plaintext_length = encode_tls13_plaintext(
        &plaintext_policy,
        TlsContentType::ApplicationData,
        &plaintext_fragment,
        &mut plaintext_record,
    )
    .expect("maximum plaintext encodes");
    assert_eq!(&plaintext_record[3..5], &[0x40, 0x00]);
    assert_eq!(
        decode_tls13_plaintext(&plaintext_policy, &plaintext_record[..plaintext_length])
            .expect("maximum plaintext decodes")
            .fragment(),
        plaintext_fragment
    );

    let ciphertext_policy = ciphertext_policy(TLS13_MAX_CIPHERTEXT_FRAGMENT_LEN);
    let ciphertext_fragment = vec![0x6b; TLS13_MAX_CIPHERTEXT_FRAGMENT_LEN];
    let mut ciphertext_record =
        vec![0xa5; TLS_RECORD_HEADER_LEN + TLS13_MAX_CIPHERTEXT_FRAGMENT_LEN];
    let ciphertext_length = encode_tls13_ciphertext(
        &ciphertext_policy,
        &ciphertext_fragment,
        &mut ciphertext_record,
    )
    .expect("maximum ciphertext encodes");
    assert_eq!(&ciphertext_record[3..5], &[0x41, 0x00]);
    assert_eq!(
        decode_tls13_ciphertext(&ciphertext_policy, &ciphertext_record[..ciphertext_length])
            .expect("maximum ciphertext decodes")
            .fragment(),
        ciphertext_fragment
    );
}

#[test]
fn encoders_preflight_every_failure_before_mutating_output() {
    let plaintext_policy = plaintext_policy(&[TlsContentType::Handshake], 3);
    let ciphertext_policy = ciphertext_policy(3);
    let mut output = [0xa5; 8];

    let failures = [
        encode_tls13_plaintext(&plaintext_policy, TlsContentType::Alert, &[1], &mut output),
        encode_tls13_plaintext(
            &plaintext_policy,
            TlsContentType::Handshake,
            &[],
            &mut output,
        ),
        encode_tls13_plaintext(
            &plaintext_policy,
            TlsContentType::Handshake,
            &[1, 2, 3, 4],
            &mut output,
        ),
        encode_tls13_ciphertext(&ciphertext_policy, &[], &mut output),
        encode_tls13_ciphertext(&ciphertext_policy, &[1, 2, 3, 4], &mut output),
    ];
    let expected = [
        RecordErrorKind::DisallowedContentType,
        RecordErrorKind::EmptyControlFragment,
        RecordErrorKind::FragmentTooLong,
        RecordErrorKind::EmptyCiphertextFragment,
        RecordErrorKind::FragmentTooLong,
    ];
    for (failure, expected_kind) in failures.into_iter().zip(expected) {
        assert_eq!(
            failure.expect_err("encoding must fail").kind(),
            expected_kind
        );
        assert_eq!(output, [0xa5; 8]);
    }

    let mut short = [0xa5; TLS_RECORD_HEADER_LEN];
    assert_eq!(
        encode_tls13_plaintext(
            &plaintext_policy,
            TlsContentType::Handshake,
            &[1],
            &mut short,
        )
        .expect_err("short output must fail")
        .kind(),
        RecordErrorKind::OutputTooSmall
    );
    assert_eq!(short, [0xa5; TLS_RECORD_HEADER_LEN]);
}

#[test]
fn validated_records_remain_compact_borrowed_evidence() {
    assert!(size_of::<tls13_record::Tls13PlaintextRecord<'_>>() <= 32);
    assert!(size_of::<tls13_record::Tls13CiphertextRecord<'_>>() <= 24);
}
