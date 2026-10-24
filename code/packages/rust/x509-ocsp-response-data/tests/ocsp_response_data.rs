use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_extensions::X509ExtensionsErrorKind;
use x509_ocsp_response_data::{
    decode_x509_ocsp_response_data, X509OcspResponderId, X509OcspResponseDataErrorKind,
    X509OcspResponseDataVersion, MAX_X509_OCSP_SINGLE_RESPONSES,
};
use x509_ocsp_single_response::X509OcspSingleResponseErrorKind;
use x509_time::X509TimeErrorKind;

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

fn time(value: &[u8]) -> Vec<u8> {
    element(0x18, value)
}

fn cert_id(serial: u8) -> Vec<u8> {
    let algorithm = sequence(&[
        element(0x06, &[0x2b, 0x0e, 0x03, 0x02, 0x1a]),
        vec![0x05, 0x00],
    ]);
    sequence(&[
        algorithm,
        element(0x04, &[0x11; 20]),
        element(0x04, &[0x22; 20]),
        element(0x02, &[serial]),
    ])
}

fn single_response(serial: u8) -> Vec<u8> {
    sequence(&[cert_id(serial), vec![0x80, 0x00], time(b"20261024123456Z")])
}

fn responder_by_name() -> Vec<u8> {
    let attribute = sequence(&[
        element(0x06, &[0x55, 0x04, 0x03]),
        element(0x13, b"OCSP responder"),
    ]);
    let rdn = element(0x31, &attribute);
    element(0xa1, &sequence(&[rdn]))
}

fn responder_by_key(value: &[u8]) -> Vec<u8> {
    element(0xa2, &element(0x04, value))
}

fn responses(values: &[Vec<u8>]) -> Vec<u8> {
    sequence(values)
}

fn extension(oid_last_arc: u8, value: &[u8]) -> Vec<u8> {
    sequence(&[
        element(0x06, &[0x55, 0x1d, oid_last_arc]),
        element(0x04, value),
    ])
}

fn response_extensions(values: &[Vec<u8>]) -> Vec<u8> {
    element(0xa1, &sequence(values))
}

fn response_data(responder: Vec<u8>, entries: &[Vec<u8>], optional: &[Vec<u8>]) -> Vec<u8> {
    let mut fields = vec![responder, time(b"20261024120000Z"), responses(entries)];
    fields.extend_from_slice(optional);
    sequence(&fields)
}

fn decode(
    encoded: &[u8],
) -> Result<
    x509_ocsp_response_data::X509OcspResponseData<'_>,
    x509_ocsp_response_data::X509OcspResponseDataError,
> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_ocsp_response_data(&mut decoder, root)
}

#[test]
fn decodes_default_v1_by_name_and_one_response() {
    let encoded = response_data(responder_by_name(), &[single_response(1)], &[]);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.version(), X509OcspResponseDataVersion::V1);
    match decoded.responder_id() {
        X509OcspResponderId::ByName(name) => {
            assert_eq!(name.rdn_count(), 1);
            assert_eq!(name.attribute_count(), 1);
        }
        X509OcspResponderId::ByKey(_) => panic!("expected byName"),
    }
    assert_eq!(decoded.produced_at().year(), 2026);
    assert_eq!(decoded.len(), 1);
    assert!(!decoded.is_empty());
    assert_eq!(
        decoded
            .get(0)
            .unwrap()
            .cert_id()
            .serial_number()
            .magnitude(),
        &[1]
    );
    assert_eq!(decoded.iter().count(), 1);
    assert_eq!(decoded.response_extensions(), None);
    assert_eq!(decoded.version().as_u8(), 0);
    let responder_debug = format!("{:?}", decoded.responder_id());
    assert!(responder_debug.contains("rdn_count"));
    assert!(responder_debug.contains("attribute_count"));
}

#[test]
fn decodes_by_key_multiple_responses_and_generic_extensions() {
    let encoded = response_data(
        responder_by_key(&[0xde, 0xad, 0xbe, 0xef]),
        &[single_response(1), single_response(2)],
        &[response_extensions(&[extension(17, &[0xca, 0xfe])])],
    );
    let decoded = decode(&encoded).unwrap();
    assert_eq!(
        decoded.responder_id(),
        &X509OcspResponderId::ByKey(&[0xde, 0xad, 0xbe, 0xef])
    );
    assert_eq!(decoded.len(), 2);
    assert_eq!(
        decoded
            .get(1)
            .unwrap()
            .cert_id()
            .serial_number()
            .magnitude(),
        &[2]
    );
    assert_eq!(
        decoded.response_extensions().map(|values| values.len()),
        Some(1)
    );
}

#[test]
fn root_must_be_one_constructed_universal_sequence() {
    for encoded in [vec![0x10, 0], vec![0x31, 0]] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509OcspResponseDataErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }
}

#[test]
fn required_fields_are_enforced_in_schema_order() {
    let responder = responder_by_name();
    let produced = time(b"20261024120000Z");
    assert_eq!(
        decode(&sequence(&[])).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::MissingResponderId
    );
    assert_eq!(
        decode(&sequence(std::slice::from_ref(&responder)))
            .unwrap_err()
            .kind(),
        X509OcspResponseDataErrorKind::MissingProducedAt
    );
    assert_eq!(
        decode(&sequence(&[responder, produced]))
            .unwrap_err()
            .kind(),
        X509OcspResponseDataErrorKind::MissingResponses
    );
}

#[test]
fn default_version_must_be_omitted_and_other_versions_are_unsupported() {
    let required = [
        responder_by_name(),
        time(b"20261024120000Z"),
        responses(&[single_response(1)]),
    ];
    for (version, expected) in [
        (0, X509OcspResponseDataErrorKind::ExplicitDefaultVersion),
        (1, X509OcspResponseDataErrorKind::UnsupportedVersion),
    ] {
        let mut fields = vec![element(0xa0, &element(0x02, &[version]))];
        fields.extend_from_slice(&required);
        assert_eq!(decode(&sequence(&fields)).unwrap_err().kind(), expected);
    }
    let mut malformed = vec![element(0xa0, &[])];
    malformed.extend_from_slice(&required);
    assert!(matches!(
        decode(&sequence(&malformed)).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::InvalidVersionWrapper(_)
    ));

    for (wrapper, expected) in [
        (
            vec![0x80, 0],
            X509OcspResponseDataErrorKind::InvalidVersionWrapper(Asn1ErrorKind::UnexpectedTag),
        ),
        (
            element(0xa0, &element(0x05, &[])),
            X509OcspResponseDataErrorKind::InvalidVersionInteger(Asn1ErrorKind::UnexpectedTag),
        ),
        (
            element(0xa0, &element(0x02, &[0xff])),
            X509OcspResponseDataErrorKind::InvalidVersionInteger(Asn1ErrorKind::NegativeInteger),
        ),
    ] {
        let mut fields = vec![wrapper];
        fields.extend_from_slice(&required);
        assert_eq!(decode(&sequence(&fields)).unwrap_err().kind(), expected);
    }
}

#[test]
fn responder_id_requires_one_exact_explicit_name_or_key_hash() {
    for responder in [
        vec![0x81, 0],
        element(0xa1, &[]),
        element(0xa1, &element(0x04, &[])),
        element(0xa2, &[]),
        element(0xa2, &element(0x05, &[])),
        element(0xa3, &[]),
    ] {
        assert!(decode(&response_data(responder, &[single_response(1)], &[])).is_err());
    }
}

#[test]
fn produced_at_uses_the_strict_ocsp_generalized_time_profile() {
    let encoded = sequence(&[
        responder_by_name(),
        time(b"20261324120000Z"),
        responses(&[single_response(1)]),
    ]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::InvalidProducedAt(X509TimeErrorKind::InvalidMonth)
    );
}

#[test]
fn responses_are_non_empty_and_fixed_capacity() {
    let empty = response_data(responder_by_name(), &[], &[]);
    assert_eq!(
        decode(&empty).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::EmptyResponses
    );

    let too_many = vec![single_response(1); MAX_X509_OCSP_SINGLE_RESPONSES + 1];
    let encoded = response_data(responder_by_name(), &too_many, &[]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::TooManyResponses
    );

    let wrong_tag = sequence(&[
        responder_by_name(),
        time(b"20261024120000Z"),
        element(0x05, &[]),
    ]);
    assert_eq!(
        decode(&wrong_tag).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::InvalidResponses(Asn1ErrorKind::UnexpectedTag)
    );
}

#[test]
fn invalid_single_responses_keep_their_stable_category() {
    let encoded = response_data(responder_by_name(), &[sequence(&[])], &[]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::InvalidSingleResponse(
            X509OcspSingleResponseErrorKind::MissingCertId
        )
    );
}

#[test]
fn response_extensions_remain_generic_non_empty_and_unique() {
    let empty = response_data(
        responder_by_name(),
        &[single_response(1)],
        &[response_extensions(&[])],
    );
    assert_eq!(
        decode(&empty).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::InvalidResponseExtensions(X509ExtensionsErrorKind::Empty)
    );

    let duplicate = response_data(
        responder_by_name(),
        &[single_response(1)],
        &[response_extensions(&[
            extension(17, &[0xaa]),
            extension(17, &[0xbb]),
        ])],
    );
    assert_eq!(
        decode(&duplicate).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::InvalidResponseExtensions(
            X509ExtensionsErrorKind::DuplicateExtensionId
        )
    );
}

#[test]
fn response_extensions_are_optional_ordered_and_terminal() {
    let ext = response_extensions(&[extension(17, &[])]);
    let required = [
        responder_by_name(),
        time(b"20261024120000Z"),
        responses(&[single_response(1)]),
    ];
    for trailing in [vec![0x05, 0], ext.clone(), element(0xa2, &[])] {
        let mut fields = required.to_vec();
        fields.push(ext.clone());
        fields.push(trailing);
        assert_eq!(
            decode(&sequence(&fields)).unwrap_err().kind(),
            X509OcspResponseDataErrorKind::TrailingElement
        );
    }

    let mut unknown = required.to_vec();
    unknown.push(element(0xa2, &[]));
    assert_eq!(
        decode(&sequence(&unknown)).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::UnexpectedOptionalField
    );
}

#[test]
fn malformed_children_report_offsets_in_the_whole_response() {
    let mut value = Vec::new();
    value.extend_from_slice(&responder_by_name());
    value.extend_from_slice(&time(b"20261024120000Z"));
    value.extend_from_slice(&[0x30, 0x02, 0x30]);
    let encoded = element(0x30, &value);
    let error = decode(&encoded).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509OcspResponseDataErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), encoded.len());

    let encoded = sequence(&[
        responder_by_name(),
        time(b"20261024120000Z"),
        element(0x30, &[0x30, 0x02, 0x30]),
    ]);
    let error = decode(&encoded).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509OcspResponseDataErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), encoded.len());
}

#[test]
fn malformed_optional_and_trailing_fields_are_rebased() {
    let required = [
        responder_by_name(),
        time(b"20261024120000Z"),
        responses(&[single_response(1)]),
    ];

    let mut malformed_optional = required.to_vec();
    malformed_optional.push(vec![0xa1, 0x02, 0x30]);
    let encoded = sequence(&malformed_optional);
    let error = decode(&encoded).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509OcspResponseDataErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), encoded.len());

    let mut empty_wrapper = required.to_vec();
    empty_wrapper.push(element(0xa1, &[]));
    assert!(matches!(
        decode(&sequence(&empty_wrapper)).unwrap_err().kind(),
        X509OcspResponseDataErrorKind::InvalidResponseExtensionsWrapper(Asn1ErrorKind::Framing(_))
    ));

    let mut malformed_trailing = required.to_vec();
    malformed_trailing.push(response_extensions(&[extension(17, &[])]));
    malformed_trailing.push(vec![0x05, 0x02, 0x00]);
    let encoded = sequence(&malformed_trailing);
    let error = decode(&encoded).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509OcspResponseDataErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), encoded.len());
}

#[test]
fn shared_depth_element_oid_and_extension_limits_are_enforced() {
    let encoded = response_data(
        responder_by_name(),
        &[single_response(1)],
        &[response_extensions(&[extension(17, &[])])],
    );
    for limits in [
        Asn1Limits {
            max_depth: 2,
            ..Asn1Limits::default()
        },
        Asn1Limits {
            max_total_elements: 7,
            ..Asn1Limits::default()
        },
        Asn1Limits {
            max_oid_arcs: 2,
            ..Asn1Limits::default()
        },
    ] {
        let mut decoder = Asn1Decoder::new(limits);
        let root = decoder.decode_exact(&encoded).unwrap();
        assert!(decode_x509_ocsp_response_data(&mut decoder, root).is_err());
    }
}

#[test]
fn debug_and_errors_do_not_disclose_responder_or_response_bytes() {
    let encoded = response_data(
        responder_by_key(&[0xde, 0xad, 0xbe, 0xef]),
        &[single_response(1)],
        &[response_extensions(&[extension(17, &[0xca, 0xfe])])],
    );
    let decoded = decode(&encoded).unwrap();
    let debug = format!("{decoded:?}");
    assert!(!debug.contains("deadbeef"));
    assert!(!debug.contains("cafe"));
    assert!(!debug.contains("111111"));

    let invalid = response_data(
        responder_by_key(&[0xde, 0xad, 0xbe, 0xef]),
        &[single_response(1)],
        &[response_extensions(&[
            extension(17, &[0xca, 0xfe]),
            extension(17, &[]),
        ])],
    );
    let error = decode(&invalid).unwrap_err().to_string();
    assert!(!error.contains("deadbeef"));
    assert!(!error.contains("cafe"));
    assert!(error.contains("byte"));
}
