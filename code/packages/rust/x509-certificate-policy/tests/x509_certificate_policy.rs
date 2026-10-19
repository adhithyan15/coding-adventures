use der_asn1::Asn1Limits;
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{validate_critical_extensions, X509CertificatePolicyErrorKind};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const BASIC_CONSTRAINTS_OID: &[u8] = &[0x55, 0x1d, 0x13];
const KEY_USAGE_OID: &[u8] = &[0x55, 0x1d, 0x0f];

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

fn name() -> Vec<u8> {
    let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(der(0x0c, b"example"));
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

fn certificate(extensions: &[Vec<u8>]) -> Vec<u8> {
    let mut validity = der(0x17, b"260101000000Z");
    validity.extend(der(0x17, b"270101000000Z"));

    let mut subject_public_key_info = algorithm(KEY_OID);
    subject_public_key_info.extend(der(0x03, &[0x00, 0x30, 0x00]));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[0x01]));
    tbs.extend(algorithm(SIGNATURE_OID));
    tbs.extend(name());
    tbs.extend(der(0x30, &validity));
    tbs.extend(name());
    tbs.extend(der(0x30, &subject_public_key_info));
    if !extensions.is_empty() {
        let extension_values = extensions.concat();
        tbs.extend(der(0xa3, &der(0x30, &extension_values)));
    }

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

#[test]
fn accepts_absent_extensions_without_consulting_policy() {
    let encoded = certificate(&[]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let mut calls = 0usize;

    validate_critical_extensions(&certificate, |_| {
        calls += 1;
        false
    })
    .unwrap();

    assert_eq!(calls, 0);
}

#[test]
fn ignores_unrecognized_noncritical_extensions() {
    let encoded = certificate(&[extension(BASIC_CONSTRAINTS_OID, false, &[0x30, 0x00])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let mut calls = 0usize;

    validate_critical_extensions(&certificate, |_| {
        calls += 1;
        false
    })
    .unwrap();

    assert_eq!(calls, 0);
}

#[test]
fn accepts_a_caller_recognized_critical_extension() {
    let encoded = certificate(&[extension(BASIC_CONSTRAINTS_OID, true, &[0x30, 0x00])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();

    validate_critical_extensions(&certificate, |oid| oid.equals(&[2, 5, 29, 19])).unwrap();
}

#[test]
fn rejects_the_first_unrecognized_critical_extension_in_wire_order() {
    let encoded = certificate(&[
        extension(BASIC_CONSTRAINTS_OID, true, &[0x30, 0x00]),
        extension(&[0x2a, 0x03], false, &[0x05, 0x00]),
        extension(KEY_USAGE_OID, true, &[0x03, 0x02, 0x07, 0x80]),
        extension(&[0x2a, 0x04], true, b"secret-payload"),
    ]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let mut consulted = Vec::new();

    let error = validate_critical_extensions(&certificate, |oid| {
        consulted.push(oid.arcs().collect::<Vec<_>>());
        oid.equals(&[2, 5, 29, 19])
    })
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificatePolicyErrorKind::UnsupportedCriticalExtension
    );
    assert_eq!(error.extension_index(), 2);
    assert_eq!(consulted, vec![vec![2, 5, 29, 19], vec![2, 5, 29, 15]]);
    assert!(!error.to_string().contains("secret-payload"));
    assert!(!error.to_string().contains("2.5.29"));
}
