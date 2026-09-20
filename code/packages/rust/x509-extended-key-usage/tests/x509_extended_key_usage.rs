use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_extended_key_usage::{
    decode_extended_key_usage, ExtendedKeyUsageErrorKind, EXTENDED_KEY_USAGE_OID,
    MAX_EXTENDED_KEY_PURPOSES,
};
use x509_extension::decode_x509_extension;

const EXTENDED_KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x25];
const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];
const SERVER_AUTH_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];
const CLIENT_AUTH_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x02];

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
    let contents: Vec<u8> = fields.iter().flatten().copied().collect();
    element(0x30, &contents)
}

fn extension(oid_contents: &[u8], critical: bool, payload: &[u8]) -> Vec<u8> {
    let mut fields = element(0x06, oid_contents);
    if critical {
        fields.extend_from_slice(&[0x01, 0x01, 0xff]);
    }
    fields.extend_from_slice(&element(0x04, payload));
    element(0x30, &fields)
}

fn purpose_list(purposes: &[Vec<u8>]) -> Vec<u8> {
    sequence(purposes)
}

fn decode_with_limits(
    encoded: &[u8],
    limits: Asn1Limits,
) -> Result<
    x509_extended_key_usage::ExtendedKeyUsage<'_>,
    x509_extended_key_usage::ExtendedKeyUsageError,
> {
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder
        .decode_exact(encoded)
        .expect("valid outer DER framing");
    let generic = decode_x509_extension(&mut decoder, root).expect("valid generic extension");
    decode_extended_key_usage(&mut decoder, generic)
}

fn decode(
    encoded: &[u8],
) -> Result<
    x509_extended_key_usage::ExtendedKeyUsage<'_>,
    x509_extended_key_usage::ExtendedKeyUsageError,
> {
    decode_with_limits(encoded, Asn1Limits::default())
}

#[test]
fn preserves_standard_and_unknown_purposes_in_wire_order() {
    assert_eq!(EXTENDED_KEY_USAGE_OID, &[2, 5, 29, 37]);
    let unknown = [0x2a, 0x03, 0x04];
    let payload = purpose_list(&[
        element(0x06, SERVER_AUTH_OID_DER),
        element(0x06, &unknown),
        element(0x06, CLIENT_AUTH_OID_DER),
    ]);
    let encoded = extension(EXTENDED_KEY_USAGE_OID_DER, false, &payload);
    let decoded = decode(&encoded).unwrap();

    assert_eq!(decoded.len(), 3);
    assert!(!decoded.is_empty());
    let values: Vec<_> = decoded.iter().collect();
    assert!(values[0].equals(&[1, 3, 6, 1, 5, 5, 7, 3, 1]));
    assert!(values[1].equals(&[1, 2, 3, 4]));
    assert!(values[2].equals(&[1, 3, 6, 1, 5, 5, 7, 3, 2]));
    assert!(decoded.contains(&[1, 2, 3, 4]));
    assert!(!decoded.contains(&[1, 2, 3, 5]));
    assert_eq!(decoded.get(3), None);
}

#[test]
fn preserves_duplicates_and_leaves_criticality_to_policy() {
    let purpose = element(0x06, SERVER_AUTH_OID_DER);
    let payload = purpose_list(&[purpose.clone(), purpose]);
    let encoded = extension(EXTENDED_KEY_USAGE_OID_DER, true, &payload);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.len(), 2);
    assert!(decoded
        .iter()
        .all(|oid| oid.equals(&[1, 3, 6, 1, 5, 5, 7, 3, 1])));
}

#[test]
fn accepts_exact_capacity() {
    let purposes: Vec<_> = (0..MAX_EXTENDED_KEY_PURPOSES)
        .map(|index| element(0x06, &[0x2a, 0x03, index as u8]))
        .collect();
    let payload = purpose_list(&purposes);
    let encoded = extension(EXTENDED_KEY_USAGE_OID_DER, false, &payload);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.len(), MAX_EXTENDED_KEY_PURPOSES);
    assert!(decoded
        .get(MAX_EXTENDED_KEY_PURPOSES - 1)
        .unwrap()
        .equals(&[1, 2, 3, (MAX_EXTENDED_KEY_PURPOSES - 1) as u64]));
}

#[test]
fn rejects_more_than_the_fixed_capacity() {
    let purposes: Vec<_> = (0..=MAX_EXTENDED_KEY_PURPOSES)
        .map(|index| element(0x06, &[0x2a, 0x03, index as u8]))
        .collect();
    let payload = purpose_list(&purposes);
    let last_offset = payload.len() - purposes.last().unwrap().len();
    let encoded = extension(EXTENDED_KEY_USAGE_OID_DER, false, &payload);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), ExtendedKeyUsageErrorKind::TooManyPurposes);
    assert_eq!(error.offset(), last_offset);
}

#[test]
fn rejects_wrong_extension_before_payload_access() {
    let encoded = extension(KEY_USAGE_OID_DER, false, &[0xde, 0xad]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), ExtendedKeyUsageErrorKind::WrongExtensionId);
    assert_eq!(error.offset(), 0);
}

#[test]
fn requires_one_exact_sequence() {
    let empty = purpose_list(&[]);
    let error = decode(&extension(EXTENDED_KEY_USAGE_OID_DER, false, &empty)).unwrap_err();
    assert_eq!(error.kind(), ExtendedKeyUsageErrorKind::Empty);
    assert_eq!(error.offset(), 2);

    for payload in [vec![], vec![0x31, 0], vec![0x30, 0, 0x05, 0]] {
        assert!(matches!(
            decode(&extension(EXTENDED_KEY_USAGE_OID_DER, false, &payload))
                .unwrap_err()
                .kind(),
            ExtendedKeyUsageErrorKind::Structure(_)
        ));
    }
}

#[test]
fn rejects_invalid_purpose_identifiers() {
    for (purpose, expected, expected_offset) in [
        (element(0x05, &[]), Asn1ErrorKind::UnexpectedTag, 2),
        (element(0x06, &[]), Asn1ErrorKind::EmptyObjectIdentifier, 4),
        (
            element(0x06, &[0x80]),
            Asn1ErrorKind::NonMinimalObjectIdentifier,
            4,
        ),
    ] {
        let payload = purpose_list(&[purpose]);
        let error = decode(&extension(EXTENDED_KEY_USAGE_OID_DER, false, &payload)).unwrap_err();
        assert_eq!(
            error.kind(),
            ExtendedKeyUsageErrorKind::InvalidPurpose(expected)
        );
        assert_eq!(error.offset(), expected_offset);
    }
}

#[test]
fn malformed_child_reports_payload_local_offset() {
    let mut contents = element(0x06, SERVER_AUTH_OID_DER);
    contents.extend_from_slice(&[0x06, 0x02, 0x2a]);
    let payload = element(0x30, &contents);
    let error = decode(&extension(EXTENDED_KEY_USAGE_OID_DER, false, &payload)).unwrap_err();
    assert!(matches!(
        error.kind(),
        ExtendedKeyUsageErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), payload.len());
}

#[test]
fn shared_depth_element_and_oid_limits_are_enforced() {
    let payload = purpose_list(&[element(0x06, SERVER_AUTH_OID_DER)]);
    let encoded = extension(EXTENDED_KEY_USAGE_OID_DER, false, &payload);

    let mut outer_decoder = Asn1Decoder::new(Asn1Limits::default());
    let outer = outer_decoder.decode_exact(&encoded).unwrap();
    let generic = decode_x509_extension(&mut outer_decoder, outer).unwrap();
    let depth_limits = Asn1Limits {
        max_depth: 1,
        ..Asn1Limits::default()
    };
    let mut depth_decoder = Asn1Decoder::new(depth_limits);
    let error = decode_extended_key_usage(&mut depth_decoder, generic).unwrap_err();
    assert_eq!(
        error.kind(),
        ExtendedKeyUsageErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    );
    assert_eq!(error.offset(), 0);

    let element_limits = Asn1Limits {
        max_total_elements: 4,
        ..Asn1Limits::default()
    };
    let error = decode_with_limits(&encoded, element_limits).unwrap_err();
    assert_eq!(
        error.kind(),
        ExtendedKeyUsageErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), 2);

    let oid_limits = Asn1Limits {
        max_oid_arcs: 4,
        ..Asn1Limits::default()
    };
    let error = decode_with_limits(&encoded, oid_limits).unwrap_err();
    assert_eq!(
        error.kind(),
        ExtendedKeyUsageErrorKind::InvalidPurpose(Asn1ErrorKind::OidArcLimitExceeded)
    );
    assert_eq!(error.offset(), 7);
}

#[test]
fn inner_value_limit_is_enforced() {
    let payload = purpose_list(&[element(0x06, SERVER_AUTH_OID_DER)]);
    let encoded = extension(EXTENDED_KEY_USAGE_OID_DER, false, &payload);

    let mut outer_decoder = Asn1Decoder::new(Asn1Limits::default());
    let outer = outer_decoder.decode_exact(&encoded).unwrap();
    let generic = decode_x509_extension(&mut outer_decoder, outer).unwrap();

    let mut limits = Asn1Limits::default();
    limits.der.max_value_len = 1;
    let mut inner_decoder = Asn1Decoder::new(limits);
    let error = decode_extended_key_usage(&mut inner_decoder, generic).unwrap_err();
    assert!(matches!(
        error.kind(),
        ExtendedKeyUsageErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), 1);
}

#[test]
fn errors_are_redacted() {
    let payload = purpose_list(&[element(0x06, &[0x80])]);
    let error = decode(&extension(EXTENDED_KEY_USAGE_OID_DER, false, &payload)).unwrap_err();
    assert!(!error.to_string().contains("80"));
    assert!(!error.to_string().contains("551d25"));
    assert!(error.to_string().contains("byte 4"));
}
