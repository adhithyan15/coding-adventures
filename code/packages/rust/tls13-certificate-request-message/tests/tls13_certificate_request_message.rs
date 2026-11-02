use tls13_certificate_request_message::{
    decode_certificate_request, CertificateRequestErrorKind, CertificateRequestLimits,
};
use tls13_extension_block::{ExtensionBlockError, ExtensionBlockLimits, ExtensionType};

fn limits(context: usize, payload: usize, count: usize) -> CertificateRequestLimits {
    CertificateRequestLimits::new(
        context,
        ExtensionBlockLimits::new(payload, count).expect("extension limits are valid"),
    )
    .expect("request limits are valid")
}

#[test]
fn decodes_empty_main_handshake_context_and_borrowed_extensions() {
    let encoded = [0, 0, 8, 0, 13, 0, 4, 0, 2, 4, 3];
    let request =
        decode_certificate_request(&encoded, limits(0, 16, 2)).expect("request body decodes");

    assert_eq!(request.encoded_body(), &encoded);
    assert!(request.context().is_empty());
    assert_eq!(request.extensions().len(), 1);
    let signature_algorithms = request
        .extensions()
        .find(ExtensionType::from_wire_value(13))
        .expect("signature algorithms remains available to later policy");
    assert_eq!(signature_algorithms.data(), &[0, 2, 4, 3]);
}

#[test]
fn retains_nonempty_context_and_unknown_extension_as_structural_data() {
    let encoded = [3, 0xaa, 0xbb, 0xcc, 0, 5, 0xff, 1, 0, 1, 0x99];
    let request = decode_certificate_request(&encoded, limits(3, 8, 1))
        .expect("post-handshake-shaped request decodes structurally");

    assert_eq!(request.context(), &[0xaa, 0xbb, 0xcc]);
    assert_eq!(request.extensions().entries()[0].data(), &[0x99]);
    assert!(format!("{request:?}").contains("context_length: 3"));
    assert!(!format!("{request:?}").contains("170"));
}

#[test]
fn validates_context_limits_before_exposing_bytes() {
    assert_eq!(
        CertificateRequestLimits::new(
            256,
            ExtensionBlockLimits::new(8, 1).expect("extension limits are valid"),
        )
        .expect_err("context limit exceeds wire width")
        .kind(),
        CertificateRequestErrorKind::InvalidLimits
    );

    let error = decode_certificate_request(&[1, 0xaa, 0, 0], limits(0, 8, 1))
        .expect_err("context exceeds caller limit");
    assert_eq!(error.kind(), CertificateRequestErrorKind::ContextTooLong);
}

#[test]
fn rejects_missing_and_truncated_context_framing() {
    assert_eq!(
        decode_certificate_request(&[], limits(0, 8, 1))
            .expect_err("missing length fails")
            .kind(),
        CertificateRequestErrorKind::MissingContextLength
    );
    assert_eq!(
        decode_certificate_request(&[2, 0xaa], limits(2, 8, 1))
            .expect_err("truncated context fails")
            .kind(),
        CertificateRequestErrorKind::TruncatedContext
    );
}

#[test]
fn delegates_exact_remaining_bytes_to_extension_block_decoder() {
    let missing = decode_certificate_request(&[0, 0], limits(0, 8, 1))
        .expect_err("one-byte extension prefix fails");
    assert_eq!(
        missing.kind(),
        CertificateRequestErrorKind::Extensions(ExtensionBlockError::MissingBlockLength)
    );

    let trailing = decode_certificate_request(&[0, 0, 0, 0], limits(0, 8, 1))
        .expect_err("trailing byte fails");
    assert_eq!(
        trailing.kind(),
        CertificateRequestErrorKind::Extensions(ExtensionBlockError::BlockLengthMismatch)
    );
}

#[test]
fn rejects_structurally_empty_extension_vector() {
    let error = decode_certificate_request(&[0, 0, 0], limits(0, 8, 1))
        .expect_err("CertificateRequest requires at least one extension");
    assert_eq!(error.kind(), CertificateRequestErrorKind::EmptyExtensions);
    assert_eq!(
        error.to_string(),
        "TLS 1.3 CertificateRequest error EmptyExtensions"
    );
}

#[test]
fn diagnostics_retain_no_context_or_extension_payload() {
    let error = decode_certificate_request(&[1, 0xde, 0], limits(1, 8, 1))
        .expect_err("invalid extension block fails");
    assert_eq!(
        error.to_string(),
        "TLS 1.3 CertificateRequest error Extensions(MissingBlockLength)"
    );
    let debug = format!("{error:?}");
    assert!(!debug.contains("222"));
}
