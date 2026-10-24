use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use der_tlv::DerErrorKind;
use x509_ocsp_basic_response::{
    decode_x509_ocsp_basic_response, X509OcspBasicResponseErrorKind, MAX_X509_OCSP_CERTIFICATES,
};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

fn der_length(length: usize) -> Vec<u8> {
    if length < 128 {
        return vec![length as u8];
    }
    let bytes = length.to_be_bytes();
    let first = bytes.iter().position(|byte| *byte != 0).unwrap();
    let significant = &bytes[first..];
    let mut encoded = vec![0x80 | significant.len() as u8];
    encoded.extend_from_slice(significant);
    encoded
}

fn element(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = vec![tag];
    encoded.extend_from_slice(&der_length(value.len()));
    encoded.extend_from_slice(value);
    encoded
}

fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    let value: Vec<u8> = fields.iter().flatten().copied().collect();
    element(0x30, &value)
}

fn algorithm(oid: &[u8]) -> Vec<u8> {
    sequence(&[element(0x06, oid), element(0x05, &[])])
}

fn time(value: &[u8]) -> Vec<u8> {
    element(0x18, value)
}

fn cert_id(serial: u8) -> Vec<u8> {
    sequence(&[
        algorithm(&[0x2b, 0x0e, 0x03, 0x02, 0x1a]),
        element(0x04, &[0x11; 20]),
        element(0x04, &[0x22; 20]),
        element(0x02, &[serial]),
    ])
}

fn single_response(serial: u8) -> Vec<u8> {
    sequence(&[cert_id(serial), vec![0x80, 0x00], time(b"20261024123456Z")])
}

fn responder_by_key() -> Vec<u8> {
    element(0xa2, &element(0x04, &[0x44; 20]))
}

fn response_data(serial: u8) -> Vec<u8> {
    sequence(&[
        responder_by_key(),
        time(b"20261024120000Z"),
        sequence(&[single_response(serial)]),
    ])
}

fn validity() -> Vec<u8> {
    sequence(&[
        element(0x17, b"260101000000Z"),
        element(0x17, b"270101000000Z"),
    ])
}

fn name(label: &[u8]) -> Vec<u8> {
    let attribute = sequence(&[element(0x06, &[0x55, 0x04, 0x03]), element(0x0c, label)]);
    sequence(&[element(0x31, &attribute)])
}

fn subject_public_key_info() -> Vec<u8> {
    sequence(&[algorithm(KEY_OID), element(0x03, &[0x00, 0x30, 0x00])])
}

fn certificate(serial: u8) -> Vec<u8> {
    let tbs = sequence(&[
        element(0xa0, &element(0x02, &[2])),
        element(0x02, &[serial]),
        algorithm(SIGNATURE_OID),
        name(b"issuer"),
        validity(),
        name(b"responder"),
        subject_public_key_info(),
    ]);
    sequence(&[
        tbs,
        algorithm(SIGNATURE_OID),
        element(0x03, &[0x00, 0xca, 0xfe]),
    ])
}

fn certificates(values: &[Vec<u8>]) -> Vec<u8> {
    element(0xa0, &sequence(values))
}

fn basic_response(optional_certificates: Option<&[Vec<u8>]>) -> Vec<u8> {
    let mut fields = vec![
        response_data(1),
        algorithm(SIGNATURE_OID),
        element(0x03, &[0x00, 0xde, 0xad, 0xbe, 0xef]),
    ];
    if let Some(values) = optional_certificates {
        fields.push(certificates(values));
    }
    sequence(&fields)
}

fn decode(
    encoded: &[u8],
) -> Result<
    x509_ocsp_basic_response::X509OcspBasicResponse<'_>,
    x509_ocsp_basic_response::X509OcspBasicResponseError,
> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_ocsp_basic_response(&mut decoder, root)
}

#[test]
fn decodes_required_fields_and_preserves_exact_signed_bytes() {
    let encoded = basic_response(None);
    let decoded = decode(&encoded).unwrap();

    assert_eq!(decoded.encoded(), encoded);
    assert_eq!(decoded.response_data().encoded(), response_data(1));
    assert!(decoded
        .signature_algorithm()
        .algorithm()
        .equals(&[1, 2, 840, 113549, 1, 1, 11]));
    assert_eq!(decoded.signature().bytes(), &[0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(decoded.signature().unused_bits(), 0);
    assert!(decoded.is_empty());
    assert_eq!(decoded.len(), 0);
    assert_eq!(decoded.iter().count(), 0);
    assert!(decoded.get(0).is_none());

    let debug = format!("{decoded:?}");
    assert!(debug.contains("signature_octet_count"));
    assert!(debug.contains("certificate_count"));
    assert!(!debug.contains("deadbeef"));
}

#[test]
fn decodes_one_bounded_certificate_through_the_shared_decoder() {
    let certs = [certificate(1)];
    let encoded = basic_response(Some(&certs));
    let decoded = decode(&encoded).unwrap();

    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded.iter().count(), 1);
    assert_eq!(
        decoded
            .get(0)
            .unwrap()
            .tbs_certificate()
            .serial_number()
            .magnitude(),
        &[1]
    );
    assert!(decoded.get(1).is_none());
}

#[test]
fn an_explicit_empty_certificate_sequence_is_structurally_valid() {
    let encoded = basic_response(Some(&[]));
    let decoded = decode(&encoded).unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn root_and_required_fields_are_enforced_in_schema_order() {
    for encoded in [vec![0x10, 0], vec![0x31, 0]] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509OcspBasicResponseErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }

    assert_eq!(
        decode(&sequence(&[])).unwrap_err().kind(),
        X509OcspBasicResponseErrorKind::MissingResponseData
    );
    assert_eq!(
        decode(&sequence(&[response_data(1)])).unwrap_err().kind(),
        X509OcspBasicResponseErrorKind::MissingSignatureAlgorithm
    );
    assert_eq!(
        decode(&sequence(&[response_data(1), algorithm(SIGNATURE_OID)]))
            .unwrap_err()
            .kind(),
        X509OcspBasicResponseErrorKind::MissingSignature
    );
}

#[test]
fn rejects_invalid_response_algorithm_and_signature_values() {
    let invalid_response =
        sequence(&[sequence(&[]), algorithm(SIGNATURE_OID), element(0x03, &[0])]);
    assert!(matches!(
        decode(&invalid_response).unwrap_err().kind(),
        X509OcspBasicResponseErrorKind::InvalidResponseData(_)
    ));

    let invalid_algorithm = sequence(&[response_data(1), sequence(&[]), element(0x03, &[0])]);
    assert!(matches!(
        decode(&invalid_algorithm).unwrap_err().kind(),
        X509OcspBasicResponseErrorKind::InvalidSignatureAlgorithm(_)
    ));

    for signature in [element(0x04, &[0]), element(0x03, &[])] {
        let encoded = sequence(&[response_data(1), algorithm(SIGNATURE_OID), signature]);
        assert!(matches!(
            decode(&encoded).unwrap_err().kind(),
            X509OcspBasicResponseErrorKind::InvalidSignature(_)
        ));
    }
}

#[test]
fn certificate_wrapper_must_be_exact_and_last() {
    let required = [
        response_data(1),
        algorithm(SIGNATURE_OID),
        element(0x03, &[0]),
    ];

    let mut wrong_tag = required.to_vec();
    wrong_tag.push(element(0xa1, &sequence(&[])));
    assert_eq!(
        decode(&sequence(&wrong_tag)).unwrap_err().kind(),
        X509OcspBasicResponseErrorKind::UnexpectedOptionalField
    );

    for wrapper in [
        element(0xa0, &[]),
        element(0xa0, &element(0x31, &[])),
        element(
            0xa0,
            &[sequence(&[]), element(0x05, &[])]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
        ),
    ] {
        let mut fields = required.to_vec();
        fields.push(wrapper);
        assert!(matches!(
            decode(&sequence(&fields)).unwrap_err().kind(),
            X509OcspBasicResponseErrorKind::InvalidCertificatesWrapper(_)
                | X509OcspBasicResponseErrorKind::InvalidCertificates(_)
        ));
    }

    let mut trailing = required.to_vec();
    trailing.push(certificates(&[]));
    trailing.push(element(0x05, &[]));
    assert_eq!(
        decode(&sequence(&trailing)).unwrap_err().kind(),
        X509OcspBasicResponseErrorKind::TrailingElement
    );
}

#[test]
fn rejects_too_many_or_invalid_certificates() {
    let values: Vec<Vec<u8>> = (0..=MAX_X509_OCSP_CERTIFICATES)
        .map(|index| certificate((index + 1) as u8))
        .collect();
    assert_eq!(
        decode(&basic_response(Some(&values))).unwrap_err().kind(),
        X509OcspBasicResponseErrorKind::TooManyCertificates
    );

    let invalid = [sequence(&[])];
    assert!(matches!(
        decode(&basic_response(Some(&invalid))).unwrap_err().kind(),
        X509OcspBasicResponseErrorKind::InvalidCertificate(_)
    ));
}

#[test]
fn nested_framing_offsets_are_rebased_to_the_whole_response() {
    let valid = basic_response(None);
    let response_offset = valid
        .windows(response_data(1).len())
        .position(|window| window == response_data(1))
        .unwrap();
    let invalid = sequence(&[
        vec![0x30, 0x03, 0x30, 0x02, 0x30],
        algorithm(SIGNATURE_OID),
        element(0x03, &[0]),
    ]);
    let error = decode(&invalid).unwrap_err();
    assert!(error.offset() >= response_offset);
    assert!(matches!(
        error.kind(),
        X509OcspBasicResponseErrorKind::InvalidResponseData(_)
    ));

    let cert = certificate(1);
    let cert_wrapper = certificates(std::slice::from_ref(&cert));
    let cert_offset = basic_response(Some(std::slice::from_ref(&cert)))
        .windows(cert_wrapper.len())
        .position(|window| window == cert_wrapper)
        .unwrap();
    let malformed = [vec![0x30, 0x03, 0x30, 0x02, 0x30]];
    let error = decode(&basic_response(Some(&malformed))).unwrap_err();
    assert!(error.offset() >= cert_offset);
    assert!(matches!(
        error.kind(),
        X509OcspBasicResponseErrorKind::InvalidCertificate(_)
    ));
}

#[test]
fn shared_limits_fail_closed_and_errors_are_payload_safe() {
    let encoded = basic_response(Some(&[certificate(1)]));
    let mut decoder = Asn1Decoder::new(Asn1Limits {
        max_total_elements: 8,
        ..Asn1Limits::default()
    });
    let root = decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_ocsp_basic_response(&mut decoder, root).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509OcspBasicResponseErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
            | X509OcspBasicResponseErrorKind::InvalidResponseData(_)
            | X509OcspBasicResponseErrorKind::InvalidCertificate(_)
    ));

    let malformed = sequence(&[
        response_data(1),
        algorithm(SIGNATURE_OID),
        element(0x03, &[0, 0xde, 0xad]),
        element(0xa0, &[0x30, 0x03, 0x30, 0x02, 0x30]),
    ]);
    let error = decode(&malformed).unwrap_err();
    assert!(!format!("{error:?}").contains("dead"));
    assert!(!error.to_string().contains("dead"));
    assert!(error.to_string().contains("byte"));
}

#[test]
fn every_outer_element_budget_cut_point_fails_closed_before_success() {
    let encoded = basic_response(Some(&[certificate(1)]));
    let mut failures = 0usize;
    let mut successes = 0usize;
    for max_total_elements in 1..=96 {
        let mut decoder = Asn1Decoder::new(Asn1Limits {
            max_total_elements,
            ..Asn1Limits::default()
        });
        match decoder.decode_exact(&encoded) {
            Ok(root) => match decode_x509_ocsp_basic_response(&mut decoder, root) {
                Ok(_) => successes += 1,
                Err(error) => {
                    assert!(error.offset() <= encoded.len());
                    failures += 1;
                }
            },
            Err(error) => {
                assert_eq!(error.kind(), Asn1ErrorKind::ElementLimitExceeded);
                failures += 1;
            }
        }
    }
    assert!(failures > 0);
    assert!(successes > 0);
}

#[test]
fn malformed_outer_framing_is_redacted() {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder
        .decode_exact(&[0x30, 0x03, 0x30, 0x02, 0x30])
        .unwrap();
    let error = decode_x509_ocsp_basic_response(&mut decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509OcspBasicResponseErrorKind::Structure(Asn1ErrorKind::Framing(
            DerErrorKind::TruncatedValue
        ))
    );
}
