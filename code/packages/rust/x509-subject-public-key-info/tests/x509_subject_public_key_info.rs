use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_algorithm_identifier::X509AlgorithmIdentifierErrorKind;
use x509_subject_public_key_info::{
    decode_x509_subject_public_key_info, X509SubjectPublicKeyInfoErrorKind,
};

const RSA_ENCRYPTION_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

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

fn algorithm(parameters: Option<Vec<u8>>) -> Vec<u8> {
    let mut fields = vec![oid(RSA_ENCRYPTION_OID)];
    if let Some(parameters) = parameters {
        fields.push(parameters);
    }
    sequence(&fields)
}

fn bit_string(unused_bits: u8, bytes: &[u8]) -> Vec<u8> {
    let mut encoded = vec![0x03, (bytes.len() + 1) as u8, unused_bits];
    encoded.extend_from_slice(bytes);
    encoded
}

fn decode(
    encoded: &[u8],
) -> Result<
    x509_subject_public_key_info::X509SubjectPublicKeyInfo<'_>,
    x509_subject_public_key_info::X509SubjectPublicKeyInfoError,
> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_subject_public_key_info(&mut decoder, element)
}

#[test]
fn decodes_borrowed_algorithm_and_subject_public_key() {
    let encoded = sequence(&[
        algorithm(Some(vec![0x05, 0x00])),
        bit_string(0, &[0x30, 0x01, 0x00]),
    ]);
    let value = decode(&encoded).unwrap();
    assert!(value
        .algorithm()
        .algorithm()
        .equals(&[1, 2, 840, 113549, 1, 1, 1]));
    assert_eq!(
        value.algorithm().parameters().unwrap().encoded(),
        &[0x05, 0x00]
    );
    assert_eq!(value.subject_public_key().bytes(), &[0x30, 0x01, 0x00]);
    assert_eq!(value.subject_public_key().bit_len(), 24);
}

#[test]
fn accepts_absent_parameters_and_any_canonical_bit_shape() {
    for key in [
        bit_string(0, &[]),
        bit_string(0, &[0xaa]),
        bit_string(3, &[0xa8]),
    ] {
        let encoded = sequence(&[algorithm(None), key]);
        decode(&encoded).unwrap();
    }
}

#[test]
fn root_must_be_a_constructed_universal_sequence() {
    for encoded in [vec![0x10, 0], vec![0x31, 0]] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509SubjectPublicKeyInfoErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }
}

#[test]
fn algorithm_is_required_and_fully_validated() {
    assert_eq!(
        decode(&sequence(&[])).unwrap_err().kind(),
        X509SubjectPublicKeyInfoErrorKind::MissingAlgorithm
    );
    assert_eq!(
        decode(&sequence(&[vec![0x05, 0x00], bit_string(0, &[0xaa])]))
            .unwrap_err()
            .kind(),
        X509SubjectPublicKeyInfoErrorKind::InvalidAlgorithm(
            X509AlgorithmIdentifierErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        )
    );

    let encoded = sequence(&[sequence(&[oid(&[0x80])]), bit_string(0, &[0xaa])]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        X509SubjectPublicKeyInfoErrorKind::InvalidAlgorithm(
            X509AlgorithmIdentifierErrorKind::InvalidAlgorithm(
                Asn1ErrorKind::NonMinimalObjectIdentifier
            )
        )
    );
    assert_eq!(error.offset(), 6);

    let malformed = sequence(&[sequence(&[vec![0x06, 2, 0x2a]]), bit_string(0, &[0xaa])]);
    let error = decode(&malformed).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509SubjectPublicKeyInfoErrorKind::InvalidAlgorithm(
            X509AlgorithmIdentifierErrorKind::Structure(Asn1ErrorKind::Framing(_))
        )
    ));
    assert_eq!(error.offset(), 7);
}

#[test]
fn algorithm_oid_arc_limit_is_inherited() {
    let encoded = sequence(&[sequence(&[oid(&[0x2a, 0x03])]), bit_string(0, &[0xaa])]);
    let limits = Asn1Limits {
        max_oid_arcs: 2,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_subject_public_key_info(&mut decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509SubjectPublicKeyInfoErrorKind::InvalidAlgorithm(
            X509AlgorithmIdentifierErrorKind::InvalidAlgorithm(Asn1ErrorKind::OidArcLimitExceeded)
        )
    );
}

#[test]
fn subject_public_key_is_required_and_fully_validated() {
    assert_eq!(
        decode(&sequence(&[algorithm(None)])).unwrap_err().kind(),
        X509SubjectPublicKeyInfoErrorKind::MissingSubjectPublicKey
    );
    assert_eq!(
        decode(&sequence(&[algorithm(None), vec![0x05, 0x00]]))
            .unwrap_err()
            .kind(),
        X509SubjectPublicKeyInfoErrorKind::InvalidSubjectPublicKey(Asn1ErrorKind::UnexpectedTag)
    );

    let encoded = sequence(&[algorithm(None), bit_string(3, &[0xaf])]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        X509SubjectPublicKeyInfoErrorKind::InvalidSubjectPublicKey(
            Asn1ErrorKind::NonZeroBitPadding
        )
    );
    assert_eq!(error.offset(), encoded.len() - 1);

    assert_eq!(
        decode(&sequence(&[algorithm(None), bit_string(8, &[0x00])]))
            .unwrap_err()
            .kind(),
        X509SubjectPublicKeyInfoErrorKind::InvalidSubjectPublicKey(
            Asn1ErrorKind::InvalidUnusedBitCount
        )
    );
}

#[test]
fn malformed_children_report_whole_element_offsets() {
    for encoded in [
        sequence(&[vec![0x30, 2, 0x06]]),
        sequence(&[algorithm(None), vec![0x03, 2, 0x00]]),
        sequence(&[algorithm(None), bit_string(0, &[0xaa]), vec![0x05, 1]]),
    ] {
        let error = decode(&encoded).unwrap_err();
        assert!(matches!(
            error.kind(),
            X509SubjectPublicKeyInfoErrorKind::Structure(Asn1ErrorKind::Framing(_))
        ));
        assert_eq!(error.offset(), encoded.len());
    }
}

#[test]
fn a_third_child_is_rejected() {
    let encoded = sequence(&[algorithm(None), bit_string(0, &[0xaa]), vec![0x05, 0x00]]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509SubjectPublicKeyInfoErrorKind::TrailingElement
    );
}

#[test]
fn shared_depth_and_element_limits_are_enforced() {
    let encoded = sequence(&[algorithm(None), bit_string(0, &[0xaa])]);

    let depth_limits = Asn1Limits {
        max_depth: 2,
        ..Asn1Limits::default()
    };
    let mut depth_decoder = Asn1Decoder::new(depth_limits);
    let root = depth_decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_subject_public_key_info(&mut depth_decoder, root)
            .unwrap_err()
            .kind(),
        X509SubjectPublicKeyInfoErrorKind::InvalidAlgorithm(
            X509AlgorithmIdentifierErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
        )
    );

    let element_limits = Asn1Limits {
        max_total_elements: 3,
        ..Asn1Limits::default()
    };
    let mut element_decoder = Asn1Decoder::new(element_limits);
    let root = element_decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_subject_public_key_info(&mut element_decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509SubjectPublicKeyInfoErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), algorithm(None).len() + 2);
}

#[test]
fn errors_are_redacted() {
    let encoded = sequence(&[sequence(&[oid(&[0x80])]), bit_string(0, &[0xaa])]);
    let error = decode(&encoded).unwrap_err();
    assert!(!error.to_string().contains("80"));
    assert!(error.to_string().contains("byte 6"));
}
