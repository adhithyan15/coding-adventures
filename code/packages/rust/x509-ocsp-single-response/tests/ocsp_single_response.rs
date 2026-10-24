use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_extensions::X509ExtensionsErrorKind;
use x509_ocsp_cert_id::X509OcspCertIdErrorKind;
use x509_ocsp_cert_status::{X509OcspCertStatus, X509OcspCertStatusErrorKind};
use x509_ocsp_single_response::{
    decode_x509_ocsp_single_response, X509OcspSingleResponseErrorKind,
};
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

fn cert_id() -> Vec<u8> {
    let algorithm = sequence(&[
        element(0x06, &[0x2b, 0x0e, 0x03, 0x02, 0x1a]),
        vec![0x05, 0x00],
    ]);
    sequence(&[
        algorithm,
        element(0x04, &[0x11; 20]),
        element(0x04, &[0x22; 20]),
        element(0x02, &[0x01, 0x23]),
    ])
}

fn time(value: &[u8]) -> Vec<u8> {
    element(0x18, value)
}

fn extension(oid_last_arc: u8, value: &[u8]) -> Vec<u8> {
    sequence(&[
        element(0x06, &[0x55, 0x1d, oid_last_arc]),
        element(0x04, value),
    ])
}

fn extensions(values: &[Vec<u8>]) -> Vec<u8> {
    sequence(values)
}

fn next_update(value: Vec<u8>) -> Vec<u8> {
    element(0xa0, &value)
}

fn single_extensions(value: Vec<u8>) -> Vec<u8> {
    element(0xa1, &value)
}

fn required_fields() -> Vec<Vec<u8>> {
    vec![cert_id(), vec![0x80, 0x00], time(b"20261024123456Z")]
}

fn response(optional: &[Vec<u8>]) -> Vec<u8> {
    let mut fields = required_fields();
    fields.extend_from_slice(optional);
    sequence(&fields)
}

fn decode(
    encoded: &[u8],
) -> Result<
    x509_ocsp_single_response::X509OcspSingleResponse<'_>,
    x509_ocsp_single_response::X509OcspSingleResponseError,
> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_ocsp_single_response(&mut decoder, root)
}

#[test]
fn decodes_the_required_single_response_fields() {
    let encoded = response(&[]);
    let decoded = decode(&encoded).unwrap();
    assert!(decoded
        .cert_id()
        .hash_algorithm()
        .algorithm()
        .equals(&[1, 3, 14, 3, 2, 26]));
    assert_eq!(decoded.cert_status(), X509OcspCertStatus::Good);
    assert_eq!(decoded.this_update().year(), 2026);
    assert_eq!(decoded.next_update(), None);
    assert_eq!(decoded.single_extensions(), None);
}

#[test]
fn decodes_each_ordered_optional_field() {
    let next = next_update(time(b"20261025123456Z"));
    let exts = single_extensions(extensions(&[extension(17, &[0xde, 0xad])]));
    for optional in [vec![next.clone()], vec![exts.clone()], vec![next, exts]] {
        let encoded = response(&optional);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.next_update().is_some(), optional[0][0] == 0xa0);
        assert_eq!(
            decoded.single_extensions().map(|values| values.len()),
            optional.last().filter(|value| value[0] == 0xa1).map(|_| 1)
        );
    }
}

#[test]
fn root_must_be_one_constructed_universal_sequence() {
    for encoded in [vec![0x10, 0], vec![0x31, 0]] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509OcspSingleResponseErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }
}

#[test]
fn every_required_field_is_enforced_in_schema_order() {
    let fields = required_fields();
    let expected = [
        X509OcspSingleResponseErrorKind::MissingCertId,
        X509OcspSingleResponseErrorKind::MissingCertStatus,
        X509OcspSingleResponseErrorKind::MissingThisUpdate,
    ];
    for count in 0..fields.len() {
        assert_eq!(
            decode(&sequence(&fields[..count])).unwrap_err().kind(),
            expected[count]
        );
    }
}

#[test]
fn nested_required_field_failures_keep_stable_categories() {
    let invalid_cert_id = sequence(&[]);
    let invalid_status = vec![0x82, 0x01, 0x00];
    let invalid_time = time(b"20260230123456Z");

    assert_eq!(
        decode(&sequence(&[
            invalid_cert_id,
            vec![0x80, 0],
            time(b"20261024123456Z")
        ]))
        .unwrap_err()
        .kind(),
        X509OcspSingleResponseErrorKind::InvalidCertId(
            X509OcspCertIdErrorKind::MissingHashAlgorithm
        )
    );
    assert_eq!(
        decode(&sequence(&[
            cert_id(),
            invalid_status,
            time(b"20261024123456Z")
        ]))
        .unwrap_err()
        .kind(),
        X509OcspSingleResponseErrorKind::InvalidCertStatus(X509OcspCertStatusErrorKind::Structure(
            Asn1ErrorKind::NonEmptyNull
        ))
    );
    assert_eq!(
        decode(&sequence(&[cert_id(), vec![0x80, 0], invalid_time]))
            .unwrap_err()
            .kind(),
        X509OcspSingleResponseErrorKind::InvalidThisUpdate(X509TimeErrorKind::InvalidDay)
    );
}

#[test]
fn optional_wrappers_must_be_constructed_explicit_and_exact() {
    let invalid = [
        vec![0x80, 0],
        element(0xa0, &[]),
        element(0xa0, &[0x18, 0x00, 0x05, 0x00]),
        element(0x81, &[]),
        element(0xa1, &[]),
        element(0xa1, &[0x30, 0x00, 0x05, 0x00]),
    ];
    for value in invalid {
        assert!(decode(&response(&[value])).is_err());
    }
}

#[test]
fn next_update_uses_the_strict_ocsp_generalized_time_profile() {
    let encoded = response(&[next_update(time(b"20261324123456Z"))]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509OcspSingleResponseErrorKind::InvalidNextUpdate(X509TimeErrorKind::InvalidMonth)
    );
}

#[test]
fn extensions_remain_generic_bounded_and_unique() {
    let empty = response(&[single_extensions(extensions(&[]))]);
    assert_eq!(
        decode(&empty).unwrap_err().kind(),
        X509OcspSingleResponseErrorKind::InvalidSingleExtensions(X509ExtensionsErrorKind::Empty)
    );

    let duplicate = response(&[single_extensions(extensions(&[
        extension(17, &[0xaa]),
        extension(17, &[0xbb]),
    ]))]);
    assert_eq!(
        decode(&duplicate).unwrap_err().kind(),
        X509OcspSingleResponseErrorKind::InvalidSingleExtensions(
            X509ExtensionsErrorKind::DuplicateExtensionId
        )
    );
}

#[test]
fn optional_fields_cannot_be_reordered_duplicated_or_extended() {
    let next = next_update(time(b"20261025123456Z"));
    let exts = single_extensions(extensions(&[extension(17, &[])]));
    for optional in [
        vec![exts.clone(), next.clone()],
        vec![next.clone(), next.clone()],
        vec![exts.clone(), exts.clone()],
        vec![element(0xa2, &[])],
    ] {
        assert_eq!(
            decode(&response(&optional)).unwrap_err().kind(),
            X509OcspSingleResponseErrorKind::UnexpectedOptionalField
        );
    }
    assert_eq!(
        decode(&response(&[next, exts, vec![0x05, 0x00]]))
            .unwrap_err()
            .kind(),
        X509OcspSingleResponseErrorKind::TrailingElement
    );
}

#[test]
fn malformed_children_report_offsets_in_the_whole_response() {
    let mut contents: Vec<u8> = required_fields().iter().flatten().copied().collect();
    contents.extend_from_slice(&[0xa0, 0x02, 0x18]);
    let encoded = element(0x30, &contents);
    let error = decode(&encoded).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509OcspSingleResponseErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), encoded.len());
}

#[test]
fn malformed_fields_are_rebased_from_each_cursor_position() {
    let next = next_update(time(b"20261025123456Z"));
    let exts = single_extensions(extensions(&[extension(17, &[])]));
    for optional in [
        vec![next.clone(), vec![0xa1, 0x02, 0x30]],
        vec![next.clone(), exts.clone(), vec![0x05, 0x02, 0x00]],
        vec![exts, vec![0xa0, 0x02, 0x18]],
    ] {
        let encoded = response(&optional);
        let error = decode(&encoded).unwrap_err();
        assert!(matches!(
            error.kind(),
            X509OcspSingleResponseErrorKind::Structure(Asn1ErrorKind::Framing(_))
        ));
        assert_eq!(error.offset(), encoded.len());
    }

    let encoded = element(0x30, &[0x30, 0x02, 0x06]);
    assert!(matches!(
        decode(&encoded).unwrap_err().kind(),
        X509OcspSingleResponseErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
}

#[test]
fn optional_wrapper_budget_errors_are_rebased_without_payloads() {
    let encoded = response(&[next_update(time(b"20261025123456Z"))]);
    let limits = Asn1Limits {
        max_total_elements: 11,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_ocsp_single_response(&mut decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509OcspSingleResponseErrorKind::InvalidNextUpdateWrapper(
            Asn1ErrorKind::ElementLimitExceeded
        )
    );
    assert!(!error.to_string().contains("20261025"));
}

#[test]
fn shared_depth_element_and_oid_limits_are_enforced() {
    let encoded = response(&[single_extensions(extensions(&[extension(17, &[])]))]);
    for limits in [
        Asn1Limits {
            max_depth: 2,
            ..Asn1Limits::default()
        },
        Asn1Limits {
            max_total_elements: 6,
            ..Asn1Limits::default()
        },
        Asn1Limits {
            max_oid_arcs: 2,
            ..Asn1Limits::default()
        },
    ] {
        let mut decoder = Asn1Decoder::new(limits);
        let root = decoder.decode_exact(&encoded).unwrap();
        assert!(decode_x509_ocsp_single_response(&mut decoder, root).is_err());
    }
}

#[test]
fn debug_and_errors_do_not_disclose_identifier_or_extension_bytes() {
    let encoded = response(&[single_extensions(extensions(&[extension(
        17,
        &[0xde, 0xad],
    )]))]);
    let decoded = decode(&encoded).unwrap();
    let debug = format!("{decoded:?}");
    assert!(!debug.contains("dead"));
    assert!(!debug.contains("111111"));
    assert!(!debug.contains("222222"));

    let invalid = response(&[single_extensions(extensions(&[
        extension(17, &[0xde, 0xad]),
        extension(17, &[]),
    ]))]);
    let error = decode(&invalid).unwrap_err().to_string();
    assert!(!error.contains("dead"));
    assert!(!error.contains("551d11"));
    assert!(error.contains("byte"));
}
