use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_extension::{decode_x509_extension, X509ExtensionErrorKind};

const SUBJECT_ALT_NAME_OID: &[u8] = &[0x55, 0x1d, 0x11];

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

fn decode(
    encoded: &[u8],
) -> Result<x509_extension::X509Extension<'_>, x509_extension::X509ExtensionError> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_extension(&mut decoder, element)
}

#[test]
fn decodes_absent_critical_as_false_and_borrows_value() {
    let inner_value = [0x30, 0x00];
    let encoded = sequence(&[oid(SUBJECT_ALT_NAME_OID), octet_string(&inner_value)]);
    let extension = decode(&encoded).unwrap();
    assert!(extension.extension_id().equals(&[2, 5, 29, 17]));
    assert!(!extension.critical());
    assert_eq!(extension.extension_value(), inner_value);
}

#[test]
fn decodes_explicit_true_critical() {
    let encoded = sequence(&[
        oid(SUBJECT_ALT_NAME_OID),
        vec![0x01, 0x01, 0xff],
        octet_string(&[0x30, 0x00]),
    ]);
    let extension = decode(&encoded).unwrap();
    assert!(extension.critical());
}

#[test]
fn rejects_explicit_default_false() {
    let encoded = sequence(&[
        oid(SUBJECT_ALT_NAME_OID),
        vec![0x01, 0x01, 0x00],
        octet_string(&[]),
    ]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), X509ExtensionErrorKind::EncodedDefaultCritical);
    assert_eq!(error.offset(), 7);
}

#[test]
fn accepts_empty_opaque_extension_value() {
    let encoded = sequence(&[oid(SUBJECT_ALT_NAME_OID), octet_string(&[])]);
    assert_eq!(decode(&encoded).unwrap().extension_value(), []);
}

#[test]
fn root_must_be_a_constructed_universal_sequence() {
    for encoded in [vec![0x10, 0], vec![0x31, 0]] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509ExtensionErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }
}

#[test]
fn extension_id_is_required_and_must_be_a_valid_oid() {
    assert_eq!(
        decode(&sequence(&[])).unwrap_err().kind(),
        X509ExtensionErrorKind::MissingExtensionId
    );
    assert_eq!(
        decode(&sequence(&[vec![0x05, 0], octet_string(&[])]))
            .unwrap_err()
            .kind(),
        X509ExtensionErrorKind::InvalidExtensionId(Asn1ErrorKind::UnexpectedTag)
    );

    let error = decode(&sequence(&[oid(&[0x80]), octet_string(&[])])).unwrap_err();
    assert_eq!(
        error.kind(),
        X509ExtensionErrorKind::InvalidExtensionId(Asn1ErrorKind::NonMinimalObjectIdentifier)
    );
    assert_eq!(error.offset(), 4);
}

#[test]
fn oid_arc_limit_is_inherited_from_the_decoder() {
    let encoded = sequence(&[oid(&[0x55, 0x1d, 0x11]), octet_string(&[])]);
    let limits = Asn1Limits {
        max_oid_arcs: 2,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_extension(&mut decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509ExtensionErrorKind::InvalidExtensionId(Asn1ErrorKind::OidArcLimitExceeded)
    );
    assert_eq!(error.offset(), 5);
}

#[test]
fn critical_must_be_canonical_boolean_true_when_present() {
    for critical in [
        vec![0x01, 0x00],
        vec![0x01, 0x01, 0x01],
        vec![0x21, 0x01, 0xff],
    ] {
        let encoded = sequence(&[oid(SUBJECT_ALT_NAME_OID), critical, octet_string(&[])]);
        let error = decode(&encoded).unwrap_err();
        assert!(matches!(
            error.kind(),
            X509ExtensionErrorKind::InvalidCritical(_)
        ));
    }
}

#[test]
fn extension_value_is_required_and_must_be_an_octet_string() {
    for fields in [
        vec![oid(SUBJECT_ALT_NAME_OID)],
        vec![oid(SUBJECT_ALT_NAME_OID), vec![0x01, 0x01, 0xff]],
    ] {
        assert_eq!(
            decode(&sequence(&fields)).unwrap_err().kind(),
            X509ExtensionErrorKind::MissingExtensionValue
        );
    }

    assert_eq!(
        decode(&sequence(&[oid(SUBJECT_ALT_NAME_OID), vec![0x05, 0]]))
            .unwrap_err()
            .kind(),
        X509ExtensionErrorKind::InvalidExtensionValue(Asn1ErrorKind::UnexpectedTag)
    );
}

#[test]
fn malformed_children_report_whole_element_offsets() {
    for encoded in [
        sequence(&[vec![0x06, 2, 0x55]]),
        sequence(&[oid(SUBJECT_ALT_NAME_OID), vec![0x01, 2, 0xff]]),
        sequence(&[oid(SUBJECT_ALT_NAME_OID), vec![0x04, 2, 0xaa]]),
        sequence(&[
            oid(SUBJECT_ALT_NAME_OID),
            vec![0x01, 0x01, 0xff],
            vec![0x04, 2, 0xaa],
        ]),
    ] {
        let error = decode(&encoded).unwrap_err();
        assert!(matches!(
            error.kind(),
            X509ExtensionErrorKind::Structure(Asn1ErrorKind::Framing(_))
        ));
        assert_eq!(error.offset(), encoded.len());
    }
}

#[test]
fn trailing_child_is_rejected() {
    let encoded = sequence(&[oid(SUBJECT_ALT_NAME_OID), octet_string(&[]), vec![0x05, 0]]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), X509ExtensionErrorKind::TrailingElement);
    assert_eq!(error.offset(), encoded.len() - 2);
}

#[test]
fn shared_depth_and_element_limits_are_enforced() {
    let encoded = sequence(&[
        oid(SUBJECT_ALT_NAME_OID),
        vec![0x01, 0x01, 0xff],
        octet_string(&[]),
    ]);

    let depth_limits = Asn1Limits {
        max_depth: 1,
        ..Asn1Limits::default()
    };
    let mut depth_decoder = Asn1Decoder::new(depth_limits);
    let root = depth_decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_extension(&mut depth_decoder, root)
            .unwrap_err()
            .kind(),
        X509ExtensionErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    );

    let element_limits = Asn1Limits {
        max_total_elements: 3,
        ..Asn1Limits::default()
    };
    let mut element_decoder = Asn1Decoder::new(element_limits);
    let root = element_decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_extension(&mut element_decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509ExtensionErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), 10);
}

#[test]
fn errors_are_redacted() {
    let encoded = sequence(&[oid(&[0x80]), octet_string(&[0xde, 0xad])]);
    let error = decode(&encoded).unwrap_err();
    assert!(!error.to_string().contains("80"));
    assert!(!error.to_string().contains("dead"));
    assert!(error.to_string().contains("byte 4"));
}
