use tls13_client_hello_message::{
    decode_client_hello, ClientHelloError, ClientHelloLimits, MAX_CIPHER_SUITE_COUNT,
};
use tls13_extension_block::{ExtensionBlockError, ExtensionBlockLimits, ExtensionType};

fn limits(suites: usize, extension_bytes: usize, extensions: usize) -> ClientHelloLimits {
    ClientHelloLimits::new(
        suites,
        ExtensionBlockLimits::new(extension_bytes, extensions).expect("valid extension limits"),
    )
    .expect("valid ClientHello limits")
}

fn valid_body() -> Vec<u8> {
    let mut body = vec![0x03, 0x03];
    body.extend_from_slice(&[0x11; 32]);
    body.push(0); // empty legacy session ID
    body.extend_from_slice(&[0, 4, 0x13, 0x01, 0x7a, 0x7a]);
    body.extend_from_slice(&[1, 0]); // single null compression method
    body.extend_from_slice(&[0, 8, 0, 43, 0, 4, 0x03, 0x04, 0x03, 0x03]);
    body
}

#[test]
fn decodes_and_retains_the_exact_structural_client_hello() {
    let body = valid_body();
    let message = decode_client_hello(&body, limits(8, 64, 8)).expect("valid body");

    assert_eq!(message.body(), body.as_slice());
    assert!(core::ptr::eq(message.body().as_ptr(), body.as_ptr()));
    assert_eq!(message.random(), &[0x11; 32]);
    assert!(message.legacy_session_id().is_empty());
    assert_eq!(message.cipher_suite_count(), 2);
    assert_eq!(message.cipher_suite(0), Some(0x1301));
    assert_eq!(message.cipher_suite(1), Some(0x7a7a));
    assert_eq!(message.cipher_suite(2), None);
    assert_eq!(
        message.cipher_suites().collect::<Vec<_>>(),
        vec![0x1301, 0x7a7a]
    );
    assert_eq!(message.extensions().len(), 1);
    assert_eq!(message.extensions().payload_length(), 8);
    assert_eq!(
        message
            .extensions()
            .find(ExtensionType::from_wire_value(43))
            .expect("supported_versions-shaped entry")
            .data(),
        &[0x03, 0x04, 0x03, 0x03]
    );
}

#[test]
fn accepts_the_maximum_legacy_session_id_length() {
    let original = valid_body();
    let mut body = original[..34].to_vec();
    body.push(32);
    body.extend_from_slice(&[0x22; 32]);
    body.extend_from_slice(&original[35..]);

    let message = decode_client_hello(&body, limits(8, 64, 8)).expect("valid body");
    assert_eq!(message.legacy_session_id(), &[0x22; 32]);
}

#[test]
fn rejects_fixed_field_and_session_id_failures() {
    assert_eq!(
        decode_client_hello(&[], limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::MissingFixedFields
    );

    let mut body = valid_body();
    body[1] = 0x04;
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::InvalidLegacyVersion
    );

    let mut body = valid_body();
    body[34] = 33;
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::LegacySessionIdTooLong
    );

    let mut body = valid_body()[..35].to_vec();
    body[34] = 2;
    body.push(0xaa);
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::TruncatedLegacySessionId
    );
}

#[test]
fn rejects_cipher_suite_framing_and_resource_failures() {
    let body = valid_body()[..35].to_vec();
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::MissingCipherSuiteLength
    );

    let mut body = valid_body();
    body[35..37].copy_from_slice(&[0, 0]);
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::EmptyCipherSuiteList
    );

    let mut body = valid_body();
    body[35..37].copy_from_slice(&[0, 3]);
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::OddCipherSuiteLength
    );

    assert_eq!(
        decode_client_hello(&valid_body(), limits(1, 64, 8)).unwrap_err(),
        ClientHelloError::TooManyCipherSuites
    );

    let mut body = valid_body()[..39].to_vec();
    body[35..37].copy_from_slice(&[0, 4]);
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::TruncatedCipherSuites
    );
}

#[test]
fn requires_the_single_null_legacy_compression_method() {
    let body = valid_body()[..41].to_vec();
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::MissingCompressionMethodsLength
    );

    let mut body = valid_body()[..42].to_vec();
    body[41] = 1;
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::TruncatedCompressionMethods
    );

    let mut body = valid_body();
    body[42] = 1;
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::InvalidCompressionMethods
    );

    let mut body = valid_body();
    body[41] = 2;
    body.insert(43, 0);
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::InvalidCompressionMethods
    );
}

#[test]
fn preserves_extension_framing_and_resource_failures() {
    let body = valid_body()[..43].to_vec();
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::Extensions(ExtensionBlockError::MissingBlockLength)
    );

    let mut body = valid_body();
    body.truncate(45);
    body[43..45].copy_from_slice(&[0, 0]);
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::ExtensionVectorTooShort
    );

    assert_eq!(
        decode_client_hello(&valid_body(), limits(8, 7, 8)).unwrap_err(),
        ClientHelloError::Extensions(ExtensionBlockError::BlockTooLong)
    );

    let mut body = valid_body();
    body.push(0);
    assert_eq!(
        decode_client_hello(&body, limits(8, 64, 8)).unwrap_err(),
        ClientHelloError::Extensions(ExtensionBlockError::BlockLengthMismatch)
    );
}

#[test]
fn validates_limits_and_redacts_message_and_iterator_diagnostics() {
    let extension_limits = ExtensionBlockLimits::new(64, 8).expect("valid extension limits");
    assert_eq!(
        ClientHelloLimits::new(0, extension_limits),
        Err(ClientHelloError::InvalidCipherSuiteLimit)
    );
    assert_eq!(
        ClientHelloLimits::new(MAX_CIPHER_SUITE_COUNT + 1, extension_limits),
        Err(ClientHelloError::InvalidCipherSuiteLimit)
    );
    let maximum =
        ClientHelloLimits::new(MAX_CIPHER_SUITE_COUNT, extension_limits).expect("protocol maximum");
    assert_eq!(maximum.max_cipher_suite_count(), MAX_CIPHER_SUITE_COUNT);
    assert_eq!(maximum.extension_limits(), extension_limits);

    let body = valid_body();
    let message = decode_client_hello(&body, limits(8, 64, 8)).expect("valid body");
    let debug = format!("{message:?}");
    assert!(debug.contains("cipher_suite_count: 2"));
    assert!(debug.contains("[OPAQUE]"));
    for disclosed in ["17", "0x11", "4865", "31354"] {
        assert!(!debug.contains(disclosed));
    }

    let mut suites = message.cipher_suites();
    let fresh_debug = format!("{suites:?}");
    assert!(fresh_debug.contains("remaining_suite_count: 2"));
    assert!(fresh_debug.contains("[OPAQUE]"));
    assert!(!fresh_debug.contains("19"));
    assert!(!fresh_debug.contains("122"));
    assert_eq!(suites.next(), Some(0x1301));
    let partial_debug = format!("{suites:?}");
    assert!(partial_debug.contains("remaining_suite_count: 1"));
    assert!(!partial_debug.contains("122"));
}

#[test]
fn errors_have_stable_payload_free_messages() {
    let cases = [
        (
            ClientHelloError::InvalidCipherSuiteLimit,
            "invalid ClientHello cipher-suite limit",
        ),
        (
            ClientHelloError::MissingFixedFields,
            "missing ClientHello fixed fields",
        ),
        (
            ClientHelloError::InvalidLegacyVersion,
            "invalid ClientHello legacy version",
        ),
        (
            ClientHelloError::LegacySessionIdTooLong,
            "ClientHello legacy session ID is too long",
        ),
        (
            ClientHelloError::TruncatedLegacySessionId,
            "truncated ClientHello legacy session ID",
        ),
        (
            ClientHelloError::MissingCipherSuiteLength,
            "missing ClientHello cipher-suite length",
        ),
        (
            ClientHelloError::EmptyCipherSuiteList,
            "empty ClientHello cipher-suite list",
        ),
        (
            ClientHelloError::OddCipherSuiteLength,
            "odd ClientHello cipher-suite length",
        ),
        (
            ClientHelloError::TooManyCipherSuites,
            "ClientHello cipher-suite list exceeds caller limit",
        ),
        (
            ClientHelloError::TruncatedCipherSuites,
            "truncated ClientHello cipher-suite list",
        ),
        (
            ClientHelloError::MissingCompressionMethodsLength,
            "missing ClientHello compression-method length",
        ),
        (
            ClientHelloError::TruncatedCompressionMethods,
            "truncated ClientHello compression methods",
        ),
        (
            ClientHelloError::InvalidCompressionMethods,
            "invalid ClientHello compression methods",
        ),
        (
            ClientHelloError::ExtensionVectorTooShort,
            "ClientHello extension vector is too short",
        ),
        (
            ClientHelloError::Extensions(ExtensionBlockError::DuplicateExtension),
            "invalid ClientHello extension vector",
        ),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}
