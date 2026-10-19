use der_asn1::Asn1Limits;
use std::cell::Cell;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_policy::{
    validate_certificate_issuer_link, X509CertificateIssuerLinkErrorKind,
};

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

fn name(string_tag: u8, common_name: &[u8]) -> Vec<u8> {
    let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(der(string_tag, common_name));
    der(0x30, &der(0x31, &der(0x30, &attribute)))
}

fn certificate(serial: u8, issuer: &[u8], subject: &[u8]) -> Vec<u8> {
    let mut validity = der(0x17, b"260101000000Z");
    validity.extend(der(0x17, b"270101000000Z"));

    let mut subject_public_key_info = algorithm(KEY_OID);
    subject_public_key_info.extend(der(0x03, &[0x00, 0x30, 0x00]));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[serial]));
    tbs.extend(algorithm(SIGNATURE_OID));
    tbs.extend(issuer);
    tbs.extend(der(0x30, &validity));
    tbs.extend(subject);
    tbs.extend(der(0x30, &subject_public_key_info));

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

fn decoded_certificate(serial: u8, issuer: &[u8], subject: &[u8]) -> X509Certificate<'static> {
    let encoded = Box::leak(certificate(serial, issuer, subject).into_boxed_slice());
    decode_x509_certificate(encoded, Asn1Limits::default()).unwrap()
}

#[test]
fn passes_child_issuer_then_candidate_subject_exactly_once() {
    let child_issuer = name(0x0c, b"issuing-ca");
    let child = decoded_certificate(1, &child_issuer, &name(0x0c, b"service"));
    let candidate = decoded_certificate(2, &name(0x0c, b"root"), &child_issuer);
    let calls = Cell::new(0);

    let evidence = validate_certificate_issuer_link(&child, &candidate, |issuer, subject| {
        calls.set(calls.get() + 1);
        assert_eq!(issuer.encoded(), child_issuer);
        assert_eq!(subject.encoded(), child_issuer);
        true
    })
    .unwrap();

    assert_eq!(calls.get(), 1);
    assert_eq!(
        format!("{evidence:?}"),
        "ValidatedCertificateIssuerLink { private: () }"
    );
}

#[test]
fn mismatch_fails_closed_once_without_disclosing_names() {
    let child = decoded_certificate(
        1,
        &name(0x0c, b"secret-child-issuer"),
        &name(0x0c, b"service"),
    );
    let candidate = decoded_certificate(
        2,
        &name(0x0c, b"root"),
        &name(0x0c, b"secret-other-subject"),
    );
    let calls = Cell::new(0);

    let error = validate_certificate_issuer_link(&child, &candidate, |issuer, subject| {
        calls.set(calls.get() + 1);
        issuer.encoded() == subject.encoded()
    })
    .unwrap_err();

    assert_eq!(calls.get(), 1);
    assert_eq!(
        error.kind(),
        X509CertificateIssuerLinkErrorKind::NameMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 certificate issuer link error NameMismatch"
    );
    assert!(!error.to_string().contains("secret-child-issuer"));
    assert!(!error.to_string().contains("secret-other-subject"));
}

#[test]
fn caller_policy_can_accept_distinct_name_encodings() {
    let child = decoded_certificate(1, &name(0x0c, b"Example CA"), &name(0x0c, b"service"));
    let candidate = decoded_certificate(2, &name(0x0c, b"root"), &name(0x13, b"EXAMPLE CA"));

    let strict_error = validate_certificate_issuer_link(&child, &candidate, |issuer, subject| {
        issuer.encoded() == subject.encoded()
    })
    .unwrap_err();
    assert_eq!(
        strict_error.kind(),
        X509CertificateIssuerLinkErrorKind::NameMismatch
    );

    validate_certificate_issuer_link(&child, &candidate, |issuer, subject| {
        assert_ne!(issuer.encoded(), subject.encoded());
        true
    })
    .unwrap();
}

#[test]
fn self_issued_certificate_can_link_under_exact_policy() {
    let self_name = name(0x0c, b"self-issued");
    let certificate = decoded_certificate(1, &self_name, &self_name);

    validate_certificate_issuer_link(&certificate, &certificate, |issuer, subject| {
        issuer.encoded() == subject.encoded()
    })
    .unwrap();
}
