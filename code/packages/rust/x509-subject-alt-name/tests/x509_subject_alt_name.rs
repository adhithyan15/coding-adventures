use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use tls_server_identity::{verify_server_identity, PresentedIdentities, ReferenceIdentity};
use x509_extension::decode_x509_extension;
use x509_subject_alt_name::{
    decode_subject_alt_name, SubjectAltNameErrorKind, MAX_GENERAL_NAMES, SUBJECT_ALT_NAME_OID,
};

const SUBJECT_ALT_NAME_OID_DER: &[u8] = &[0x55, 0x1d, 0x11];
const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];

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

fn decode_with_limits(
    encoded: &[u8],
    limits: Asn1Limits,
) -> Result<
    x509_subject_alt_name::SubjectAlternativeName<'_>,
    x509_subject_alt_name::SubjectAltNameError,
> {
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder
        .decode_exact(encoded)
        .expect("valid outer DER framing");
    let generic = decode_x509_extension(&mut decoder, root).expect("valid generic extension");
    decode_subject_alt_name(&mut decoder, generic)
}

fn decode(
    encoded: &[u8],
) -> Result<
    x509_subject_alt_name::SubjectAlternativeName<'_>,
    x509_subject_alt_name::SubjectAltNameError,
> {
    decode_with_limits(encoded, Asn1Limits::default())
}

#[test]
fn retains_dns_and_typed_ip_identities_and_counts_other_choices() {
    assert_eq!(SUBJECT_ALT_NAME_OID, &[2, 5, 29, 17]);
    let ipv6 = Ipv6Addr::LOCALHOST.octets();
    let payload = sequence(&[
        element(0x81, b"ops@example.test"),
        element(0x82, b"api.example.test"),
        element(0x86, b"https://example.test/status"),
        element(0x87, &[192, 0, 2, 44]),
        element(0x87, &ipv6),
        element(0x88, &[0x2a, 0x03, 0x04]),
        element(0xa0, &[0x05, 0x00]),
        element(0xa3, &[0x30, 0x00]),
        element(0xa4, &[0x30, 0x00]),
        element(0xa5, &[0x30, 0x00]),
    ]);
    let encoded = extension(SUBJECT_ALT_NAME_OID_DER, false, &payload);
    let decoded = decode(&encoded).unwrap();

    assert_eq!(decoded.len(), 10);
    assert!(!decoded.is_empty());
    assert_eq!(decoded.dns_names(), &["api.example.test"]);
    assert_eq!(
        decoded.ip_addresses(),
        &[
            IpAddr::V4(Ipv4Addr::new(192, 0, 2, 44)),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
        ]
    );
    assert_eq!(decoded.opaque_names(), 7);
    assert_eq!(
        format!("{decoded:?}"),
        "SubjectAlternativeName { dns_names: 1, ip_addresses: 2, opaque_names: 7 }"
    );
}

#[test]
fn criticality_remains_caller_owned() {
    let payload = sequence(&[element(0x82, b"critical.example")]);
    let encoded = extension(SUBJECT_ALT_NAME_OID_DER, true, &payload);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.dns_names(), &["critical.example"]);
}

#[test]
fn decoded_slices_feed_the_existing_identity_matcher_without_conversion() {
    let payload = sequence(&[
        element(0x82, b"api.example.test"),
        element(0x87, &[192, 0, 2, 44]),
    ]);
    let encoded = extension(SUBJECT_ALT_NAME_OID_DER, false, &payload);
    let decoded = decode(&encoded).unwrap();
    let presented = PresentedIdentities::new(decoded.dns_names(), decoded.ip_addresses());

    let dns_reference = ReferenceIdentity::parse("api.example.test").unwrap();
    assert!(verify_server_identity(&dns_reference, presented).is_ok());
    let ip_reference = ReferenceIdentity::parse("192.0.2.44").unwrap();
    assert!(verify_server_identity(&ip_reference, presented).is_ok());
}

#[test]
fn rejects_wrong_extension_before_payload_access() {
    let encoded = extension(KEY_USAGE_OID_DER, false, &[0xde, 0xad]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), SubjectAltNameErrorKind::WrongExtensionId);
    assert_eq!(error.offset(), 0);
}

#[test]
fn requires_one_non_empty_exact_sequence() {
    let empty = sequence(&[]);
    let error = decode(&extension(SUBJECT_ALT_NAME_OID_DER, false, &empty)).unwrap_err();
    assert_eq!(error.kind(), SubjectAltNameErrorKind::Empty);
    assert_eq!(error.offset(), 2);

    for payload in [vec![], vec![0x31, 0], vec![0x30, 0, 0x05, 0]] {
        assert!(matches!(
            decode(&extension(SUBJECT_ALT_NAME_OID_DER, false, &payload))
                .unwrap_err()
                .kind(),
            SubjectAltNameErrorKind::Structure(_)
        ));
    }
}

#[test]
fn enforces_fixed_general_name_capacity() {
    let names: Vec<_> = (0..MAX_GENERAL_NAMES)
        .map(|_| element(0x82, b"a"))
        .collect();
    let payload = sequence(&names);
    let encoded = extension(SUBJECT_ALT_NAME_OID_DER, false, &payload);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.len(), MAX_GENERAL_NAMES);
    assert_eq!(decoded.dns_names().len(), MAX_GENERAL_NAMES);

    let mut too_many = names;
    too_many.push(element(0x82, b"b"));
    let payload = sequence(&too_many);
    let last_offset = payload.len() - too_many.last().unwrap().len();
    let error = decode(&extension(SUBJECT_ALT_NAME_OID_DER, false, &payload)).unwrap_err();
    assert_eq!(error.kind(), SubjectAltNameErrorKind::TooManyNames);
    assert_eq!(error.offset(), last_offset);
}

#[test]
fn requires_exact_context_specific_tag_forms() {
    for invalid in [
        element(0x16, b"universal"),
        element(0x42, b"application"),
        element(0xc2, b"private"),
        element(0xa2, b"constructed-dns"),
        element(0x80, b"primitive-other"),
        element(0x89, b"unknown"),
    ] {
        let payload = sequence(&[invalid]);
        let error = decode(&extension(SUBJECT_ALT_NAME_OID_DER, false, &payload)).unwrap_err();
        assert_eq!(error.kind(), SubjectAltNameErrorKind::InvalidGeneralNameTag);
        assert_eq!(error.offset(), 2);
    }
}

#[test]
fn rejects_empty_general_names_of_every_supported_shape() {
    for tag in [0xa0, 0x81, 0x82, 0xa3, 0xa4, 0xa5, 0x86, 0x87, 0x88] {
        let payload = sequence(&[element(tag, &[])]);
        let error = decode(&extension(SUBJECT_ALT_NAME_OID_DER, false, &payload)).unwrap_err();
        assert_eq!(error.kind(), SubjectAltNameErrorKind::EmptyGeneralName);
        assert_eq!(error.offset(), 4);
    }
}

#[test]
fn validates_all_ia5_choices() {
    for tag in [0x81, 0x82, 0x86] {
        let payload = sequence(&[element(tag, &[b'a', 0x80])]);
        let error = decode(&extension(SUBJECT_ALT_NAME_OID_DER, false, &payload)).unwrap_err();
        assert_eq!(
            error.kind(),
            SubjectAltNameErrorKind::InvalidIa5Name(Asn1ErrorKind::NonAsciiIa5String)
        );
        assert_eq!(error.offset(), 5);
    }
}

#[test]
fn rejects_non_address_ip_lengths() {
    for length in [1usize, 3, 5, 15, 17] {
        let payload = sequence(&[element(0x87, &vec![0; length])]);
        let error = decode(&extension(SUBJECT_ALT_NAME_OID_DER, false, &payload)).unwrap_err();
        assert_eq!(
            error.kind(),
            SubjectAltNameErrorKind::InvalidIpAddressLength
        );
        assert_eq!(error.offset(), 4);
    }
}

#[test]
fn validates_registered_ids() {
    for (contents, kind, offset) in [
        (&[0x80][..], Asn1ErrorKind::NonMinimalObjectIdentifier, 4),
        (
            &[0x2a, 0x86][..],
            Asn1ErrorKind::UnterminatedObjectIdentifier,
            6,
        ),
    ] {
        let payload = sequence(&[element(0x88, contents)]);
        let error = decode(&extension(SUBJECT_ALT_NAME_OID_DER, false, &payload)).unwrap_err();
        assert_eq!(
            error.kind(),
            SubjectAltNameErrorKind::InvalidRegisteredId(kind)
        );
        assert_eq!(error.offset(), offset);
    }
}

#[test]
fn malformed_child_reports_payload_local_offset() {
    let mut contents = element(0x82, b"ok.example");
    contents.extend_from_slice(&[0x82, 0x02, b'x']);
    let payload = element(0x30, &contents);
    let error = decode(&extension(SUBJECT_ALT_NAME_OID_DER, false, &payload)).unwrap_err();
    assert!(matches!(
        error.kind(),
        SubjectAltNameErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), payload.len());
}

#[test]
fn shared_depth_element_and_oid_limits_are_enforced() {
    let payload = sequence(&[element(0x88, &[0x2a, 0x03, 0x04, 0x05])]);
    let encoded = extension(SUBJECT_ALT_NAME_OID_DER, false, &payload);

    let mut outer_decoder = Asn1Decoder::new(Asn1Limits::default());
    let outer = outer_decoder.decode_exact(&encoded).unwrap();
    let generic = decode_x509_extension(&mut outer_decoder, outer).unwrap();
    let mut depth_decoder = Asn1Decoder::new(Asn1Limits {
        max_depth: 1,
        ..Asn1Limits::default()
    });
    let error = decode_subject_alt_name(&mut depth_decoder, generic).unwrap_err();
    assert_eq!(
        error.kind(),
        SubjectAltNameErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    );

    let error = decode_with_limits(
        &encoded,
        Asn1Limits {
            max_total_elements: 4,
            ..Asn1Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        SubjectAltNameErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );

    let error = decode_with_limits(
        &encoded,
        Asn1Limits {
            max_oid_arcs: 4,
            ..Asn1Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        SubjectAltNameErrorKind::InvalidRegisteredId(Asn1ErrorKind::OidArcLimitExceeded)
    );
}

#[test]
fn diagnostics_are_redacted() {
    let secret = "secret.internal.example";
    let payload = sequence(&[element(0x82, &[secret.as_bytes(), &[0x80]].concat())]);
    let error = decode(&extension(SUBJECT_ALT_NAME_OID_DER, false, &payload)).unwrap_err();
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains(secret));
}
