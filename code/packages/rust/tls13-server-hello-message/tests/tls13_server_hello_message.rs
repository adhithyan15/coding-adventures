use tls13_extension_block::ExtensionBlockError;
use tls13_server_hello_message::{
    decode_tls13_server_hello, Tls13ServerHelloErrorKind, Tls13ServerHelloField,
    Tls13ServerHelloLimits, TLS13_SERVER_HELLO_MAX_MESSAGE_LENGTH,
    TLS13_SERVER_HELLO_MAX_SESSION_ID_ECHO_LENGTH, TLS13_SERVER_HELLO_MIN_MESSAGE_LENGTH,
};

fn limits(
    message_bytes: usize,
    extension_payload_bytes: usize,
    extension_count: usize,
) -> Tls13ServerHelloLimits {
    Tls13ServerHelloLimits::new(message_bytes, extension_payload_bytes, extension_count).unwrap()
}

fn encoded_server_hello(session_id: &[u8], random: [u8; 32]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&[0x03, 0x03]);
    encoded.extend_from_slice(&random);
    encoded.push(session_id.len() as u8);
    encoded.extend_from_slice(session_id);
    encoded.extend_from_slice(&[0x13, 0x01]);
    encoded.push(0);
    encoded.extend_from_slice(&[0, 6, 0, 43, 0, 2, 0x03, 0x04]);
    encoded
}

#[test]
fn decodes_exact_borrowed_fields_and_extensions() {
    let random = [0x5a; 32];
    let encoded = encoded_server_hello(&[0xaa, 0xbb], random);
    let decoded = decode_tls13_server_hello(&encoded, limits(128, 64, 4)).unwrap();

    assert_eq!(decoded.encoded().as_ptr(), encoded.as_ptr());
    assert_eq!(decoded.random(), &random);
    assert_eq!(decoded.session_id_echo(), &[0xaa, 0xbb]);
    assert_eq!(decoded.cipher_suite().wire_value(), 0x1301);
    assert_eq!(decoded.extensions().len(), 1);
    assert_eq!(
        decoded.extensions().entries()[0]
            .extension_type()
            .wire_value(),
        43
    );
    assert_eq!(decoded.extensions().entries()[0].data(), &[0x03, 0x04]);
}

#[test]
fn accepts_zero_and_maximum_session_id_echo_boundaries() {
    let empty = encoded_server_hello(&[], [0; 32]);
    assert!(
        decode_tls13_server_hello(&empty, limits(TLS13_SERVER_HELLO_MIN_MESSAGE_LENGTH, 6, 1))
            .is_ok()
    );

    let session_id = [0x44; TLS13_SERVER_HELLO_MAX_SESSION_ID_ECHO_LENGTH];
    let maximum = encoded_server_hello(&session_id, [1; 32]);
    let decoded = decode_tls13_server_hello(&maximum, limits(maximum.len(), 6, 1)).unwrap();
    assert_eq!(decoded.session_id_echo(), session_id);
}

#[test]
fn rejects_fixed_legacy_field_violations_and_long_session_id() {
    let mut wrong_version = encoded_server_hello(&[], [0; 32]);
    wrong_version[1] = 0x04;
    let error = decode_tls13_server_hello(&wrong_version, limits(128, 64, 4)).unwrap_err();
    assert_eq!(
        error.kind(),
        Tls13ServerHelloErrorKind::InvalidLegacyVersion
    );
    assert_eq!(error.offset(), 0);

    let mut long_session = encoded_server_hello(&[], [0; 32]);
    long_session[34] = 33;
    let error = decode_tls13_server_hello(&long_session, limits(128, 64, 4)).unwrap_err();
    assert_eq!(
        error.kind(),
        Tls13ServerHelloErrorKind::SessionIdEchoTooLong
    );
    assert_eq!(error.offset(), 34);

    let mut compression = encoded_server_hello(&[], [0; 32]);
    compression[37] = 1;
    let error = decode_tls13_server_hello(&compression, limits(128, 64, 4)).unwrap_err();
    assert_eq!(
        error.kind(),
        Tls13ServerHelloErrorKind::InvalidLegacyCompressionMethod
    );
    assert_eq!(error.offset(), 37);
}

#[test]
fn reports_truncated_fixed_fields_at_local_offsets() {
    let encoded = encoded_server_hello(&[0xaa, 0xbb], [0; 32]);
    let cases = [
        (0, Tls13ServerHelloField::LegacyVersion, 0),
        (2, Tls13ServerHelloField::Random, 2),
        (34, Tls13ServerHelloField::SessionIdEchoLength, 34),
        (36, Tls13ServerHelloField::SessionIdEcho, 35),
        (37, Tls13ServerHelloField::CipherSuite, 37),
        (39, Tls13ServerHelloField::LegacyCompressionMethod, 39),
    ];

    for (length, field, offset) in cases {
        let error = decode_tls13_server_hello(&encoded[..length], limits(128, 64, 4)).unwrap_err();
        assert_eq!(error.kind(), Tls13ServerHelloErrorKind::Truncated(field));
        assert_eq!(error.offset(), offset);
    }
}

#[test]
fn delegates_exact_extension_framing_and_duplicate_rejection() {
    let mut missing_length = encoded_server_hello(&[], [0; 32]);
    missing_length.truncate(38);
    let error = decode_tls13_server_hello(&missing_length, limits(128, 64, 4)).unwrap_err();
    assert_eq!(
        error.kind(),
        Tls13ServerHelloErrorKind::InvalidExtensions(ExtensionBlockError::MissingBlockLength)
    );
    assert_eq!(error.offset(), 38);

    let mut trailing = encoded_server_hello(&[], [0; 32]);
    trailing.push(0xff);
    let error = decode_tls13_server_hello(&trailing, limits(128, 64, 4)).unwrap_err();
    assert_eq!(
        error.kind(),
        Tls13ServerHelloErrorKind::InvalidExtensions(ExtensionBlockError::BlockLengthMismatch)
    );

    let mut duplicate = encoded_server_hello(&[], [0; 32]);
    duplicate.truncate(38);
    duplicate.extend_from_slice(&[0, 12, 0, 43, 0, 2, 3, 4, 0, 43, 0, 2, 3, 4]);
    let error = decode_tls13_server_hello(&duplicate, limits(128, 64, 4)).unwrap_err();
    assert_eq!(
        error.kind(),
        Tls13ServerHelloErrorKind::InvalidExtensions(ExtensionBlockError::DuplicateExtension)
    );
}

#[test]
fn enforces_message_extension_and_count_limits() {
    assert_eq!(
        Tls13ServerHelloLimits::new(TLS13_SERVER_HELLO_MIN_MESSAGE_LENGTH - 1, 6, 1)
            .unwrap_err()
            .kind(),
        Tls13ServerHelloErrorKind::InvalidLimits
    );
    assert_eq!(
        Tls13ServerHelloLimits::new(TLS13_SERVER_HELLO_MAX_MESSAGE_LENGTH + 1, 6, 1)
            .unwrap_err()
            .kind(),
        Tls13ServerHelloErrorKind::InvalidLimits
    );
    assert_eq!(
        Tls13ServerHelloLimits::new(128, 5, 1).unwrap_err().kind(),
        Tls13ServerHelloErrorKind::InvalidLimits
    );
    assert_eq!(
        Tls13ServerHelloLimits::new(128, 6, 0).unwrap_err().kind(),
        Tls13ServerHelloErrorKind::InvalidLimits
    );

    let encoded = encoded_server_hello(&[0xaa], [0; 32]);
    let error = decode_tls13_server_hello(
        &encoded,
        limits(TLS13_SERVER_HELLO_MIN_MESSAGE_LENGTH, 6, 1),
    )
    .unwrap_err();
    assert_eq!(error.kind(), Tls13ServerHelloErrorKind::MessageTooLong);

    let mut short_extensions = encoded_server_hello(&[], [0; 32]);
    short_extensions.truncate(38);
    short_extensions.extend_from_slice(&[0, 4, 0, 43, 0, 0]);
    let error = decode_tls13_server_hello(&short_extensions, limits(128, 64, 4)).unwrap_err();
    assert_eq!(error.kind(), Tls13ServerHelloErrorKind::ExtensionsTooShort);
}

#[test]
fn debug_output_redacts_random_session_and_extension_bytes() {
    let random = *b"RANDOM-BYTES-MUST-STAY-OPAQUE!!!";
    let encoded = encoded_server_hello(b"SESSION-SECRET", random);
    let decoded = decode_tls13_server_hello(&encoded, limits(128, 64, 4)).unwrap();
    let debug = format!("{decoded:?}");

    assert!(debug.contains("[OPAQUE]"));
    assert!(!debug.contains("RANDOM-BYTES"));
    assert!(!debug.contains("SESSION-SECRET"));
    assert!(!debug.contains("0304"));

    let error = decode_tls13_server_hello(&encoded[..2], limits(128, 64, 4)).unwrap_err();
    assert!(!format!("{error:?}").contains("RANDOM-BYTES"));
    assert!(!error.to_string().contains("SESSION-SECRET"));
}
