use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_basic_constraints::{
    decode_basic_constraints, BasicConstraintsErrorKind, BASIC_CONSTRAINTS_OID,
};
use x509_extension::decode_x509_extension;

const BASIC_CONSTRAINTS_OID_DER: &[u8] = &[0x55, 0x1d, 0x13];
const SUBJECT_ALT_NAME_OID_DER: &[u8] = &[0x55, 0x1d, 0x11];

fn oid(contents: &[u8]) -> Vec<u8> {
    let mut encoded = vec![0x06, contents.len() as u8];
    encoded.extend_from_slice(contents);
    encoded
}

fn octet_string(contents: &[u8]) -> Vec<u8> {
    let mut encoded = vec![0x04, contents.len() as u8];
    encoded.extend_from_slice(contents);
    encoded
}

fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    let value_length: usize = fields.iter().map(Vec::len).sum();
    assert!(value_length < 128);
    let mut encoded = vec![0x30, value_length as u8];
    for field in fields {
        encoded.extend_from_slice(field);
    }
    encoded
}

fn extension(oid_contents: &[u8], payload: &[u8]) -> Vec<u8> {
    sequence(&[oid(oid_contents), octet_string(payload)])
}

fn decode_with_limits(
    encoded: &[u8],
    limits: Asn1Limits,
) -> Result<
    x509_basic_constraints::BasicConstraints<'_>,
    x509_basic_constraints::BasicConstraintsError,
> {
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder
        .decode_exact(encoded)
        .expect("valid outer DER framing");
    let generic = decode_x509_extension(&mut decoder, root).expect("valid generic extension");
    decode_basic_constraints(&mut decoder, generic)
}

fn decode(
    encoded: &[u8],
) -> Result<
    x509_basic_constraints::BasicConstraints<'_>,
    x509_basic_constraints::BasicConstraintsError,
> {
    decode_with_limits(encoded, Asn1Limits::default())
}

#[test]
fn decodes_default_and_true_ca_forms() {
    assert_eq!(BASIC_CONSTRAINTS_OID, &[2, 5, 29, 19]);

    let default_payload = sequence(&[]);
    let default_extension = extension(BASIC_CONSTRAINTS_OID_DER, &default_payload);
    let default = decode(&default_extension).unwrap();
    assert!(!default.is_ca());
    assert!(default.path_len_constraint().is_none());

    let ca_payload = sequence(&[vec![0x01, 0x01, 0xff]]);
    let ca_extension = extension(BASIC_CONSTRAINTS_OID_DER, &ca_payload);
    let ca = decode(&ca_extension).unwrap();
    assert!(ca.is_ca());
    assert!(ca.path_len_constraint().is_none());
}

#[test]
fn leaves_outer_criticality_to_certificate_policy() {
    let payload = sequence(&[]);
    let encoded = sequence(&[
        oid(BASIC_CONSTRAINTS_OID_DER),
        vec![0x01, 0x01, 0xff],
        octet_string(&payload),
    ]);
    let constraints = decode(&encoded).unwrap();
    assert!(!constraints.is_ca());
}

#[test]
fn decodes_zero_and_multi_octet_path_lengths() {
    for (integer, expected) in [
        (vec![0x02, 0x01, 0x00], &[0x00][..]),
        (vec![0x02, 0x02, 0x01, 0x00], &[0x01, 0x00][..]),
    ] {
        let payload = sequence(&[vec![0x01, 0x01, 0xff], integer]);
        let encoded = extension(BASIC_CONSTRAINTS_OID_DER, &payload);
        let constraints = decode(&encoded).unwrap();
        assert!(constraints.is_ca());
        assert_eq!(
            constraints.path_len_constraint().unwrap().signed_bytes(),
            expected
        );
    }
}

#[test]
fn preserves_canonical_positive_path_lengths_larger_than_u64() {
    let integer = vec![0x02, 0x09, 0x01, 0, 0, 0, 0, 0, 0, 0, 0];
    let payload = sequence(&[vec![0x01, 0x01, 0xff], integer]);
    let encoded = extension(BASIC_CONSTRAINTS_OID_DER, &payload);
    let constraints = decode(&encoded).unwrap();
    let path_len = constraints.path_len_constraint().unwrap();
    assert_eq!(path_len.signed_bytes(), &[1, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(
        path_len.to_u64().unwrap_err().kind(),
        Asn1ErrorKind::IntegerOverflow
    );
}

#[test]
fn rejects_the_wrong_extension_identifier_before_payload_access() {
    let encoded = extension(SUBJECT_ALT_NAME_OID_DER, &[0xde, 0xad]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), BasicConstraintsErrorKind::WrongExtensionId);
    assert_eq!(error.offset(), 0);
}

#[test]
fn payload_must_be_one_exact_sequence() {
    for payload in [vec![], vec![0x31, 0], vec![0x30, 0, 0x05, 0]] {
        let error = decode(&extension(BASIC_CONSTRAINTS_OID_DER, &payload)).unwrap_err();
        assert!(matches!(
            error.kind(),
            BasicConstraintsErrorKind::Structure(_)
        ));
    }

    let empty = decode(&extension(BASIC_CONSTRAINTS_OID_DER, &[])).unwrap_err();
    assert!(matches!(
        empty.kind(),
        BasicConstraintsErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
}

#[test]
fn rejects_encoded_default_and_path_length_without_ca() {
    let encoded_default = sequence(&[vec![0x01, 0x01, 0x00]]);
    assert_eq!(
        decode(&extension(BASIC_CONSTRAINTS_OID_DER, &encoded_default))
            .unwrap_err()
            .kind(),
        BasicConstraintsErrorKind::EncodedDefaultCa
    );

    let path_without_ca = sequence(&[vec![0x02, 0x01, 0x03]]);
    assert_eq!(
        decode(&extension(BASIC_CONSTRAINTS_OID_DER, &path_without_ca))
            .unwrap_err()
            .kind(),
        BasicConstraintsErrorKind::PathLenWithoutCa
    );
}

#[test]
fn rejects_negative_and_malformed_path_lengths() {
    let negative = sequence(&[vec![0x01, 0x01, 0xff], vec![0x02, 0x01, 0xff]]);
    let error = decode(&extension(BASIC_CONSTRAINTS_OID_DER, &negative)).unwrap_err();
    assert_eq!(error.kind(), BasicConstraintsErrorKind::NegativePathLen);
    assert_eq!(error.offset(), 7);

    for integer in [vec![0x02, 0], vec![0x02, 0x02, 0x00, 0x01], vec![0x05, 0]] {
        let payload = sequence(&[vec![0x01, 0x01, 0xff], integer]);
        assert!(matches!(
            decode(&extension(BASIC_CONSTRAINTS_OID_DER, &payload))
                .unwrap_err()
                .kind(),
            BasicConstraintsErrorKind::InvalidPathLen(_)
        ));
    }
}

#[test]
fn rejects_invalid_ca_and_trailing_fields() {
    for boolean in [vec![0x01, 0], vec![0x01, 0x01, 0x01], vec![0x21, 1, 0xff]] {
        let payload = sequence(&[boolean]);
        assert!(matches!(
            decode(&extension(BASIC_CONSTRAINTS_OID_DER, &payload))
                .unwrap_err()
                .kind(),
            BasicConstraintsErrorKind::InvalidCa(_)
        ));
    }

    let trailing = sequence(&[
        vec![0x01, 0x01, 0xff],
        vec![0x02, 0x01, 0x01],
        vec![0x05, 0],
    ]);
    let error = decode(&extension(BASIC_CONSTRAINTS_OID_DER, &trailing)).unwrap_err();
    assert_eq!(error.kind(), BasicConstraintsErrorKind::TrailingElement);
    assert_eq!(error.offset(), 8);
}

#[test]
fn malformed_children_report_payload_local_offsets() {
    for payload in [
        sequence(&[vec![0x01, 2, 0xff]]),
        sequence(&[vec![0x01, 0x01, 0xff], vec![0x02, 2, 1]]),
    ] {
        let error = decode(&extension(BASIC_CONSTRAINTS_OID_DER, &payload)).unwrap_err();
        assert!(matches!(
            error.kind(),
            BasicConstraintsErrorKind::Structure(Asn1ErrorKind::Framing(_))
        ));
        assert_eq!(error.offset(), payload.len());
    }
}

#[test]
fn shared_element_and_inner_depth_limits_are_enforced() {
    let payload = sequence(&[vec![0x01, 0x01, 0xff]]);
    let encoded = extension(BASIC_CONSTRAINTS_OID_DER, &payload);

    let element_limits = Asn1Limits {
        max_total_elements: 3,
        ..Asn1Limits::default()
    };
    let error = decode_with_limits(&encoded, element_limits).unwrap_err();
    assert_eq!(
        error.kind(),
        BasicConstraintsErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), 0);

    let first_child_limits = Asn1Limits {
        max_total_elements: 4,
        ..Asn1Limits::default()
    };
    let error = decode_with_limits(&encoded, first_child_limits).unwrap_err();
    assert_eq!(
        error.kind(),
        BasicConstraintsErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), 2);

    let trailing_payload = sequence(&[
        vec![0x01, 0x01, 0xff],
        vec![0x02, 0x01, 0x01],
        vec![0x05, 0],
    ]);
    let trailing_encoded = extension(BASIC_CONSTRAINTS_OID_DER, &trailing_payload);
    let trailing_limits = Asn1Limits {
        max_total_elements: 6,
        ..Asn1Limits::default()
    };
    let error = decode_with_limits(&trailing_encoded, trailing_limits).unwrap_err();
    assert_eq!(
        error.kind(),
        BasicConstraintsErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), 8);

    let mut outer_decoder = Asn1Decoder::new(Asn1Limits::default());
    let outer = outer_decoder.decode_exact(&encoded).unwrap();
    let generic = decode_x509_extension(&mut outer_decoder, outer).unwrap();
    let depth_limits = Asn1Limits {
        max_depth: 1,
        ..Asn1Limits::default()
    };
    let mut inner_decoder = Asn1Decoder::new(depth_limits);
    let error = decode_basic_constraints(&mut inner_decoder, generic).unwrap_err();
    assert_eq!(
        error.kind(),
        BasicConstraintsErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    );
}

#[test]
fn errors_are_redacted() {
    let payload = sequence(&[vec![0x01, 0x01, 0xff], vec![0x02, 0x01, 0xff]]);
    let error = decode(&extension(BASIC_CONSTRAINTS_OID_DER, &payload)).unwrap_err();
    assert!(!error.to_string().contains("ff"));
    assert!(error.to_string().contains("byte 7"));
}
