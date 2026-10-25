use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_ocsp_cert_id_binding::{
    bind_x509_ocsp_cert_id_to_certificates, X509OcspCertIdBindingErrorKind, X509OcspCertIdHashInput,
};
use x509_ocsp_single_response::{decode_x509_ocsp_single_response, X509OcspSingleResponse};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const HASH_OID: &[u8] = &[0x2b, 0x0e, 0x03, 0x02, 0x1a];
const NAME_HASH: [u8; 4] = [0x11; 4];
const KEY_HASH: [u8; 4] = [0x22; 4];
const ISSUER_NAME: &[u8] = b"issuer-subject";
const ISSUER_KEY: &[u8] = &[0x30, 0x00];

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

fn single_response(serial: &[u8], issuer_name_hash: &[u8], issuer_key_hash: &[u8]) -> Vec<u8> {
    let cert_id = sequence(&[
        algorithm(HASH_OID),
        element(0x04, issuer_name_hash),
        element(0x04, issuer_key_hash),
        element(0x02, serial),
    ]);
    sequence(&[cert_id, vec![0x80, 0x00], time(b"20261024123456Z")])
}

fn certificate(serial: &[u8], issuer_label: &[u8], subject_label: &[u8], key: &[u8]) -> Vec<u8> {
    let validity = sequence(&[
        element(0x17, b"260101000000Z"),
        element(0x17, b"270101000000Z"),
    ]);
    let subject_public_key_info =
        sequence(&[algorithm(KEY_OID), element(0x03, &[&[0x00], key].concat())]);
    let tbs = sequence(&[
        element(0xa0, &element(0x02, &[2])),
        element(0x02, serial),
        algorithm(SIGNATURE_OID),
        name(issuer_label),
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

fn decode_response(encoded: &[u8]) -> X509OcspSingleResponse<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    decode_x509_ocsp_single_response(&mut decoder, root).unwrap()
}

fn decode_certificate(encoded: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(encoded, Asn1Limits::default()).unwrap()
}

fn fixtures() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        single_response(&[2], &NAME_HASH, &KEY_HASH),
        certificate(&[2], ISSUER_NAME, b"target", &[0x01]),
        certificate(&[1], b"root", ISSUER_NAME, ISSUER_KEY),
    )
}

#[test]
fn binds_exact_response_target_and_issuer_in_stable_hash_order() {
    let (response_der, target_der, issuer_der) = fixtures();
    let response = decode_response(&response_der);
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    let expected_algorithm = response.cert_id().hash_algorithm();
    let mut calls = 0usize;

    let bound =
        bind_x509_ocsp_cert_id_to_certificates(&response, &target, &issuer, |algorithm, input| {
            assert_eq!(algorithm, expected_algorithm);
            calls += 1;
            match (calls, input) {
                (1, X509OcspCertIdHashInput::IssuerNameDer(bytes)) => {
                    assert_eq!(bytes, name(ISSUER_NAME));
                    assert_eq!(input.bytes(), bytes);
                    Some(NAME_HASH)
                }
                (2, X509OcspCertIdHashInput::IssuerSubjectPublicKey(bytes)) => {
                    assert_eq!(bytes, ISSUER_KEY);
                    assert_eq!(input.bytes(), bytes);
                    Some(KEY_HASH)
                }
                _ => panic!("hash policy call order changed"),
            }
        })
        .unwrap();

    assert_eq!(calls, 2);
    assert!(std::ptr::eq(bound.response(), &response));
    assert!(std::ptr::eq(bound.target(), &target));
    assert!(std::ptr::eq(bound.issuer(), &issuer));
    assert_eq!(format!("{bound:?}"), "BoundX509OcspCertId");
}

#[test]
fn rejects_zero_or_mismatched_public_hash_lengths_without_policy_calls() {
    let (_, target_der, issuer_der) = fixtures();
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    let mut calls = 0usize;

    let empty_der = single_response(&[2], &[], &[]);
    let empty = decode_response(&empty_der);
    let error = bind_x509_ocsp_cert_id_to_certificates(&empty, &target, &issuer, |_, _| {
        calls += 1;
        Some([])
    })
    .unwrap_err();
    assert_eq!(
        error.kind(),
        X509OcspCertIdBindingErrorKind::InvalidHashLength
    );

    let mismatched_der = single_response(&[2], &[0x11; 3], &KEY_HASH);
    let mismatched = decode_response(&mismatched_der);
    let error = bind_x509_ocsp_cert_id_to_certificates(&mismatched, &target, &issuer, |_, _| {
        calls += 1;
        Some(NAME_HASH)
    })
    .unwrap_err();
    assert_eq!(
        error.kind(),
        X509OcspCertIdBindingErrorKind::InvalidHashLength
    );
    assert_eq!(calls, 0);
}

#[test]
fn rejects_target_serial_mismatch_before_hash_policy() {
    let (response_der, _, issuer_der) = fixtures();
    let target_der = certificate(&[3], ISSUER_NAME, b"target", &[0x01]);
    let response = decode_response(&response_der);
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    let mut calls = 0usize;

    let error = bind_x509_ocsp_cert_id_to_certificates(&response, &target, &issuer, |_, _| {
        calls += 1;
        Some(NAME_HASH)
    })
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspCertIdBindingErrorKind::SerialNumberMismatch
    );
    assert_eq!(calls, 0);
}

#[test]
fn reports_name_hash_policy_rejection_without_payloads() {
    let (response_der, target_der, issuer_der) = fixtures();
    let response = decode_response(&response_der);
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    let mut calls = 0usize;

    let error = bind_x509_ocsp_cert_id_to_certificates(&response, &target, &issuer, |_, input| {
        calls += 1;
        assert!(matches!(input, X509OcspCertIdHashInput::IssuerNameDer(_)));
        None::<[u8; 4]>
    })
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspCertIdBindingErrorKind::IssuerNameHashPolicyRejected
    );
    assert_eq!(calls, 1);
    assert!(!format!("{error:?}").contains("issuer-subject"));
    assert!(!error.to_string().contains("issuer-subject"));
}

#[test]
fn rejects_complete_name_hash_mismatches_before_key_policy() {
    let (response_der, target_der, issuer_der) = fixtures();
    let response = decode_response(&response_der);
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);

    for index in [0, NAME_HASH.len() - 1] {
        let mut computed = NAME_HASH;
        computed[index] ^= 0x01;
        let mut calls = 0usize;
        let error =
            bind_x509_ocsp_cert_id_to_certificates(&response, &target, &issuer, |_, input| {
                calls += 1;
                assert!(matches!(input, X509OcspCertIdHashInput::IssuerNameDer(_)));
                Some(computed)
            })
            .unwrap_err();
        assert_eq!(
            error.kind(),
            X509OcspCertIdBindingErrorKind::IssuerNameHashMismatch
        );
        assert_eq!(calls, 1);
    }
}

#[test]
fn reports_key_hash_policy_rejection_after_name_match() {
    let (response_der, target_der, issuer_der) = fixtures();
    let response = decode_response(&response_der);
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    let mut calls = 0usize;

    let error = bind_x509_ocsp_cert_id_to_certificates(&response, &target, &issuer, |_, input| {
        calls += 1;
        match input {
            X509OcspCertIdHashInput::IssuerNameDer(_) => Some(NAME_HASH),
            X509OcspCertIdHashInput::IssuerSubjectPublicKey(_) => None,
        }
    })
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspCertIdBindingErrorKind::IssuerKeyHashPolicyRejected
    );
    assert_eq!(calls, 2);
}

#[test]
fn rejects_complete_key_hash_mismatches_after_name_match() {
    let (response_der, target_der, issuer_der) = fixtures();
    let response = decode_response(&response_der);
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);

    for index in [0, KEY_HASH.len() - 1] {
        let mut computed = KEY_HASH;
        computed[index] ^= 0x01;
        let mut calls = 0usize;
        let error =
            bind_x509_ocsp_cert_id_to_certificates(&response, &target, &issuer, |_, input| {
                calls += 1;
                match input {
                    X509OcspCertIdHashInput::IssuerNameDer(_) => Some(NAME_HASH),
                    X509OcspCertIdHashInput::IssuerSubjectPublicKey(_) => Some(computed),
                }
            })
            .unwrap_err();
        assert_eq!(
            error.kind(),
            X509OcspCertIdBindingErrorKind::IssuerKeyHashMismatch
        );
        assert_eq!(calls, 2);
    }
}
