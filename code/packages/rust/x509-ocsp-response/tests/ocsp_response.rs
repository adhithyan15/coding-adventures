use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use der_tlv::DerErrorKind;
use x509_ocsp_response::{
    decode_x509_ocsp_response, X509OcspResponseErrorKind, X509OcspResponseStatus,
};

const BASIC_RESPONSE_OID: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01, 0x01];

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

fn status(value: &[u8]) -> Vec<u8> {
    element(0x0a, value)
}

fn response_bytes(oid: &[u8], response: &[u8]) -> Vec<u8> {
    element(
        0xa0,
        &sequence(&[element(0x06, oid), element(0x04, response)]),
    )
}

fn response(status_value: u8, body: Option<Vec<u8>>) -> Vec<u8> {
    let mut fields = vec![status(&[status_value])];
    if let Some(body) = body {
        fields.push(body);
    }
    sequence(&fields)
}

fn successful(response_body: &[u8]) -> Vec<u8> {
    response(0, Some(response_bytes(BASIC_RESPONSE_OID, response_body)))
}

fn decode(
    encoded: &[u8],
) -> Result<x509_ocsp_response::X509OcspResponse<'_>, x509_ocsp_response::X509OcspResponseError> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_ocsp_response(&mut decoder, root)
}

#[test]
fn decodes_success_and_preserves_generic_response_bytes() {
    let inner = sequence(&[element(0x02, &[1])]);
    let encoded = successful(&inner);
    let decoded = decode(&encoded).unwrap();
    let bytes = decoded.response_bytes().unwrap();
    let response_bytes_encoding = response_bytes(BASIC_RESPONSE_OID, &inner);

    assert_eq!(decoded.encoded(), encoded);
    assert_eq!(decoded.status(), X509OcspResponseStatus::Successful);
    assert_eq!(decoded.status().as_u8(), 0);
    assert_eq!(bytes.encoded(), &response_bytes_encoding[2..]);
    assert!(bytes
        .response_type()
        .equals(&[1, 3, 6, 1, 5, 5, 7, 48, 1, 1]));
    assert_eq!(bytes.response(), inner);

    let bytes_debug = format!("{bytes:?}");
    assert!(bytes_debug.contains("response_type_arc_count"));
    assert!(bytes_debug.contains("response_octet_count"));
    assert!(!bytes_debug.contains("020101"));

    let debug = format!("{decoded:?}");
    assert!(debug.contains("response_octet_count"));
    assert!(!debug.contains("020101"));
}

#[test]
fn decodes_every_assigned_error_status_without_response_bytes() {
    for (value, expected) in [
        (1, X509OcspResponseStatus::MalformedRequest),
        (2, X509OcspResponseStatus::InternalError),
        (3, X509OcspResponseStatus::TryLater),
        (5, X509OcspResponseStatus::SigRequired),
        (6, X509OcspResponseStatus::Unauthorized),
    ] {
        let encoded = response(value, None);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.status(), expected);
        assert_eq!(decoded.status().as_u8(), value);
        assert!(decoded.response_bytes().is_none());
    }
}

#[test]
fn response_type_is_data_not_an_enabled_registry_entry() {
    let private_oid = &[0x2a, 0x03, 0x04];
    let encoded = response(0, Some(response_bytes(private_oid, &[])));
    let decoded = decode(&encoded).unwrap();
    let bytes = decoded.response_bytes().unwrap();

    assert!(bytes.response_type().equals(&[1, 2, 3, 4]));
    assert!(bytes.response().is_empty());
}

#[test]
fn status_and_response_bytes_presence_are_bound_together() {
    assert_eq!(
        decode(&response(0, None)).unwrap_err().kind(),
        X509OcspResponseErrorKind::MissingResponseBytes
    );

    for value in [1, 2, 3, 5, 6] {
        assert_eq!(
            decode(&response(
                value,
                Some(response_bytes(BASIC_RESPONSE_OID, &[1]))
            ))
            .unwrap_err()
            .kind(),
            X509OcspResponseErrorKind::UnexpectedResponseBytes
        );
    }

    assert_eq!(
        decode(&response(1, Some(element(0x05, &[]))))
            .unwrap_err()
            .kind(),
        X509OcspResponseErrorKind::UnexpectedOptionalField
    );
}

#[test]
fn rejects_unassigned_negative_nonminimal_and_oversized_statuses() {
    assert_eq!(
        decode(&sequence(&[status(&[4])])).unwrap_err().kind(),
        X509OcspResponseErrorKind::UnsupportedStatus
    );
    assert_eq!(
        decode(&sequence(&[status(&[7])])).unwrap_err().kind(),
        X509OcspResponseErrorKind::UnsupportedStatus
    );

    for invalid in [vec![0x80], vec![0x00, 0x01], vec![1; 9]] {
        assert!(matches!(
            decode(&sequence(&[status(&invalid)])).unwrap_err().kind(),
            X509OcspResponseErrorKind::InvalidStatus(_)
        ));
    }
}

#[test]
fn root_and_required_status_are_enforced_in_schema_order() {
    for encoded in [vec![0x10, 0], vec![0x31, 0]] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509OcspResponseErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }
    assert_eq!(
        decode(&sequence(&[])).unwrap_err().kind(),
        X509OcspResponseErrorKind::MissingStatus
    );
    assert!(matches!(
        decode(&sequence(&[element(0x02, &[0])]))
            .unwrap_err()
            .kind(),
        X509OcspResponseErrorKind::InvalidStatus(_)
    ));
}

#[test]
fn response_bytes_wrapper_must_be_exact_and_terminal() {
    for wrapper in [
        element(0xa1, &sequence(&[])),
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
        assert!(matches!(
            decode(&response(0, Some(wrapper))).unwrap_err().kind(),
            X509OcspResponseErrorKind::UnexpectedOptionalField
                | X509OcspResponseErrorKind::InvalidResponseBytesWrapper(_)
                | X509OcspResponseErrorKind::InvalidResponseBytes(_)
        ));
    }

    let encoded = sequence(&[
        status(&[0]),
        response_bytes(BASIC_RESPONSE_OID, &[1]),
        element(0x05, &[]),
    ]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509OcspResponseErrorKind::TrailingElement
    );
}

#[test]
fn response_bytes_fields_are_required_ordered_and_exclusive() {
    let empty = element(0xa0, &sequence(&[]));
    assert_eq!(
        decode(&response(0, Some(empty))).unwrap_err().kind(),
        X509OcspResponseErrorKind::MissingResponseType
    );

    let only_oid = element(0xa0, &sequence(&[element(0x06, BASIC_RESPONSE_OID)]));
    assert_eq!(
        decode(&response(0, Some(only_oid))).unwrap_err().kind(),
        X509OcspResponseErrorKind::MissingResponse
    );

    let wrong_oid = element(0xa0, &sequence(&[element(0x04, &[1]), element(0x04, &[2])]));
    assert!(matches!(
        decode(&response(0, Some(wrong_oid))).unwrap_err().kind(),
        X509OcspResponseErrorKind::InvalidResponseType(_)
    ));

    let wrong_body = element(
        0xa0,
        &sequence(&[element(0x06, BASIC_RESPONSE_OID), element(0x06, &[0x2a])]),
    );
    assert!(matches!(
        decode(&response(0, Some(wrong_body))).unwrap_err().kind(),
        X509OcspResponseErrorKind::InvalidResponse(_)
    ));

    let trailing = element(
        0xa0,
        &sequence(&[
            element(0x06, BASIC_RESPONSE_OID),
            element(0x04, &[1]),
            element(0x05, &[]),
        ]),
    );
    assert_eq!(
        decode(&response(0, Some(trailing))).unwrap_err().kind(),
        X509OcspResponseErrorKind::TrailingResponseBytesElement
    );
}

#[test]
fn nested_framing_offsets_are_rebased_to_the_complete_response() {
    let malformed_fields = [element(0x06, BASIC_RESPONSE_OID), vec![0x04, 0x02, 0x01]];
    let wrapper = element(0xa0, &sequence(&malformed_fields));
    let encoded = response(0, Some(wrapper.clone()));
    let wrapper_offset = encoded
        .windows(wrapper.len())
        .position(|window| window == wrapper)
        .unwrap();
    let error = decode(&encoded).unwrap_err();

    assert!(error.offset() >= wrapper_offset);
    assert!(
        matches!(
            error.kind(),
            X509OcspResponseErrorKind::InvalidResponseBytes(Asn1ErrorKind::Framing(
                DerErrorKind::TruncatedValue
            ))
        ),
        "{error:?}"
    );

    let malformed_inner_trailing = element(
        0xa0,
        &sequence(&[
            element(0x06, BASIC_RESPONSE_OID),
            element(0x04, &[1]),
            vec![0x05, 0x01],
        ]),
    );
    assert!(matches!(
        decode(&response(0, Some(malformed_inner_trailing)))
            .unwrap_err()
            .kind(),
        X509OcspResponseErrorKind::InvalidResponseBytes(Asn1ErrorKind::Framing(
            DerErrorKind::TruncatedValue
        ))
    ));

    let malformed_outer_trailing = sequence(&[
        status(&[0]),
        response_bytes(BASIC_RESPONSE_OID, &[1]),
        vec![0x05, 0x01],
    ]);
    assert!(matches!(
        decode(&malformed_outer_trailing).unwrap_err().kind(),
        X509OcspResponseErrorKind::Structure(Asn1ErrorKind::Framing(DerErrorKind::TruncatedValue))
    ));
}

#[test]
fn shared_limits_fail_closed_and_errors_are_payload_safe() {
    let encoded = successful(&[0xde, 0xad, 0xbe, 0xef]);
    let mut decoder = Asn1Decoder::new(Asn1Limits {
        max_oid_arcs: 3,
        ..Asn1Limits::default()
    });
    let root = decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_ocsp_response(&mut decoder, root).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509OcspResponseErrorKind::InvalidResponseType(_)
    ));
    assert!(!format!("{error:?}").contains("deadbeef"));
    assert!(!error.to_string().contains("deadbeef"));
    assert!(error.to_string().contains("byte"));
}

#[test]
fn every_outer_element_budget_cut_point_fails_closed_before_success() {
    let encoded = successful(&[1, 2, 3]);
    let mut failures = 0usize;
    let mut successes = 0usize;
    for max_total_elements in 1..=12 {
        let mut decoder = Asn1Decoder::new(Asn1Limits {
            max_total_elements,
            ..Asn1Limits::default()
        });
        match decoder.decode_exact(&encoded) {
            Ok(root) => match decode_x509_ocsp_response(&mut decoder, root) {
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
