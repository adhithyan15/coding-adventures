use der_asn1::{Asn1Decoder, Asn1Limits};
use std::cell::Cell;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_ocsp_basic_response::{decode_x509_ocsp_basic_response, X509OcspBasicResponse};
use x509_ocsp_responder_id_binding::{
    bind_x509_ocsp_responder_id_to_certificate, X509OcspResponderIdBindingErrorKind,
    X509OcspResponderIdKind,
};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
// Independently checked with Python's hashlib.sha1 over [0x30, 0x00].
const RESPONDER_KEY_HASH: &[u8; 20] = &[
    0xf9, 0x44, 0xdc, 0xd6, 0x35, 0xf9, 0x80, 0x1f, 0x7a, 0xc9, 0x0a, 0x40, 0x7f, 0xbc, 0x47, 0x99,
    0x64, 0xde, 0xc0, 0x24,
];

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
    let value: Vec<u8> = fields.iter().flatten().copied().collect();
    element(0x30, &value)
}

fn algorithm(oid: &[u8]) -> Vec<u8> {
    sequence(&[element(0x06, oid), element(0x05, &[])])
}

fn time(value: &[u8]) -> Vec<u8> {
    element(0x18, value)
}

fn name(label: &[u8]) -> Vec<u8> {
    let attribute = sequence(&[element(0x06, &[0x55, 0x04, 0x03]), element(0x0c, label)]);
    sequence(&[element(0x31, &attribute)])
}

fn cert_id() -> Vec<u8> {
    sequence(&[
        algorithm(&[0x2b, 0x0e, 0x03, 0x02, 0x1a]),
        element(0x04, &[0x11; 20]),
        element(0x04, &[0x22; 20]),
        element(0x02, &[1]),
    ])
}

fn single_response() -> Vec<u8> {
    sequence(&[cert_id(), vec![0x80, 0x00], time(b"20261024123456Z")])
}

fn response_data(responder_id: Vec<u8>) -> Vec<u8> {
    sequence(&[
        responder_id,
        time(b"20261024120000Z"),
        sequence(&[single_response()]),
    ])
}

fn responder_by_name(label: &[u8]) -> Vec<u8> {
    element(0xa1, &name(label))
}

fn responder_by_key(key_hash: &[u8]) -> Vec<u8> {
    element(0xa2, &element(0x04, key_hash))
}

fn basic_response(responder_id: Vec<u8>) -> Vec<u8> {
    sequence(&[
        response_data(responder_id),
        algorithm(SIGNATURE_OID),
        element(0x03, &[0x00, 0xde, 0xad]),
    ])
}

fn certificate(subject_label: &[u8]) -> Vec<u8> {
    let validity = sequence(&[
        element(0x17, b"260101000000Z"),
        element(0x17, b"270101000000Z"),
    ]);
    let subject_public_key_info =
        sequence(&[algorithm(KEY_OID), element(0x03, &[0x00, 0x30, 0x00])]);
    let tbs = sequence(&[
        element(0xa0, &element(0x02, &[2])),
        element(0x02, &[1]),
        algorithm(SIGNATURE_OID),
        name(b"issuer"),
        validity,
        name(subject_label),
        subject_public_key_info,
    ]);
    sequence(&[
        tbs,
        algorithm(SIGNATURE_OID),
        element(0x03, &[0x00, 0xca, 0xfe]),
    ])
}

fn decode_response(encoded: &[u8]) -> X509OcspBasicResponse<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    decode_x509_ocsp_basic_response(&mut decoder, root).unwrap()
}

fn decode_certificate(encoded: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(encoded, Asn1Limits::default()).unwrap()
}

#[test]
fn binds_by_name_once_in_responder_to_subject_direction() {
    let response_der = basic_response(responder_by_name(b"responder"));
    let certificate_der = certificate(b"responder");
    let response = decode_response(&response_der);
    let certificate = decode_certificate(&certificate_der);
    let calls = Cell::new(0usize);

    let bound = bind_x509_ocsp_responder_id_to_certificate(
        &response,
        &certificate,
        |responder_name, certificate_subject| {
            calls.set(calls.get() + 1);
            assert_eq!(responder_name.encoded(), name(b"responder"));
            assert_eq!(
                certificate_subject.encoded(),
                certificate.tbs_certificate().subject().encoded()
            );
            responder_name.encoded() == certificate_subject.encoded()
        },
    )
    .unwrap();

    assert_eq!(calls.get(), 1);
    assert_eq!(bound.kind(), X509OcspResponderIdKind::ByName);
    assert!(std::ptr::eq(bound.response(), &response));
    assert!(std::ptr::eq(bound.certificate(), &certificate));
    assert_eq!(format!("{bound:?}"), "BoundX509OcspResponderCertificate");
}

#[test]
fn rejects_a_by_name_policy_mismatch_after_one_call() {
    let response_der = basic_response(responder_by_name(b"responder"));
    let certificate_der = certificate(b"other");
    let response = decode_response(&response_der);
    let certificate = decode_certificate(&certificate_der);
    let calls = Cell::new(0usize);

    let error = bind_x509_ocsp_responder_id_to_certificate(&response, &certificate, |_, _| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap_err();

    assert_eq!(calls.get(), 1);
    assert_eq!(
        error.kind(),
        X509OcspResponderIdBindingErrorKind::NameMismatch
    );
}

#[test]
fn binds_by_key_without_invoking_name_policy() {
    let response_der = basic_response(responder_by_key(RESPONDER_KEY_HASH));
    let certificate_der = certificate(b"responder");
    let response = decode_response(&response_der);
    let certificate = decode_certificate(&certificate_der);
    let calls = Cell::new(0usize);

    let bound = bind_x509_ocsp_responder_id_to_certificate(&response, &certificate, |_, _| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();

    assert_eq!(calls.get(), 0);
    assert_eq!(bound.kind(), X509OcspResponderIdKind::ByKey);
    assert!(std::ptr::eq(bound.response(), &response));
    assert!(std::ptr::eq(bound.certificate(), &certificate));
}

#[test]
fn by_key_requires_the_fixed_sha1_digest_length() {
    let certificate_der = certificate(b"responder");
    let certificate = decode_certificate(&certificate_der);

    for key_hash in [&RESPONDER_KEY_HASH[..19], &[0x44; 21]] {
        let response_der = basic_response(responder_by_key(key_hash));
        let response = decode_response(&response_der);
        let error = bind_x509_ocsp_responder_id_to_certificate(&response, &certificate, |_, _| {
            panic!("name policy must not run for byKey")
        })
        .unwrap_err();
        assert_eq!(
            error.kind(),
            X509OcspResponderIdBindingErrorKind::InvalidKeyHashLength
        );
    }
}

#[test]
fn by_key_rejects_every_complete_digest_mismatch_without_payloads() {
    let certificate_der = certificate(b"responder");
    let certificate = decode_certificate(&certificate_der);

    for index in [0, RESPONDER_KEY_HASH.len() - 1] {
        let mut mismatched = *RESPONDER_KEY_HASH;
        mismatched[index] ^= 0x01;
        let response_der = basic_response(responder_by_key(&mismatched));
        let response = decode_response(&response_der);
        let error = bind_x509_ocsp_responder_id_to_certificate(&response, &certificate, |_, _| {
            panic!("name policy must not run for byKey")
        })
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509OcspResponderIdBindingErrorKind::KeyHashMismatch
        );
        assert!(!format!("{error:?}").contains("f944"));
        assert!(!error.to_string().contains("f944"));
    }
}
