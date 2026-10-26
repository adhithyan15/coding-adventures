use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_authority_information_access::{
    decode_authority_information_access, AuthorityInformationAccessErrorKind,
    AUTHORITY_INFORMATION_ACCESS_OID, ID_AD_CA_ISSUERS_OID, ID_AD_OCSP_OID,
    MAX_ACCESS_DESCRIPTIONS,
};
use x509_extension::decode_x509_extension;

const AIA_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x01, 0x01];
const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];
const ID_AD_OCSP_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01];
const ID_AD_CA_ISSUERS_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x02];

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

fn description(method: &[u8], uri: &[u8]) -> Vec<u8> {
    sequence(&[element(0x06, method), element(0x86, uri)])
}

fn decode_with_limits(
    encoded: &[u8],
    limits: Asn1Limits,
) -> Result<
    x509_authority_information_access::AuthorityInformationAccess<'_>,
    x509_authority_information_access::AuthorityInformationAccessError,
> {
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder
        .decode_exact(encoded)
        .expect("valid outer DER framing");
    let generic = decode_x509_extension(&mut decoder, root).expect("valid generic extension");
    decode_authority_information_access(&mut decoder, generic)
}

fn decode(
    encoded: &[u8],
) -> Result<
    x509_authority_information_access::AuthorityInformationAccess<'_>,
    x509_authority_information_access::AuthorityInformationAccessError,
> {
    decode_with_limits(encoded, Asn1Limits::default())
}

#[test]
fn preserves_methods_and_uri_locations_in_wire_order() {
    assert_eq!(
        AUTHORITY_INFORMATION_ACCESS_OID,
        &[1, 3, 6, 1, 5, 5, 7, 1, 1]
    );
    assert_eq!(ID_AD_OCSP_OID, &[1, 3, 6, 1, 5, 5, 7, 48, 1]);
    assert_eq!(ID_AD_CA_ISSUERS_OID, &[1, 3, 6, 1, 5, 5, 7, 48, 2]);

    let unknown_method = [0x2a, 0x03, 0x04];
    let payload = sequence(&[
        description(ID_AD_OCSP_OID_DER, b"http://ocsp.digicert.com"),
        description(&unknown_method, b"urn:example:policy"),
        description(ID_AD_CA_ISSUERS_OID_DER, b"http://example.test/ca.der"),
    ]);
    let encoded = extension(AIA_OID_DER, false, &payload);
    let decoded = decode(&encoded).unwrap();

    assert_eq!(decoded.len(), 3);
    assert!(!decoded.is_empty());
    assert_eq!(decoded.get(0).unwrap().uri(), "http://ocsp.digicert.com");
    assert!(decoded
        .get(1)
        .unwrap()
        .access_method()
        .equals(&[1, 2, 3, 4]));
    assert_eq!(decoded.get(3), None);
    assert_eq!(
        decoded.locations_for(ID_AD_OCSP_OID).collect::<Vec<_>>(),
        vec!["http://ocsp.digicert.com"]
    );
    assert_eq!(
        decoded.iter().map(|value| value.uri()).collect::<Vec<_>>(),
        vec![
            "http://ocsp.digicert.com",
            "urn:example:policy",
            "http://example.test/ca.der"
        ]
    );
}

#[test]
fn rejects_critical_and_wrong_extensions_before_payload_access() {
    let malformed_payload = [0xde, 0xad];
    let critical = extension(AIA_OID_DER, true, &malformed_payload);
    let error = decode(&critical).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::CriticalExtension
    );
    assert_eq!(error.offset(), 0);

    let wrong = extension(KEY_USAGE_OID_DER, false, &malformed_payload);
    let error = decode(&wrong).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::WrongExtensionId
    );
    assert_eq!(error.offset(), 0);
}

#[test]
fn requires_one_exact_non_empty_sequence() {
    let empty = sequence(&[]);
    let empty_encoded = extension(AIA_OID_DER, false, &empty);
    let error = decode(&empty_encoded).unwrap_err();
    assert_eq!(error.kind(), AuthorityInformationAccessErrorKind::Empty);
    assert_eq!(error.offset(), 2);

    for payload in [vec![], vec![0x31, 0], vec![0x30, 0, 0x05, 0]] {
        assert!(matches!(
            decode(&extension(AIA_OID_DER, false, &payload))
                .unwrap_err()
                .kind(),
            AuthorityInformationAccessErrorKind::Structure(_)
        ));
    }
}

#[test]
fn accepts_exact_capacity_and_rejects_one_more() {
    let descriptions: Vec<_> = (0..MAX_ACCESS_DESCRIPTIONS)
        .map(|index| description(&[0x2a, 0x03, index as u8], b"urn:bounded"))
        .collect();
    let payload = sequence(&descriptions);
    let encoded = extension(AIA_OID_DER, false, &payload);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.len(), MAX_ACCESS_DESCRIPTIONS);

    let mut too_many = descriptions;
    too_many.push(description(&[0x2a, 0x03, 0x40], b"urn:excess"));
    let payload = sequence(&too_many);
    let last_offset = payload.len() - too_many.last().unwrap().len();
    let encoded = extension(AIA_OID_DER, false, &payload);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::TooManyAccessDescriptions
    );
    assert_eq!(error.offset(), last_offset);
}

#[test]
fn rejects_missing_or_invalid_access_method() {
    let missing_payload = sequence(&[sequence(&[])]);
    let missing_encoded = extension(AIA_OID_DER, false, &missing_payload);
    let error = decode(&missing_encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::MissingAccessMethod
    );
    assert_eq!(error.offset(), 4);

    for (method, expected) in [
        (element(0x05, &[]), Asn1ErrorKind::UnexpectedTag),
        (element(0x06, &[]), Asn1ErrorKind::EmptyObjectIdentifier),
        (
            element(0x06, &[0x80]),
            Asn1ErrorKind::NonMinimalObjectIdentifier,
        ),
    ] {
        let payload = sequence(&[sequence(&[method, element(0x86, b"urn:test")])]);
        let encoded = extension(AIA_OID_DER, false, &payload);
        let error = decode(&encoded).unwrap_err();
        assert_eq!(
            error.kind(),
            AuthorityInformationAccessErrorKind::InvalidAccessMethod(expected)
        );
    }
}

#[test]
fn rejects_missing_unsupported_empty_and_non_ascii_locations() {
    let method = element(0x06, ID_AD_OCSP_OID_DER);
    let cases = [
        (
            sequence(std::slice::from_ref(&method)),
            AuthorityInformationAccessErrorKind::MissingAccessLocation,
        ),
        (
            sequence(&[method.clone(), element(0x82, b"ocsp.example")]),
            AuthorityInformationAccessErrorKind::UnsupportedAccessLocation,
        ),
        (
            sequence(&[method.clone(), element(0x86, &[])]),
            AuthorityInformationAccessErrorKind::EmptyUri,
        ),
        (
            sequence(&[method, element(0x86, &[0xff])]),
            AuthorityInformationAccessErrorKind::InvalidUri(Asn1ErrorKind::NonAsciiIa5String),
        ),
    ];

    for (access_description, expected) in cases {
        let payload = sequence(&[access_description]);
        let encoded = extension(AIA_OID_DER, false, &payload);
        assert_eq!(decode(&encoded).unwrap_err().kind(), expected);
    }
}

#[test]
fn rejects_wrong_uri_tag_shape_and_trailing_fields() {
    let method = element(0x06, ID_AD_OCSP_OID_DER);
    let constructed_uri = element(0xa6, b"urn:test");
    let payload = sequence(&[sequence(&[method.clone(), constructed_uri])]);
    let encoded = extension(AIA_OID_DER, false, &payload);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::InvalidUri(Asn1ErrorKind::UnexpectedTag)
    );

    let payload = sequence(&[sequence(&[
        method,
        element(0x86, b"urn:test"),
        element(0x05, &[]),
    ])]);
    let encoded = extension(AIA_OID_DER, false, &payload);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::TrailingAccessDescriptionField
    );
}

#[test]
fn malformed_nested_child_reports_payload_local_offset() {
    let mut fields = element(0x06, ID_AD_OCSP_OID_DER);
    fields.extend_from_slice(&[0x86, 0x02, b'h']);
    let payload = sequence(&[element(0x30, &fields)]);
    let encoded = extension(AIA_OID_DER, false, &payload);
    let error = decode(&encoded).unwrap_err();
    assert!(matches!(
        error.kind(),
        AuthorityInformationAccessErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), payload.len());
}

#[test]
fn malformed_description_boundaries_fail_closed() {
    let wrong_container = sequence(&[element(0x31, &[])]);
    let encoded = extension(AIA_OID_DER, false, &wrong_container);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
    );

    let malformed_outer_child = element(0x30, &[0x30, 0x02, 0x06]);
    let encoded = extension(AIA_OID_DER, false, &malformed_outer_child);
    assert!(matches!(
        decode(&encoded).unwrap_err().kind(),
        AuthorityInformationAccessErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));

    let malformed_method = sequence(&[element(0x30, &[0x06, 0x02, 0x2a])]);
    let encoded = extension(AIA_OID_DER, false, &malformed_method);
    assert!(matches!(
        decode(&encoded).unwrap_err().kind(),
        AuthorityInformationAccessErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));

    let mut fields = element(0x06, ID_AD_OCSP_OID_DER);
    fields.extend_from_slice(&element(0x86, b"urn:test"));
    fields.extend_from_slice(&[0x05, 0x01]);
    let malformed_trailing = sequence(&[element(0x30, &fields)]);
    let encoded = extension(AIA_OID_DER, false, &malformed_trailing);
    assert!(matches!(
        decode(&encoded).unwrap_err().kind(),
        AuthorityInformationAccessErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
}

#[test]
fn shared_depth_element_value_and_oid_limits_are_enforced() {
    let payload = sequence(&[description(ID_AD_OCSP_OID_DER, b"urn:test")]);
    let encoded = extension(AIA_OID_DER, false, &payload);

    let element_limits = Asn1Limits {
        max_total_elements: 6,
        ..Asn1Limits::default()
    };
    let error = decode_with_limits(&encoded, element_limits).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );

    let mut outer_decoder = Asn1Decoder::new(Asn1Limits::default());
    let outer = outer_decoder.decode_exact(&encoded).unwrap();
    let generic = decode_x509_extension(&mut outer_decoder, outer).unwrap();

    let oid_limits = Asn1Limits {
        max_oid_arcs: 4,
        ..Asn1Limits::default()
    };
    let mut oid_decoder = Asn1Decoder::new(oid_limits);
    let error = decode_authority_information_access(&mut oid_decoder, generic).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::InvalidAccessMethod(
            Asn1ErrorKind::OidArcLimitExceeded
        )
    );

    let depth_limits = Asn1Limits {
        max_depth: 1,
        ..Asn1Limits::default()
    };
    let mut depth_decoder = Asn1Decoder::new(depth_limits);
    let error = decode_authority_information_access(&mut depth_decoder, generic).unwrap_err();
    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    );

    let mut value_limits = Asn1Limits::default();
    value_limits.der.max_value_len = 1;
    let mut value_decoder = Asn1Decoder::new(value_limits);
    let error = decode_authority_information_access(&mut value_decoder, generic).unwrap_err();
    assert!(matches!(
        error.kind(),
        AuthorityInformationAccessErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
}

#[test]
fn embedded_value_preserves_the_enclosing_extension_depth() {
    let payload = sequence(&[description(ID_AD_OCSP_OID_DER, b"urn:test")]);
    let encoded = extension(AIA_OID_DER, false, &payload);
    let limits = Asn1Limits {
        max_depth: 4,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    let generic = decode_x509_extension(&mut decoder, root).unwrap();
    let error = decode_authority_information_access(&mut decoder, generic).unwrap_err();

    assert_eq!(
        error.kind(),
        AuthorityInformationAccessErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    );
}

#[test]
fn errors_are_redacted() {
    let secret_uri = b"http://private.example.invalid/credential";
    let payload = sequence(&[sequence(&[
        element(0x06, ID_AD_OCSP_OID_DER),
        element(0x82, secret_uri),
    ])]);
    let encoded = extension(AIA_OID_DER, false, &payload);
    let error = decode(&encoded).unwrap_err();
    let displayed = error.to_string();
    assert!(!displayed.contains("private"));
    assert!(!displayed.contains("1.3.6.1.5.5.7.48.1"));
    assert!(displayed.contains("byte"));
}
