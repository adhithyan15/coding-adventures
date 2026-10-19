use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{validate_certificate_validity_at, X509CertificateValidityErrorKind};
use x509_time::{decode_x509_time, X509Time};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

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

fn utc(value: &[u8; 13]) -> Vec<u8> {
    der(0x17, value)
}

fn generalized(value: &[u8; 15]) -> Vec<u8> {
    der(0x18, value)
}

fn certificate(not_before: &[u8], not_after: &[u8]) -> Vec<u8> {
    let mut validity = not_before.to_vec();
    validity.extend(not_after);

    let mut subject_public_key_info = algorithm(KEY_OID);
    subject_public_key_info.extend(der(0x03, &[0x00, 0x30, 0x00]));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[0x01]));
    tbs.extend(algorithm(SIGNATURE_OID));
    tbs.extend(name(b"issuer.example"));
    tbs.extend(der(0x30, &validity));
    tbs.extend(name(b"subject.example"));
    tbs.extend(der(0x30, &subject_public_key_info));

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

fn time(encoded: &[u8]) -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(encoded).unwrap();
    decode_x509_time(element).unwrap()
}

fn validate(
    encoded: &[u8],
    observed: &X509Time,
) -> Result<
    x509_certificate_policy::ValidatedCertificateValidity,
    x509_certificate_policy::X509CertificateValidityError,
> {
    let certificate = decode_x509_certificate(encoded, Asn1Limits::default()).unwrap();
    validate_certificate_validity_at(&certificate, observed)
}

#[test]
fn accepts_inside_and_at_both_inclusive_endpoints() {
    let encoded = certificate(&utc(b"240101000000Z"), &utc(b"241231235959Z"));

    for observed in [
        utc(b"240101000000Z"),
        utc(b"240701120000Z"),
        utc(b"241231235959Z"),
    ] {
        assert!(validate(&encoded, &time(&observed)).is_ok());
    }
}

#[test]
fn distinguishes_not_yet_valid_from_expired() {
    let encoded = certificate(&utc(b"240101000000Z"), &utc(b"241231235959Z"));

    let early = validate(&encoded, &time(&utc(b"231231235959Z"))).unwrap_err();
    assert_eq!(early.kind(), X509CertificateValidityErrorKind::NotYetValid);

    let late = validate(&encoded, &time(&utc(b"250101000000Z"))).unwrap_err();
    assert_eq!(late.kind(), X509CertificateValidityErrorKind::Expired);
}

#[test]
fn supports_mixed_encodings_and_zero_duration_intervals() {
    let mixed = certificate(&utc(b"491231235959Z"), &generalized(b"20500101000000Z"));
    assert!(validate(&mixed, &time(&generalized(b"20500101000000Z"))).is_ok());

    let instant = generalized(b"20500101000000Z");
    let zero_duration = certificate(&instant, &instant);
    assert!(validate(&zero_duration, &time(&instant)).is_ok());
}

#[test]
fn failures_do_not_disclose_certificate_or_observed_times() {
    let encoded = certificate(&utc(b"240101000000Z"), &utc(b"241231235959Z"));
    let error = validate(&encoded, &time(&utc(b"231231235959Z"))).unwrap_err();
    let rendered = error.to_string();

    assert!(!rendered.contains("231231"));
    assert!(!rendered.contains("240101"));
    assert!(!rendered.contains("241231"));
}
