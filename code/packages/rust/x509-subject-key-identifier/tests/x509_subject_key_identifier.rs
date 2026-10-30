use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_extension::decode_x509_extension;
use x509_subject_key_identifier::{
    decode_subject_key_identifier, SubjectKeyIdentifierErrorKind, SubjectKeyIdentifierPolicy,
    SubjectKeyIdentifierPolicyErrorKind, SUBJECT_KEY_IDENTIFIER_OID,
};

const SUBJECT_KEY_IDENTIFIER_OID_DER: &[u8] = &[0x55, 0x1d, 0x0e];
const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];

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

fn decode(
    encoded: &[u8],
    maximum_identifier_bytes: usize,
) -> Result<
    x509_subject_key_identifier::SubjectKeyIdentifier<'_>,
    x509_subject_key_identifier::SubjectKeyIdentifierError,
> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder
        .decode_exact(encoded)
        .expect("valid outer DER framing");
    let generic = decode_x509_extension(&mut decoder, root).expect("valid generic extension");
    decode_subject_key_identifier(
        &mut decoder,
        generic,
        SubjectKeyIdentifierPolicy::new(maximum_identifier_bytes).unwrap(),
    )
}

#[test]
fn decodes_one_exact_borrowed_identifier_under_the_caller_ceiling() {
    assert_eq!(SUBJECT_KEY_IDENTIFIER_OID, &[2, 5, 29, 14]);
    let identifier: Vec<u8> = (0u8..20).collect();
    let payload = octet_string(&identifier);
    let encoded = extension(SUBJECT_KEY_IDENTIFIER_OID_DER, &payload);

    let decoded = decode(&encoded, 20).unwrap();

    assert_eq!(decoded.identifier(), identifier);
}

#[test]
fn leaves_outer_criticality_to_certificate_policy() {
    let payload = octet_string(&[0xaa; 20]);
    let encoded = sequence(&[
        oid(SUBJECT_KEY_IDENTIFIER_OID_DER),
        vec![0x01, 0x01, 0xff],
        octet_string(&payload),
    ]);

    assert_eq!(decode(&encoded, 20).unwrap().identifier(), &[0xaa; 20]);
}

#[test]
fn rejects_wrong_oid_before_accessing_opaque_payload() {
    let encoded = extension(KEY_USAGE_OID_DER, &[0xde, 0xad]);
    let error = decode(&encoded, 64).unwrap_err();
    assert_eq!(
        error.kind(),
        SubjectKeyIdentifierErrorKind::WrongExtensionId
    );
    assert_eq!(error.offset(), 0);
}

#[test]
fn requires_one_exact_primitive_octet_string() {
    for payload in [vec![], vec![0x05, 0x00], vec![0x04, 0x01, 0xaa, 0x05, 0x00]] {
        let error = decode(&extension(SUBJECT_KEY_IDENTIFIER_OID_DER, &payload), 64).unwrap_err();
        assert!(matches!(
            error.kind(),
            SubjectKeyIdentifierErrorKind::Structure(_)
                | SubjectKeyIdentifierErrorKind::InvalidIdentifier(_)
        ));
    }

    let wrong_tag = decode(
        &extension(SUBJECT_KEY_IDENTIFIER_OID_DER, &[0x05, 0x00]),
        64,
    )
    .unwrap_err();
    assert!(matches!(
        wrong_tag.kind(),
        SubjectKeyIdentifierErrorKind::InvalidIdentifier(Asn1ErrorKind::UnexpectedTag)
    ));
}

#[test]
fn rejects_empty_and_over_policy_identifiers() {
    let empty = decode(
        &extension(SUBJECT_KEY_IDENTIFIER_OID_DER, &octet_string(&[])),
        64,
    )
    .unwrap_err();
    assert_eq!(empty.kind(), SubjectKeyIdentifierErrorKind::EmptyIdentifier);
    assert_eq!(empty.offset(), 2);

    let too_long = decode(
        &extension(SUBJECT_KEY_IDENTIFIER_OID_DER, &octet_string(&[0x7d; 21])),
        20,
    )
    .unwrap_err();
    assert_eq!(
        too_long.kind(),
        SubjectKeyIdentifierErrorKind::IdentifierTooLong
    );
    assert_eq!(too_long.offset(), 2);
}

#[test]
fn validates_policy_and_redacts_identifier_bytes_and_lengths() {
    let invalid_policy = SubjectKeyIdentifierPolicy::new(0).unwrap_err();
    assert_eq!(
        invalid_policy.kind(),
        SubjectKeyIdentifierPolicyErrorKind::ZeroMaximumIdentifierBytes
    );
    assert_eq!(
        SubjectKeyIdentifierPolicy::new(64)
            .unwrap()
            .maximum_identifier_bytes(),
        64
    );

    let error = decode(
        &extension(SUBJECT_KEY_IDENTIFIER_OID_DER, &octet_string(&[0x7d; 21])),
        20,
    )
    .unwrap_err();
    for rendered in [
        format!("{invalid_policy:?}"),
        invalid_policy.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("0x7d"));
        assert!(!rendered.contains("21"));
        assert!(!rendered.contains("20"));
    }
}
