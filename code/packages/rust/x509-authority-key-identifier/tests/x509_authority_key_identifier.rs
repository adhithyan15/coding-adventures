use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_authority_key_identifier::{
    decode_authority_key_identifier, AuthorityKeyIdentifierErrorKind, AuthorityKeyIdentifierPolicy,
    AuthorityKeyIdentifierPolicyErrorKind, AUTHORITY_KEY_IDENTIFIER_OID,
};
use x509_extension::decode_x509_extension;

const AUTHORITY_KEY_IDENTIFIER_OID_DER: &[u8] = &[0x55, 0x1d, 0x23];
const SUBJECT_KEY_IDENTIFIER_OID_DER: &[u8] = &[0x55, 0x1d, 0x0e];

fn tlv(tag: u8, contents: &[u8]) -> Vec<u8> {
    assert!(contents.len() < 128);
    let mut encoded = vec![tag, contents.len() as u8];
    encoded.extend_from_slice(contents);
    encoded
}
fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    let value: Vec<u8> = fields.iter().flatten().copied().collect();
    tlv(0x30, &value)
}
fn extension(oid_contents: &[u8], critical: bool, payload: &[u8]) -> Vec<u8> {
    let mut fields = vec![tlv(0x06, oid_contents)];
    if critical {
        fields.push(vec![0x01, 0x01, 0xff]);
    }
    fields.push(tlv(0x04, payload));
    sequence(&fields)
}
fn key_only(identifier: &[u8]) -> Vec<u8> {
    sequence(&[tlv(0x80, identifier)])
}

fn decode(
    encoded: &[u8],
    maximum_identifier_bytes: usize,
) -> Result<
    x509_authority_key_identifier::AuthorityKeyIdentifier<'_>,
    x509_authority_key_identifier::AuthorityKeyIdentifierError,
> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder
        .decode_exact(encoded)
        .expect("valid outer DER framing");
    let generic = decode_x509_extension(&mut decoder, root).expect("valid generic extension");
    decode_authority_key_identifier(
        &mut decoder,
        generic,
        AuthorityKeyIdentifierPolicy::new(maximum_identifier_bytes).unwrap(),
    )
}

#[test]
fn decodes_exact_noncritical_key_identifier_only_profile() {
    assert_eq!(AUTHORITY_KEY_IDENTIFIER_OID, &[2, 5, 29, 35]);
    let identifier: Vec<u8> = (0u8..20).collect();
    let encoded = extension(
        AUTHORITY_KEY_IDENTIFIER_OID_DER,
        false,
        &key_only(&identifier),
    );
    assert_eq!(decode(&encoded, 20).unwrap().key_identifier(), identifier);
}

#[test]
fn rejects_wrong_oid_and_criticality_before_payload_decoding() {
    let wrong = extension(SUBJECT_KEY_IDENTIFIER_OID_DER, false, &[0xde, 0xad]);
    let error = decode(&wrong, 64).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityKeyIdentifierErrorKind::WrongExtensionId
    );
    assert_eq!(error.offset(), 0);

    let critical = extension(AUTHORITY_KEY_IDENTIFIER_OID_DER, true, &[0xde, 0xad]);
    let error = decode(&critical, 64).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityKeyIdentifierErrorKind::CriticalExtension
    );
    assert_eq!(error.offset(), 0);
}

#[test]
fn requires_one_exact_sequence() {
    for payload in [vec![], vec![0x31, 0x00], vec![0x30, 0x00, 0x05, 0x00]] {
        let error = decode(
            &extension(AUTHORITY_KEY_IDENTIFIER_OID_DER, false, &payload),
            64,
        )
        .unwrap_err();
        assert!(matches!(
            error.kind(),
            AuthorityKeyIdentifierErrorKind::Structure(_)
        ));
    }
    let empty = decode(
        &extension(AUTHORITY_KEY_IDENTIFIER_OID_DER, false, &sequence(&[])),
        64,
    )
    .unwrap_err();
    assert_eq!(
        empty.kind(),
        AuthorityKeyIdentifierErrorKind::UnsupportedProfile
    );
}

#[test]
fn rejects_issuer_serial_and_trailing_forms_instead_of_ignoring_them() {
    for value in [
        sequence(&[tlv(0xa1, &[]), tlv(0x82, &[0x01])]),
        sequence(&[tlv(0x82, &[0x01])]),
        sequence(&[tlv(0x80, &[0xaa; 20]), tlv(0xa1, &[]), tlv(0x82, &[0x01])]),
        sequence(&[tlv(0x80, &[0xaa; 20]), tlv(0x80, &[0xbb; 20])]),
    ] {
        let error = decode(
            &extension(AUTHORITY_KEY_IDENTIFIER_OID_DER, false, &value),
            64,
        )
        .unwrap_err();
        assert_eq!(
            error.kind(),
            AuthorityKeyIdentifierErrorKind::UnsupportedProfile
        );
    }
}

#[test]
fn rejects_constructed_empty_and_over_policy_key_identifiers() {
    let constructed = sequence(&[tlv(0xa0, &[])]);
    let error = decode(
        &extension(AUTHORITY_KEY_IDENTIFIER_OID_DER, false, &constructed),
        64,
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityKeyIdentifierErrorKind::UnsupportedProfile
    );

    let empty = decode(
        &extension(AUTHORITY_KEY_IDENTIFIER_OID_DER, false, &key_only(&[])),
        64,
    )
    .unwrap_err();
    assert_eq!(
        empty.kind(),
        AuthorityKeyIdentifierErrorKind::EmptyKeyIdentifier
    );

    let too_long = decode(
        &extension(
            AUTHORITY_KEY_IDENTIFIER_OID_DER,
            false,
            &key_only(&[0x7d; 21]),
        ),
        20,
    )
    .unwrap_err();
    assert_eq!(
        too_long.kind(),
        AuthorityKeyIdentifierErrorKind::KeyIdentifierTooLong
    );
}

#[test]
fn validates_policy_preserves_depth_budget_and_redacts_failures() {
    let invalid_policy = AuthorityKeyIdentifierPolicy::new(0).unwrap_err();
    assert_eq!(
        invalid_policy.kind(),
        AuthorityKeyIdentifierPolicyErrorKind::ZeroMaximumIdentifierBytes
    );
    assert_eq!(
        AuthorityKeyIdentifierPolicy::new(64)
            .unwrap()
            .maximum_identifier_bytes(),
        64
    );

    let encoded = extension(
        AUTHORITY_KEY_IDENTIFIER_OID_DER,
        false,
        &key_only(&[0xaa; 20]),
    );
    let limits = Asn1Limits {
        max_depth: 2,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    let generic = decode_x509_extension(&mut decoder, root).unwrap();
    let depth_error = decode_authority_key_identifier(
        &mut decoder,
        generic,
        AuthorityKeyIdentifierPolicy::new(64).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        depth_error.kind(),
        AuthorityKeyIdentifierErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    ));

    let too_long = decode(
        &extension(
            AUTHORITY_KEY_IDENTIFIER_OID_DER,
            false,
            &key_only(&[0x7d; 21]),
        ),
        20,
    )
    .unwrap_err();
    for rendered in [
        format!("{invalid_policy:?}"),
        invalid_policy.to_string(),
        format!("{too_long:?}"),
        too_long.to_string(),
    ] {
        assert!(!rendered.contains("0x7d"));
        assert!(!rendered.contains("21"));
        assert!(!rendered.contains("20"));
    }
}
