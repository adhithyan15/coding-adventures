use der_asn1::Asn1Limits;
use std::cell::Cell;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_policy::classify_certificate_self_issued;

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
fn binds_one_exact_self_issued_certificate_after_one_directed_comparison() {
    let self_name = name(0x0c, b"private-self-issued-ca");
    let certificate = decoded_certificate(1, &self_name, &self_name);
    let calls = Cell::new(0);

    let classification = classify_certificate_self_issued(&certificate, |issuer, subject| {
        calls.set(calls.get() + 1);
        assert_eq!(issuer.encoded(), self_name);
        assert_eq!(subject.encoded(), self_name);
        issuer.encoded() == subject.encoded()
    });

    assert_eq!(calls.get(), 1);
    assert!(classification.is_self_issued());
    assert!(std::ptr::eq(classification.certificate(), &certificate));
    assert_eq!(
        format!("{classification:?}"),
        "ClassifiedCertificateSelfIssued { is_self_issued: true, .. }"
    );
    assert!(!format!("{classification:?}").contains("private-self-issued-ca"));
}

#[test]
fn binds_a_non_self_issued_result_without_disclosing_names() {
    let issuer_name = name(0x0c, b"private-issuer");
    let subject_name = name(0x0c, b"private-subject");
    let certificate = decoded_certificate(2, &issuer_name, &subject_name);
    let calls = Cell::new(0);

    let classification = classify_certificate_self_issued(&certificate, |issuer, subject| {
        calls.set(calls.get() + 1);
        issuer.encoded() == subject.encoded()
    });

    assert_eq!(calls.get(), 1);
    assert!(!classification.is_self_issued());
    assert!(std::ptr::eq(classification.certificate(), &certificate));
    let debug = format!("{classification:?}");
    assert!(!debug.contains("private-issuer"));
    assert!(!debug.contains("private-subject"));
}

#[test]
fn caller_owns_name_equivalence_across_distinct_encodings() {
    let issuer_name = name(0x0c, b"Example CA");
    let subject_name = name(0x13, b"EXAMPLE CA");
    let certificate = decoded_certificate(3, &issuer_name, &subject_name);

    let strict = classify_certificate_self_issued(&certificate, |issuer, subject| {
        issuer.encoded() == subject.encoded()
    });
    assert!(!strict.is_self_issued());

    let caller_equivalent = classify_certificate_self_issued(&certificate, |issuer, subject| {
        assert_ne!(issuer.encoded(), subject.encoded());
        true
    });
    assert!(caller_equivalent.is_self_issued());
}
