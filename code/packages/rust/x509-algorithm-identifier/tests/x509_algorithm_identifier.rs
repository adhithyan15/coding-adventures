use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_algorithm_identifier::{
    decode_x509_algorithm_identifier, X509AlgorithmIdentifierErrorKind,
};

const SHA256_WITH_RSA_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];

fn oid(contents: &[u8]) -> Vec<u8> {
    let mut encoded = vec![0x06, contents.len() as u8];
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
) -> Result<
    x509_algorithm_identifier::X509AlgorithmIdentifier<'_>,
    x509_algorithm_identifier::X509AlgorithmIdentifierError,
> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_algorithm_identifier(&mut decoder, element)
}

#[test]
fn decodes_oid_with_absent_parameters() {
    let encoded = sequence(&[oid(SHA256_WITH_RSA_OID)]);
    let value = decode(&encoded).unwrap();
    assert!(value.algorithm().equals(&[1, 2, 840, 113549, 1, 1, 11]));
    assert_eq!(value.parameters(), None);
}

#[test]
fn borrows_any_one_canonical_parameter_element() {
    for parameter in [
        vec![0x05, 0x00],
        vec![0x30, 0x00],
        vec![0x04, 0x02, 0xaa, 0xbb],
    ] {
        let encoded = sequence(&[oid(SHA256_WITH_RSA_OID), parameter.clone()]);
        let value = decode(&encoded).unwrap();
        assert_eq!(value.parameters().unwrap().encoded(), parameter);
    }
}

#[test]
fn root_must_be_a_constructed_universal_sequence() {
    for encoded in [vec![0x10, 0], vec![0x31, 0]] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509AlgorithmIdentifierErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }
}

#[test]
fn algorithm_is_required_and_must_be_a_valid_oid() {
    assert_eq!(
        decode(&sequence(&[])).unwrap_err().kind(),
        X509AlgorithmIdentifierErrorKind::MissingAlgorithm
    );
    assert_eq!(
        decode(&sequence(&[vec![0x05, 0]])).unwrap_err().kind(),
        X509AlgorithmIdentifierErrorKind::InvalidAlgorithm(Asn1ErrorKind::UnexpectedTag)
    );

    let error = decode(&sequence(&[oid(&[0x80])])).unwrap_err();
    assert_eq!(
        error.kind(),
        X509AlgorithmIdentifierErrorKind::InvalidAlgorithm(
            Asn1ErrorKind::NonMinimalObjectIdentifier
        )
    );
    assert_eq!(error.offset(), 4);
}

#[test]
fn oid_arc_limit_is_inherited_from_the_decoder() {
    let encoded = sequence(&[oid(&[0x2a, 0x03])]);
    let limits = Asn1Limits {
        max_oid_arcs: 2,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_algorithm_identifier(&mut decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509AlgorithmIdentifierErrorKind::InvalidAlgorithm(Asn1ErrorKind::OidArcLimitExceeded)
    );
    assert_eq!(error.offset(), 5);
}

#[test]
fn malformed_children_report_whole_element_offsets() {
    for encoded in [
        sequence(&[vec![0x06, 2, 0x2a]]),
        sequence(&[oid(SHA256_WITH_RSA_OID), vec![0x04, 2, 0xaa]]),
        sequence(&[oid(SHA256_WITH_RSA_OID), vec![0x05, 0], vec![0x04, 2, 0xaa]]),
    ] {
        let error = decode(&encoded).unwrap_err();
        assert!(matches!(
            error.kind(),
            X509AlgorithmIdentifierErrorKind::Structure(Asn1ErrorKind::Framing(_))
        ));
        assert_eq!(error.offset(), encoded.len());
    }
}

#[test]
fn a_third_child_is_rejected() {
    let encoded = sequence(&[oid(SHA256_WITH_RSA_OID), vec![0x05, 0], vec![0x05, 0]]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509AlgorithmIdentifierErrorKind::TrailingElement
    );
}

#[test]
fn shared_depth_and_element_limits_are_enforced() {
    let encoded = sequence(&[oid(SHA256_WITH_RSA_OID), vec![0x05, 0]]);

    let depth_limits = Asn1Limits {
        max_depth: 1,
        ..Asn1Limits::default()
    };
    let mut depth_decoder = Asn1Decoder::new(depth_limits);
    let root = depth_decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_algorithm_identifier(&mut depth_decoder, root)
            .unwrap_err()
            .kind(),
        X509AlgorithmIdentifierErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    );

    let element_limits = Asn1Limits {
        max_total_elements: 2,
        ..Asn1Limits::default()
    };
    let mut element_decoder = Asn1Decoder::new(element_limits);
    let root = element_decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_algorithm_identifier(&mut element_decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509AlgorithmIdentifierErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), 13);
}

#[test]
fn errors_are_redacted() {
    let encoded = sequence(&[oid(&[0x80])]);
    let error = decode(&encoded).unwrap_err();
    assert!(!error.to_string().contains("80"));
    assert!(error.to_string().contains("byte 4"));
}
