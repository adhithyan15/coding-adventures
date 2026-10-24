use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_algorithm_identifier::X509AlgorithmIdentifierErrorKind;
use x509_ocsp_cert_id::{decode_x509_ocsp_cert_id, X509OcspCertIdErrorKind};
use x509_serial_number::X509SerialNumberErrorKind;

const SHA1_OID: &[u8] = &[0x2b, 0x0e, 0x03, 0x02, 0x1a];

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
    let mut fields = vec![oid(SHA1_OID)];
    if let Some(parameters) = parameters {
        fields.push(parameters);
    }
    sequence(&fields)
}

fn octet_string(contents: &[u8]) -> Vec<u8> {
    let mut encoded = vec![0x04, contents.len() as u8];
    encoded.extend_from_slice(contents);
    encoded
}

fn integer(contents: &[u8]) -> Vec<u8> {
    let mut encoded = vec![0x02, contents.len() as u8];
    encoded.extend_from_slice(contents);
    encoded
}

fn cert_id(
    hash_algorithm: Vec<u8>,
    issuer_name_hash: Vec<u8>,
    issuer_key_hash: Vec<u8>,
    serial_number: Vec<u8>,
) -> Vec<u8> {
    sequence(&[
        hash_algorithm,
        issuer_name_hash,
        issuer_key_hash,
        serial_number,
    ])
}

fn valid_cert_id() -> Vec<u8> {
    cert_id(
        algorithm(Some(vec![0x05, 0x00])),
        octet_string(&[0x11; 20]),
        octet_string(&[0x22; 20]),
        integer(&[0x01, 0x23]),
    )
}

fn decode(
    encoded: &[u8],
) -> Result<
    x509_ocsp_cert_id::X509OcspCertId<'_>,
    x509_ocsp_cert_id::X509OcspCertIdError,
> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_ocsp_cert_id(&mut decoder, element)
}

#[test]
fn decodes_the_exact_borrowed_certificate_identifier() {
    let encoded = valid_cert_id();
    let value = decode(&encoded).unwrap();

    assert_eq!(value.contents(), &encoded[2..]);
    assert!(value
        .hash_algorithm()
        .algorithm()
        .equals(&[1, 3, 14, 3, 2, 26]));
    assert_eq!(
        value.hash_algorithm().parameters().unwrap().encoded(),
        &[0x05, 0x00]
    );
    assert_eq!(value.issuer_name_hash(), &[0x11; 20]);
    assert_eq!(value.issuer_key_hash(), &[0x22; 20]);
    assert_eq!(value.serial_number().encoded_value(), &[0x01, 0x23]);
}

#[test]
fn hash_shapes_remain_data_driven_and_serial_numbers_remain_canonical() {
    for (name_hash, key_hash) in [
        (&[][..], &[][..]),
        (&[0xaa][..], &[0xbb, 0xcc][..]),
        (&[0x11; 32][..], &[0x22; 48][..]),
    ] {
        let encoded = cert_id(
            algorithm(None),
            octet_string(name_hash),
            octet_string(key_hash),
            integer(&[0x00, 0x80]),
        );
        let value = decode(&encoded).unwrap();
        assert_eq!(value.issuer_name_hash(), name_hash);
        assert_eq!(value.issuer_key_hash(), key_hash);
        assert_eq!(value.serial_number().magnitude(), &[0x80]);
    }
}

#[test]
fn root_must_be_one_constructed_universal_sequence() {
    for encoded in [vec![0x10, 0], vec![0x31, 0]] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509OcspCertIdErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }
}

#[test]
fn every_field_is_required_in_schema_order() {
    let fields = [
        algorithm(None),
        octet_string(&[0x11]),
        octet_string(&[0x22]),
        integer(&[0x01]),
    ];
    let expected = [
        X509OcspCertIdErrorKind::MissingHashAlgorithm,
        X509OcspCertIdErrorKind::MissingIssuerNameHash,
        X509OcspCertIdErrorKind::MissingIssuerKeyHash,
        X509OcspCertIdErrorKind::MissingSerialNumber,
    ];

    for field_count in 0..fields.len() {
        assert_eq!(
            decode(&sequence(&fields[..field_count])).unwrap_err().kind(),
            expected[field_count]
        );
    }
}

#[test]
fn hash_algorithm_is_fully_validated_under_shared_limits() {
    let wrong_tag = cert_id(
        vec![0x05, 0x00],
        octet_string(&[0x11]),
        octet_string(&[0x22]),
        integer(&[0x01]),
    );
    assert_eq!(
        decode(&wrong_tag).unwrap_err().kind(),
        X509OcspCertIdErrorKind::InvalidHashAlgorithm(
            X509AlgorithmIdentifierErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        )
    );

    let invalid_oid = cert_id(
        sequence(&[oid(&[0x80])]),
        octet_string(&[0x11]),
        octet_string(&[0x22]),
        integer(&[0x01]),
    );
    assert_eq!(
        decode(&invalid_oid).unwrap_err().kind(),
        X509OcspCertIdErrorKind::InvalidHashAlgorithm(
            X509AlgorithmIdentifierErrorKind::InvalidAlgorithm(
                Asn1ErrorKind::NonMinimalObjectIdentifier
            )
        )
    );

    let encoded = cert_id(
        sequence(&[oid(&[0x2a, 0x03])]),
        octet_string(&[0x11]),
        octet_string(&[0x22]),
        integer(&[0x01]),
    );
    let limits = Asn1Limits {
        max_oid_arcs: 2,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_ocsp_cert_id(&mut decoder, root)
            .unwrap_err()
            .kind(),
        X509OcspCertIdErrorKind::InvalidHashAlgorithm(
            X509AlgorithmIdentifierErrorKind::InvalidAlgorithm(
                Asn1ErrorKind::OidArcLimitExceeded
            )
        )
    );
}

#[test]
fn issuer_hashes_must_be_primitive_octet_strings() {
    for (name_hash, key_hash, expected) in [
        (
            vec![0x05, 0x00],
            octet_string(&[0x22]),
            X509OcspCertIdErrorKind::InvalidIssuerNameHash(Asn1ErrorKind::UnexpectedTag),
        ),
        (
            octet_string(&[0x11]),
            vec![0x24, 0x00],
            X509OcspCertIdErrorKind::InvalidIssuerKeyHash(Asn1ErrorKind::UnexpectedTag),
        ),
    ] {
        let encoded = cert_id(algorithm(None), name_hash, key_hash, integer(&[0x01]));
        assert_eq!(decode(&encoded).unwrap_err().kind(), expected);
    }
}

#[test]
fn serial_number_uses_the_strict_rfc_5280_primitive() {
    for (serial, expected) in [
        (
            integer(&[0x00]),
            X509SerialNumberErrorKind::NotPositive,
        ),
        (
            integer(&[0x80]),
            X509SerialNumberErrorKind::NotPositive,
        ),
        (
            integer(&[0x01; 21]),
            X509SerialNumberErrorKind::TooLong,
        ),
    ] {
        let encoded = cert_id(
            algorithm(None),
            octet_string(&[0x11]),
            octet_string(&[0x22]),
            serial,
        );
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509OcspCertIdErrorKind::InvalidSerialNumber(expected)
        );
    }
}

#[test]
fn malformed_children_and_trailing_fields_fail_closed() {
    for encoded in [
        sequence(&[vec![0x30, 2, 0x06]]),
        sequence(&[algorithm(None), vec![0x04, 2, 0xaa]]),
        sequence(&[
            algorithm(None),
            octet_string(&[0x11]),
            vec![0x04, 2, 0xaa],
        ]),
        sequence(&[
            algorithm(None),
            octet_string(&[0x11]),
            octet_string(&[0x22]),
            vec![0x02, 2, 0x01],
        ]),
    ] {
        let error = decode(&encoded).unwrap_err();
        assert!(matches!(
            error.kind(),
            X509OcspCertIdErrorKind::Structure(Asn1ErrorKind::Framing(_))
                | X509OcspCertIdErrorKind::InvalidHashAlgorithm(
                    X509AlgorithmIdentifierErrorKind::Structure(Asn1ErrorKind::Framing(_))
                )
        ));
        assert_eq!(error.offset(), encoded.len());
    }

    let mut fields = vec![
        algorithm(None),
        octet_string(&[0x11]),
        octet_string(&[0x22]),
        integer(&[0x01]),
    ];
    fields.push(vec![0x05, 0x00]);
    assert_eq!(
        decode(&sequence(&fields)).unwrap_err().kind(),
        X509OcspCertIdErrorKind::TrailingElement
    );
}

#[test]
fn shared_depth_and_element_budgets_are_enforced() {
    let encoded = valid_cert_id();

    let depth_limits = Asn1Limits {
        max_depth: 2,
        ..Asn1Limits::default()
    };
    let mut depth_decoder = Asn1Decoder::new(depth_limits);
    let root = depth_decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_ocsp_cert_id(&mut depth_decoder, root)
            .unwrap_err()
            .kind(),
        X509OcspCertIdErrorKind::InvalidHashAlgorithm(
            X509AlgorithmIdentifierErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
        )
    );

    let element_limits = Asn1Limits {
        max_total_elements: 5,
        ..Asn1Limits::default()
    };
    let mut element_decoder = Asn1Decoder::new(element_limits);
    let root = element_decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_ocsp_cert_id(&mut element_decoder, root)
            .unwrap_err()
            .kind(),
        X509OcspCertIdErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
}

#[test]
fn diagnostics_and_debug_output_do_not_echo_identifier_payloads() {
    let encoded = cert_id(
        sequence(&[oid(&[0x80])]),
        octet_string(&[0xaa]),
        octet_string(&[0xbb]),
        integer(&[0x01]),
    );
    let error = decode(&encoded).unwrap_err();
    assert!(!error.to_string().contains("80"));
    assert!(!error.to_string().contains("aa"));

    let valid = valid_cert_id();
    let value = decode(&valid).unwrap();
    let debug = format!("{value:?}");
    assert!(!debug.contains("17, 17"));
    assert!(!debug.contains("34, 34"));
    assert!(!debug.contains("291"));
}
