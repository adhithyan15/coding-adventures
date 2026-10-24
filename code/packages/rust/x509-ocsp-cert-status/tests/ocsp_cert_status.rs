use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_ocsp_cert_status::{
    decode_x509_ocsp_cert_status, X509OcspCertStatus, X509OcspCertStatusErrorKind,
    X509OcspRevocationReason,
};
use x509_time::X509TimeErrorKind;

const REVOCATION_TIME: &[u8] = b"20261024123456Z";

fn generalized_time(contents: &[u8]) -> Vec<u8> {
    let mut encoded = vec![0x18, contents.len() as u8];
    encoded.extend_from_slice(contents);
    encoded
}

fn reason(value: &[u8]) -> Vec<u8> {
    let mut enumerated = vec![0x0a, value.len() as u8];
    enumerated.extend_from_slice(value);
    let mut wrapper = vec![0xa0, enumerated.len() as u8];
    wrapper.extend_from_slice(&enumerated);
    wrapper
}

fn revoked(fields: &[Vec<u8>]) -> Vec<u8> {
    let value_length: usize = fields.iter().map(Vec::len).sum();
    assert!(value_length < 128);
    let mut encoded = vec![0xa1, value_length as u8];
    for field in fields {
        encoded.extend_from_slice(field);
    }
    encoded
}

fn decode(
    encoded: &[u8],
) -> Result<X509OcspCertStatus, x509_ocsp_cert_status::X509OcspCertStatusError> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_ocsp_cert_status(&mut decoder, element)
}

#[test]
fn decodes_the_three_exact_certificate_status_choices() {
    assert_eq!(decode(&[0x80, 0x00]).unwrap(), X509OcspCertStatus::Good);
    assert_eq!(decode(&[0x82, 0x00]).unwrap(), X509OcspCertStatus::Unknown);

    let encoded = revoked(&[generalized_time(REVOCATION_TIME)]);
    let X509OcspCertStatus::Revoked(info) = decode(&encoded).unwrap() else {
        panic!("expected revoked status");
    };
    assert_eq!(info.revocation_time().year(), 2026);
    assert_eq!(info.revocation_time().month(), 10);
    assert_eq!(info.revocation_time().day(), 24);
    assert_eq!(info.revocation_time().hour(), 12);
    assert_eq!(info.revocation_time().minute(), 34);
    assert_eq!(info.revocation_time().second(), 56);
    assert_eq!(info.revocation_reason(), None);
}

#[test]
fn good_and_unknown_are_exact_implicit_empty_nulls() {
    for (encoded, expected) in [
        (
            vec![0x80, 0x01, 0x00],
            X509OcspCertStatusErrorKind::Structure(Asn1ErrorKind::NonEmptyNull),
        ),
        (
            vec![0x82, 0x01, 0x00],
            X509OcspCertStatusErrorKind::Structure(Asn1ErrorKind::NonEmptyNull),
        ),
        (
            vec![0xa0, 0x00],
            X509OcspCertStatusErrorKind::Structure(Asn1ErrorKind::UnexpectedTag),
        ),
        (
            vec![0xa2, 0x00],
            X509OcspCertStatusErrorKind::Structure(Asn1ErrorKind::UnexpectedTag),
        ),
    ] {
        assert_eq!(decode(&encoded).unwrap_err().kind(), expected);
    }
}

#[test]
fn rejects_every_unrecognized_choice_tag_or_shape() {
    for encoded in [
        vec![0x05, 0x00],
        vec![0x81, 0x00],
        vec![0x83, 0x00],
        vec![
            0x81, 0x11, 0x18, 0x0f, b'2', b'0', b'2', b'6', b'1', b'0', b'2', b'4', b'1', b'2',
            b'3', b'4', b'5', b'6', b'Z',
        ],
    ] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509OcspCertStatusErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }
}

#[test]
fn revoked_requires_one_valid_ocsp_generalized_time() {
    assert_eq!(
        decode(&[0xa1, 0x00]).unwrap_err().kind(),
        X509OcspCertStatusErrorKind::MissingRevocationTime
    );

    for (time, expected) in [
        (
            vec![
                0x17, 0x0d, b'2', b'6', b'1', b'0', b'2', b'4', b'1', b'2', b'3', b'4', b'5', b'6',
                b'Z',
            ],
            X509TimeErrorKind::UnexpectedTag,
        ),
        (
            generalized_time(b"20261324123456Z"),
            X509TimeErrorKind::InvalidMonth,
        ),
        (
            generalized_time(b"20261024123456+"),
            X509TimeErrorKind::MissingZulu,
        ),
    ] {
        assert_eq!(
            decode(&revoked(&[time])).unwrap_err().kind(),
            X509OcspCertStatusErrorKind::InvalidRevocationTime(expected)
        );
    }
}

#[test]
fn maps_every_rfc_defined_crl_reason() {
    let cases = [
        (0, X509OcspRevocationReason::Unspecified),
        (1, X509OcspRevocationReason::KeyCompromise),
        (2, X509OcspRevocationReason::CaCompromise),
        (3, X509OcspRevocationReason::AffiliationChanged),
        (4, X509OcspRevocationReason::Superseded),
        (5, X509OcspRevocationReason::CessationOfOperation),
        (6, X509OcspRevocationReason::CertificateHold),
        (8, X509OcspRevocationReason::RemoveFromCrl),
        (9, X509OcspRevocationReason::PrivilegeWithdrawn),
        (10, X509OcspRevocationReason::AaCompromise),
    ];

    for (wire, expected) in cases {
        let encoded = revoked(&[generalized_time(REVOCATION_TIME), reason(&[wire])]);
        let X509OcspCertStatus::Revoked(info) = decode(&encoded).unwrap() else {
            panic!("expected revoked status");
        };
        assert_eq!(info.revocation_reason(), Some(expected));
        assert_eq!(expected.as_u8(), wire);
    }
}

#[test]
fn rejects_noncanonical_negative_unassigned_and_out_of_range_reasons() {
    for (value, expected) in [
        (
            vec![],
            X509OcspCertStatusErrorKind::InvalidRevocationReason(Asn1ErrorKind::EmptyInteger),
        ),
        (
            vec![0x00, 0x01],
            X509OcspCertStatusErrorKind::InvalidRevocationReason(Asn1ErrorKind::NonMinimalInteger),
        ),
        (
            vec![0xff],
            X509OcspCertStatusErrorKind::InvalidRevocationReason(Asn1ErrorKind::NegativeInteger),
        ),
        (
            vec![0x07],
            X509OcspCertStatusErrorKind::UnsupportedRevocationReason,
        ),
        (
            vec![0x0b],
            X509OcspCertStatusErrorKind::UnsupportedRevocationReason,
        ),
        (
            vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            X509OcspCertStatusErrorKind::InvalidRevocationReason(Asn1ErrorKind::IntegerOverflow),
        ),
    ] {
        let encoded = revoked(&[generalized_time(REVOCATION_TIME), reason(&value)]);
        assert_eq!(decode(&encoded).unwrap_err().kind(), expected);
    }
}

#[test]
fn reason_wrapper_and_trailing_fields_fail_closed() {
    let time = generalized_time(REVOCATION_TIME);
    for (field, expected) in [
        (
            vec![0x0a, 0x01, 0x01],
            X509OcspCertStatusErrorKind::InvalidRevocationReason(Asn1ErrorKind::UnexpectedTag),
        ),
        (
            vec![0xa0, 0x00],
            X509OcspCertStatusErrorKind::InvalidRevocationReason(Asn1ErrorKind::Framing(
                der_tlv::DerErrorKind::EmptyInput,
            )),
        ),
        (
            vec![0xa0, 0x05, 0x0a, 0x01, 0x01, 0x05, 0x00],
            X509OcspCertStatusErrorKind::InvalidRevocationReason(Asn1ErrorKind::Framing(
                der_tlv::DerErrorKind::TrailingData,
            )),
        ),
    ] {
        assert_eq!(
            decode(&revoked(&[time.clone(), field])).unwrap_err().kind(),
            expected
        );
    }

    let encoded = revoked(&[time, reason(&[0x01]), vec![0x05, 0x00]]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509OcspCertStatusErrorKind::TrailingElement
    );
}

#[test]
fn malformed_children_and_shared_budgets_fail_closed() {
    let malformed = vec![0xa1, 0x02, 0x18, 0x01];
    let error = decode(&malformed).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509OcspCertStatusErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), malformed.len());

    let encoded = revoked(&[generalized_time(REVOCATION_TIME), reason(&[0x01])]);
    let limits = Asn1Limits {
        max_depth: 2,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_ocsp_cert_status(&mut decoder, root)
            .unwrap_err()
            .kind(),
        X509OcspCertStatusErrorKind::InvalidRevocationReason(Asn1ErrorKind::DepthLimitExceeded,)
    );

    let limits = Asn1Limits {
        max_total_elements: 3,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_ocsp_cert_status(&mut decoder, root)
            .unwrap_err()
            .kind(),
        X509OcspCertStatusErrorKind::InvalidRevocationReason(Asn1ErrorKind::ElementLimitExceeded,)
    );
}

#[test]
fn diagnostics_do_not_echo_revocation_payloads() {
    let encoded = revoked(&[generalized_time(b"20261324123456Z"), reason(&[0x0b])]);
    let error = decode(&encoded).unwrap_err();
    let diagnostic = error.to_string();
    assert!(!diagnostic.contains("20261324"));
    assert!(!diagnostic.contains("0b"));
}
