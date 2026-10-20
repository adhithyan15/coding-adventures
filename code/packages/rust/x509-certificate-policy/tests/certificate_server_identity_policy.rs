use der_asn1::Asn1Limits;
use std::net::{IpAddr, Ipv4Addr};
use tls_server_identity::{MatchKind, ReferenceIdentity};
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    validate_certificate_server_identity, X509CertificateServerIdentityErrorKind,
};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const SUBJECT_ALT_NAME_OID: &[u8] = &[0x55, 0x1d, 0x11];
const OTHER_EXTENSION_OID: &[u8] = &[0x2a, 0x03];

fn der(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(value.len() + 4);
    encoded.push(tag);
    match value.len() {
        length @ 0..=127 => encoded.push(length as u8),
        length @ 128..=255 => encoded.extend([0x81, length as u8]),
        length @ 256..=65_535 => {
            encoded.extend([0x82, (length >> 8) as u8, length as u8]);
        }
        _ => panic!("test value too large"),
    }
    encoded.extend(value);
    encoded
}

fn algorithm(oid: &[u8]) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    fields.extend(der(0x05, &[]));
    der(0x30, &fields)
}

fn name(common_name: &[u8]) -> Vec<u8> {
    let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(der(0x0c, common_name));
    der(0x30, &der(0x31, &der(0x30, &attribute)))
}

fn extension(oid: &[u8], critical: bool, value: &[u8]) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    if critical {
        fields.extend(der(0x01, &[0xff]));
    }
    fields.extend(der(0x04, value));
    der(0x30, &fields)
}

fn dns_name(value: &str) -> Vec<u8> {
    der(0x82, value.as_bytes())
}

fn ip_address(value: &[u8]) -> Vec<u8> {
    der(0x87, value)
}

fn uri_name(value: &str) -> Vec<u8> {
    der(0x86, value.as_bytes())
}

fn subject_alt_name(critical: bool, names: &[Vec<u8>]) -> Vec<u8> {
    extension(SUBJECT_ALT_NAME_OID, critical, &der(0x30, &names.concat()))
}

fn certificate(common_name: &[u8], extensions: &[Vec<u8>]) -> Vec<u8> {
    let mut validity = der(0x17, b"260101000000Z");
    validity.extend(der(0x17, b"270101000000Z"));

    let mut subject_public_key_info = algorithm(KEY_OID);
    subject_public_key_info.extend(der(0x03, &[0x00, 0x30, 0x00]));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[0x01]));
    tbs.extend(algorithm(SIGNATURE_OID));
    tbs.extend(name(b"issuer.example"));
    tbs.extend(der(0x30, &validity));
    tbs.extend(name(common_name));
    tbs.extend(der(0x30, &subject_public_key_info));
    if !extensions.is_empty() {
        tbs.extend(der(0xa3, &der(0x30, &extensions.concat())));
    }

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

fn validate(
    encoded: &[u8],
    reference: &ReferenceIdentity,
    limits: Asn1Limits,
) -> Result<
    x509_certificate_policy::ValidatedCertificateServerIdentity,
    x509_certificate_policy::X509CertificateServerIdentityError,
> {
    let certificate = decode_x509_certificate(encoded, Asn1Limits::default()).unwrap();
    validate_certificate_server_identity(&certificate, reference, limits)
}

#[test]
fn requires_subject_alt_name_without_common_name_fallback() {
    let encoded = certificate(b"api.example.test", &[]);
    let reference = ReferenceIdentity::parse("api.example.test").unwrap();

    let error = validate(&encoded, &reference, Asn1Limits::default()).unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificateServerIdentityErrorKind::MissingSubjectAlternativeName
    );
    assert_eq!(error.extension_index(), None);
    assert!(!error.to_string().contains("api.example.test"));
}

#[test]
fn rejects_malformed_subject_alt_name_with_redacted_wire_index() {
    let extensions = [
        extension(OTHER_EXTENSION_OID, false, &der(0x05, &[])),
        extension(SUBJECT_ALT_NAME_OID, false, b"secret-presented-name"),
    ];
    let encoded = certificate(b"irrelevant.example", &extensions);
    let reference = ReferenceIdentity::parse("api.example.test").unwrap();

    let error = validate(&encoded, &reference, Asn1Limits::default()).unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificateServerIdentityErrorKind::InvalidSubjectAlternativeName
    );
    assert_eq!(error.extension_index(), Some(1));
    assert!(!error.to_string().contains("secret"));
    assert!(!error.to_string().contains("2.5.29.17"));
}

#[test]
fn accepts_exact_and_wildcard_dns_ids_at_either_criticality() {
    for (critical, presented, reference_name, expected) in [
        (
            false,
            "API.EXAMPLE.TEST",
            "api.example.test",
            MatchKind::ExactDns,
        ),
        (
            true,
            "*.example.test",
            "mail.example.test",
            MatchKind::WildcardDns,
        ),
    ] {
        let encoded = certificate(
            b"wrong.example.test",
            &[subject_alt_name(critical, &[dns_name(presented)])],
        );
        let reference = ReferenceIdentity::parse(reference_name).unwrap();

        let evidence = validate(&encoded, &reference, Asn1Limits::default()).unwrap();

        assert_eq!(evidence.extension_index(), 0);
        assert_eq!(evidence.match_kind(), expected);
    }
}

#[test]
fn matches_ip_only_against_ip_id() {
    let reference = ReferenceIdentity::IpAddress(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)));
    let encoded = certificate(
        b"192.0.2.10",
        &[subject_alt_name(false, &[ip_address(&[192, 0, 2, 10])])],
    );
    let evidence = validate(&encoded, &reference, Asn1Limits::default()).unwrap();
    assert_eq!(evidence.match_kind(), MatchKind::IpAddress);

    let dns_only = certificate(
        b"192.0.2.10",
        &[subject_alt_name(false, &[dns_name("192.0.2.10")])],
    );
    let error = validate(&dns_only, &reference, Asn1Limits::default()).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificateServerIdentityErrorKind::IdentityMismatch
    );
}

#[test]
fn rejects_closed_mismatch_without_disclosing_presented_names() {
    let encoded = certificate(
        b"wrong.example.test",
        &[subject_alt_name(
            false,
            &[
                uri_name("https://secret.example/path"),
                dns_name("secret.example"),
            ],
        )],
    );
    let reference = ReferenceIdentity::parse("api.example.test").unwrap();

    let error = validate(&encoded, &reference, Asn1Limits::default()).unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificateServerIdentityErrorKind::IdentityMismatch
    );
    assert_eq!(error.extension_index(), Some(0));
    assert!(!error.to_string().contains("secret.example"));
}

#[test]
fn applies_the_caller_decoder_budget_to_subject_alt_name() {
    let encoded = certificate(
        b"wrong.example.test",
        &[subject_alt_name(false, &[dns_name("api.example.test")])],
    );
    let reference = ReferenceIdentity::parse("api.example.test").unwrap();
    let limits = Asn1Limits {
        max_total_elements: 1,
        ..Asn1Limits::default()
    };

    let error = validate(&encoded, &reference, limits).unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificateServerIdentityErrorKind::InvalidSubjectAlternativeName
    );
    assert_eq!(error.extension_index(), Some(0));
}
