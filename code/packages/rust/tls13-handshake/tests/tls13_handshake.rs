use std::mem::size_of;

use tls13_handshake::{
    decode_tls13_handshake, decode_tls13_handshake_prefix, encode_tls13_handshake, HandshakeError,
    HandshakeErrorKind, Tls13HandshakePolicy, Tls13HandshakeType, TLS13_HANDSHAKE_HEADER_LEN,
    TLS13_MAX_HANDSHAKE_BODY_LEN,
};

const ALL_TYPES: [Tls13HandshakeType; 10] = [
    Tls13HandshakeType::ClientHello,
    Tls13HandshakeType::ServerHello,
    Tls13HandshakeType::NewSessionTicket,
    Tls13HandshakeType::EndOfEarlyData,
    Tls13HandshakeType::EncryptedExtensions,
    Tls13HandshakeType::Certificate,
    Tls13HandshakeType::CertificateRequest,
    Tls13HandshakeType::CertificateVerify,
    Tls13HandshakeType::Finished,
    Tls13HandshakeType::KeyUpdate,
];

fn policy(types: &[Tls13HandshakeType], maximum_body_length: usize) -> Tls13HandshakePolicy {
    Tls13HandshakePolicy::new(types, maximum_body_length).expect("valid handshake policy")
}

#[test]
fn constants_and_wire_values_match_rfc_8446() {
    assert_eq!(TLS13_HANDSHAKE_HEADER_LEN, 4);
    assert_eq!(TLS13_MAX_HANDSHAKE_BODY_LEN, 0x00ff_ffff);
    assert_eq!(
        ALL_TYPES.map(Tls13HandshakeType::wire_value),
        [1, 2, 4, 5, 8, 11, 13, 15, 20, 24]
    );
}

#[test]
fn policy_is_explicit_duplicate_free_and_protocol_bounded() {
    assert_eq!(
        Tls13HandshakePolicy::new(&[], 0)
            .expect_err("empty allowlist must fail")
            .kind(),
        HandshakeErrorKind::EmptyHandshakeTypePolicy
    );
    assert_eq!(
        Tls13HandshakePolicy::new(
            &[Tls13HandshakeType::Finished, Tls13HandshakeType::Finished],
            32,
        )
        .expect_err("duplicate type must fail")
        .kind(),
        HandshakeErrorKind::DuplicateHandshakeType
    );
    assert_eq!(
        Tls13HandshakePolicy::new(
            &[Tls13HandshakeType::Finished],
            TLS13_MAX_HANDSHAKE_BODY_LEN + 1,
        )
        .expect_err("oversized policy must fail")
        .kind(),
        HandshakeErrorKind::BodyLimitOutOfRange
    );

    let empty_body_policy = policy(&[Tls13HandshakeType::EndOfEarlyData], 0);
    assert!(empty_body_policy.allows(Tls13HandshakeType::EndOfEarlyData));
    assert!(!empty_body_policy.allows(Tls13HandshakeType::Finished));
    assert_eq!(empty_body_policy.maximum_body_length(), 0);

    let maximum_policy = policy(
        &[Tls13HandshakeType::Certificate],
        TLS13_MAX_HANDSHAKE_BODY_LEN,
    );
    assert_eq!(
        maximum_policy.maximum_body_length(),
        TLS13_MAX_HANDSHAKE_BODY_LEN
    );
}

#[test]
fn every_recognized_wire_type_round_trips() {
    let policy = policy(&ALL_TYPES, 16);

    for handshake_type in ALL_TYPES {
        let body: &[u8] = if handshake_type == Tls13HandshakeType::EndOfEarlyData {
            b""
        } else {
            b"body"
        };
        let mut output = [0xa5; 32];
        let used = encode_tls13_handshake(&policy, handshake_type, body, &mut output)
            .expect("message encodes");
        let message = decode_tls13_handshake(&policy, &output[..used]).expect("message decodes");

        assert_eq!(message.handshake_type(), handshake_type);
        assert_eq!(message.body(), body);
        assert_eq!(message.encoded(), &output[..used]);
        assert_eq!(message.encoded().as_ptr(), output.as_ptr());
        assert_eq!(message.body().as_ptr(), output[4..].as_ptr());
    }
}

#[test]
fn uint24_length_is_network_order_and_not_truncated_to_u16() {
    let policy = policy(&[Tls13HandshakeType::Certificate], 0x010203);
    let body = vec![0x5a; 0x010203];
    let mut output = vec![0xa5; TLS13_HANDSHAKE_HEADER_LEN + body.len()];
    let used = encode_tls13_handshake(&policy, Tls13HandshakeType::Certificate, &body, &mut output)
        .expect("message encodes");

    assert_eq!(&output[..4], &[11, 0x01, 0x02, 0x03]);
    assert_eq!(used, output.len());
    assert_eq!(
        decode_tls13_handshake(&policy, &output)
            .expect("message decodes")
            .body(),
        body
    );
}

#[test]
fn prefix_decode_returns_exact_message_and_stream_tail() {
    let policy = policy(&[Tls13HandshakeType::Finished], 16);
    let bytes = [20, 0, 0, 3, 1, 2, 3, 24, 0, 0];
    let (message, remainder) =
        decode_tls13_handshake_prefix(&policy, &bytes).expect("prefix decodes");

    assert_eq!(message.encoded(), &bytes[..7]);
    assert_eq!(message.body(), &[1, 2, 3]);
    assert_eq!(remainder, &bytes[7..]);
    assert_eq!(
        decode_tls13_handshake(&policy, &bytes)
            .expect_err("exact mode rejects tail")
            .kind(),
        HandshakeErrorKind::TrailingData
    );
}

#[test]
fn malformed_headers_types_lengths_and_bodies_fail_closed() {
    let policy = policy(&[Tls13HandshakeType::Finished], 3);
    let cases: &[(&[u8], HandshakeErrorKind)] = &[
        (&[20, 0, 0], HandshakeErrorKind::TruncatedHeader),
        (&[254, 0, 0, 0], HandshakeErrorKind::UnknownHandshakeType),
        (&[25, 0, 0, 0], HandshakeErrorKind::UnknownHandshakeType),
        (&[24, 0, 0, 0], HandshakeErrorKind::DisallowedHandshakeType),
        (&[20, 0, 0, 4], HandshakeErrorKind::BodyTooLong),
        (&[20, 0, 0, 2, 0], HandshakeErrorKind::TruncatedBody),
    ];

    for (bytes, expected) in cases {
        assert_eq!(
            decode_tls13_handshake(&policy, bytes)
                .expect_err("malformed message must fail")
                .kind(),
            *expected
        );
    }
}

#[test]
fn encoder_failure_is_atomic_for_every_preflight_rejection() {
    let policy = policy(&[Tls13HandshakeType::Finished], 3);
    let cases = [
        (
            Tls13HandshakeType::KeyUpdate,
            &b""[..],
            8,
            HandshakeErrorKind::DisallowedHandshakeType,
        ),
        (
            Tls13HandshakeType::Finished,
            &b"four"[..],
            8,
            HandshakeErrorKind::BodyTooLong,
        ),
        (
            Tls13HandshakeType::Finished,
            &b"abc"[..],
            6,
            HandshakeErrorKind::OutputTooSmall,
        ),
    ];

    for (handshake_type, body, output_length, expected) in cases {
        let mut output = [0xa5; 8];
        assert_eq!(
            encode_tls13_handshake(&policy, handshake_type, body, &mut output[..output_length],)
                .expect_err("invalid encoding must fail")
                .kind(),
            expected
        );
        assert_eq!(output, [0xa5; 8]);
    }
}

#[test]
fn errors_are_payload_free_stable_categories() {
    assert_eq!(size_of::<HandshakeError>(), size_of::<HandshakeErrorKind>());
    let error = decode_tls13_handshake(&policy(&[Tls13HandshakeType::Finished], 8), &[20, 0, 0])
        .expect_err("truncated header must fail");
    assert_eq!(error.kind(), HandshakeErrorKind::TruncatedHeader);
    assert_eq!(
        error.to_string(),
        "TLS 1.3 handshake framing error TruncatedHeader"
    );
}
