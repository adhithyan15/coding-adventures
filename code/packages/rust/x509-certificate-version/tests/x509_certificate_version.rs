use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_certificate_version::{
    decode_explicit_x509_certificate_version, X509CertificateVersion, X509CertificateVersionError,
    X509CertificateVersionErrorKind,
};

fn decode(encoded: &[u8]) -> Result<X509CertificateVersion, X509CertificateVersionError> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder
        .decode_exact(encoded)
        .expect("valid outer DER framing");
    decode_explicit_x509_certificate_version(&mut decoder, root)
}

#[test]
fn omitted_field_has_the_v1_default() {
    let version = X509CertificateVersion::default();
    assert_eq!(version, X509CertificateVersion::V1);
    assert_eq!(version.as_u8(), 0);
}

#[test]
fn decodes_explicit_v2_and_v3() {
    assert_eq!(
        decode(&[0xa0, 0x03, 0x02, 0x01, 0x01]).unwrap(),
        X509CertificateVersion::V2
    );
    assert_eq!(
        decode(&[0xa0, 0x03, 0x02, 0x01, 0x02]).unwrap(),
        X509CertificateVersion::V3
    );
    assert_eq!(X509CertificateVersion::V2.as_u8(), 1);
    assert_eq!(X509CertificateVersion::V3.as_u8(), 2);
}

#[test]
fn rejects_an_explicitly_encoded_default() {
    let error = decode(&[0xa0, 0x03, 0x02, 0x01, 0x00]).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificateVersionErrorKind::ExplicitDefault
    );
    assert_eq!(error.offset(), 4);
}

#[test]
fn rejects_unsupported_positive_versions() {
    for value in [3, 127] {
        let error = decode(&[0xa0, 0x03, 0x02, 0x01, value]).unwrap_err();
        assert_eq!(
            error.kind(),
            X509CertificateVersionErrorKind::UnsupportedVersion
        );
        assert_eq!(error.offset(), 4);
    }
}

#[test]
fn rejects_negative_and_oversized_version_integers() {
    let negative = decode(&[0xa0, 0x03, 0x02, 0x01, 0xff]).unwrap_err();
    assert_eq!(
        negative.kind(),
        X509CertificateVersionErrorKind::InvalidInteger(Asn1ErrorKind::NegativeInteger)
    );
    assert_eq!(negative.offset(), 4);

    let mut oversized = vec![0xa0, 0x0b, 0x02, 0x09, 0x01];
    oversized.extend([0; 8]);
    let error = decode(&oversized).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificateVersionErrorKind::InvalidInteger(Asn1ErrorKind::IntegerOverflow)
    );
    assert_eq!(error.offset(), 4);
}

#[test]
fn requires_constructed_context_specific_tag_zero() {
    for encoded in [
        &[0x80, 0x03, 0x02, 0x01, 0x02][..],
        &[0xa1, 0x03, 0x02, 0x01, 0x02][..],
        &[0x30, 0x03, 0x02, 0x01, 0x02][..],
    ] {
        let error = decode(encoded).unwrap_err();
        assert_eq!(
            error.kind(),
            X509CertificateVersionErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
        assert_eq!(error.offset(), 0);
    }
}

#[test]
fn explicit_wrapper_must_hold_exactly_one_child() {
    for encoded in [
        &[0xa0, 0x00][..],
        &[0xa0, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02][..],
        &[0xa0, 0x02, 0x02, 0x01][..],
    ] {
        let error = decode(encoded).unwrap_err();
        assert!(matches!(
            error.kind(),
            X509CertificateVersionErrorKind::Structure(Asn1ErrorKind::Framing(_))
        ));
        assert!(error.offset() >= 2);
    }
}

#[test]
fn child_must_be_a_canonical_integer() {
    let wrong_tag = decode(&[0xa0, 0x03, 0x04, 0x01, 0x02]).unwrap_err();
    assert_eq!(
        wrong_tag.kind(),
        X509CertificateVersionErrorKind::InvalidInteger(Asn1ErrorKind::UnexpectedTag)
    );
    assert_eq!(wrong_tag.offset(), 2);

    let non_minimal = decode(&[0xa0, 0x04, 0x02, 0x02, 0x00, 0x02]).unwrap_err();
    assert_eq!(
        non_minimal.kind(),
        X509CertificateVersionErrorKind::InvalidInteger(Asn1ErrorKind::NonMinimalInteger)
    );
    assert_eq!(non_minimal.offset(), 4);
}

#[test]
fn shared_depth_and_element_limits_are_enforced() {
    let encoded = [0xa0, 0x03, 0x02, 0x01, 0x02];

    let depth_limits = Asn1Limits {
        max_depth: 1,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(depth_limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    let error = decode_explicit_x509_certificate_version(&mut decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificateVersionErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    );

    let element_limits = Asn1Limits {
        max_total_elements: 1,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(element_limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    let error = decode_explicit_x509_certificate_version(&mut decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificateVersionErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), 2);
}

#[test]
fn errors_are_redacted() {
    let error = decode(&[0xa0, 0x03, 0x02, 0x01, 0x7f]).unwrap_err();
    let rendered = error.to_string();
    assert!(!rendered.contains("127"));
    assert!(!rendered.contains("7f"));
    assert!(rendered.contains("byte 4"));
}
