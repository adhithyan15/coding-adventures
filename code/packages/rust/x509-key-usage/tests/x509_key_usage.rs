use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_extension::decode_x509_extension;
use x509_key_usage::{decode_key_usage, KeyUsageBit, KeyUsageErrorKind, KEY_USAGE_OID};

const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];
const BASIC_CONSTRAINTS_OID_DER: &[u8] = &[0x55, 0x1d, 0x13];

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
) -> Result<x509_key_usage::KeyUsage, x509_key_usage::KeyUsageError> {
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder
        .decode_exact(encoded)
        .expect("valid outer DER framing");
    let generic = decode_x509_extension(&mut decoder, root).expect("valid generic extension");
    decode_key_usage(&mut decoder, generic)
}

fn decode(encoded: &[u8]) -> Result<x509_key_usage::KeyUsage, x509_key_usage::KeyUsageError> {
    decode_with_limits(encoded, Asn1Limits::default())
}

#[test]
fn decodes_each_defined_usage_bit() {
    assert_eq!(KEY_USAGE_OID, &[2, 5, 29, 15]);

    let cases = [
        (KeyUsageBit::DigitalSignature, vec![0x03, 0x02, 0x07, 0x80]),
        (KeyUsageBit::ContentCommitment, vec![0x03, 0x02, 0x06, 0x40]),
        (KeyUsageBit::KeyEncipherment, vec![0x03, 0x02, 0x05, 0x20]),
        (KeyUsageBit::DataEncipherment, vec![0x03, 0x02, 0x04, 0x10]),
        (KeyUsageBit::KeyAgreement, vec![0x03, 0x02, 0x03, 0x08]),
        (KeyUsageBit::KeyCertSign, vec![0x03, 0x02, 0x02, 0x04]),
        (KeyUsageBit::CrlSign, vec![0x03, 0x02, 0x01, 0x02]),
        (KeyUsageBit::EncipherOnly, vec![0x03, 0x02, 0x00, 0x01]),
        (
            KeyUsageBit::DecipherOnly,
            vec![0x03, 0x03, 0x07, 0x00, 0x80],
        ),
    ];

    for (usage, payload) in cases {
        let encoded = extension(KEY_USAGE_OID_DER, &payload);
        let decoded = decode(&encoded).unwrap();
        assert!(decoded.contains(usage));
        assert_eq!(decoded.bits(), usage as u16);
    }
}

#[test]
fn decodes_combinations_into_an_exact_normalized_mask() {
    let payload = [0x03, 0x03, 0x07, 0xff, 0x80];
    let decoded = decode(&extension(KEY_USAGE_OID_DER, &payload)).unwrap();
    assert_eq!(decoded.bits(), 0x01ff);
    for usage in [
        KeyUsageBit::DigitalSignature,
        KeyUsageBit::ContentCommitment,
        KeyUsageBit::KeyEncipherment,
        KeyUsageBit::DataEncipherment,
        KeyUsageBit::KeyAgreement,
        KeyUsageBit::KeyCertSign,
        KeyUsageBit::CrlSign,
        KeyUsageBit::EncipherOnly,
        KeyUsageBit::DecipherOnly,
    ] {
        assert!(decoded.contains(usage));
    }
}

#[test]
fn leaves_outer_criticality_to_certificate_policy() {
    let payload = [0x03, 0x02, 0x07, 0x80];
    let encoded = sequence(&[
        oid(KEY_USAGE_OID_DER),
        vec![0x01, 0x01, 0xff],
        octet_string(&payload),
    ]);
    assert!(decode(&encoded)
        .unwrap()
        .contains(KeyUsageBit::DigitalSignature));
}

#[test]
fn rejects_the_wrong_extension_identifier_before_payload_access() {
    let encoded = extension(BASIC_CONSTRAINTS_OID_DER, &[0xde, 0xad]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), KeyUsageErrorKind::WrongExtensionId);
    assert_eq!(error.offset(), 0);
}

#[test]
fn payload_must_be_one_exact_bit_string() {
    for payload in [
        vec![],
        vec![0x04, 0x02, 0x07, 0x80],
        vec![0x03, 0x02, 0x07, 0x80, 0x05, 0x00],
    ] {
        let error = decode(&extension(KEY_USAGE_OID_DER, &payload)).unwrap_err();
        assert!(matches!(
            error.kind(),
            KeyUsageErrorKind::Structure(_) | KeyUsageErrorKind::InvalidBitString(_)
        ));
    }
}

#[test]
fn rejects_empty_undefined_and_non_minimal_named_bits() {
    let empty = [0x03, 0x01, 0x00];
    let error = decode(&extension(KEY_USAGE_OID_DER, &empty)).unwrap_err();
    assert_eq!(error.kind(), KeyUsageErrorKind::Empty);
    assert_eq!(error.offset(), 2);

    let undefined_bit_nine = [0x03, 0x03, 0x06, 0x00, 0x40];
    let error = decode(&extension(KEY_USAGE_OID_DER, &undefined_bit_nine)).unwrap_err();
    assert_eq!(error.kind(), KeyUsageErrorKind::UndefinedBit);
    assert_eq!(error.offset(), 4);

    let trailing_named_zeroes = [0x03, 0x02, 0x00, 0x80];
    let error = decode(&extension(KEY_USAGE_OID_DER, &trailing_named_zeroes)).unwrap_err();
    assert_eq!(error.kind(), KeyUsageErrorKind::NonMinimalNamedBitString);
    assert_eq!(error.offset(), 3);
}

#[test]
fn rejects_malformed_unused_counts_and_padding() {
    for (payload, expected) in [
        (vec![0x03, 0x00], Asn1ErrorKind::MissingUnusedBitCount),
        (
            vec![0x03, 0x02, 0x08, 0x00],
            Asn1ErrorKind::InvalidUnusedBitCount,
        ),
        (
            vec![0x03, 0x02, 0x07, 0x81],
            Asn1ErrorKind::NonZeroBitPadding,
        ),
    ] {
        let error = decode(&extension(KEY_USAGE_OID_DER, &payload)).unwrap_err();
        assert_eq!(error.kind(), KeyUsageErrorKind::InvalidBitString(expected));
    }
}

#[test]
fn shared_element_limit_is_enforced() {
    let payload = [0x03, 0x02, 0x07, 0x80];
    let encoded = extension(KEY_USAGE_OID_DER, &payload);
    let limits = Asn1Limits {
        max_total_elements: 3,
        ..Asn1Limits::default()
    };
    let error = decode_with_limits(&encoded, limits).unwrap_err();
    assert_eq!(
        error.kind(),
        KeyUsageErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), 0);
}

#[test]
fn inner_value_limit_is_enforced() {
    let payload = [0x03, 0x02, 0x07, 0x80];
    let encoded = extension(KEY_USAGE_OID_DER, &payload);

    let mut outer_decoder = Asn1Decoder::new(Asn1Limits::default());
    let outer = outer_decoder.decode_exact(&encoded).unwrap();
    let generic = decode_x509_extension(&mut outer_decoder, outer).unwrap();

    let mut limits = Asn1Limits::default();
    limits.der.max_value_len = 1;
    let mut inner_decoder = Asn1Decoder::new(limits);
    let error = decode_key_usage(&mut inner_decoder, generic).unwrap_err();
    assert!(matches!(
        error.kind(),
        KeyUsageErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), 1);
}

#[test]
fn errors_are_redacted() {
    let payload = [0x03, 0x03, 0x06, 0x00, 0x40];
    let error = decode(&extension(KEY_USAGE_OID_DER, &payload)).unwrap_err();
    assert!(!error.to_string().contains("40"));
    assert!(error.to_string().contains("byte 4"));
}
