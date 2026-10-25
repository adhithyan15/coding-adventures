use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    validate_certificate_algorithm_policy, validate_certificate_role_policy,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile,
    X509CertificatePurposeErrorKind, X509CertificateRole, X509CertificateValidityErrorKind,
};
use x509_ocsp_responder_certificate_policy::{
    validate_x509_ocsp_responder_certificate_policy, X509OcspResponderCertificatePolicyErrorKind,
};
use x509_time::{decode_x509_time, X509Time};

const SIGNATURE_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const SIGNATURE_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const KEY_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const KEY_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];
const BASIC_CONSTRAINTS_OID_DER: &[u8] = &[0x55, 0x1d, 0x13];
const EXTENDED_KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x25];
const OCSP_SIGNING_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x09];
const SERVER_AUTH_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];
const NULL_DER: &[u8] = &[0x05, 0x00];

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
    fields.extend(NULL_DER);
    der(0x30, &fields)
}

fn name(value: &[u8]) -> Vec<u8> {
    let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(der(0x0c, value));
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

    let mut subject_public_key_info = algorithm(KEY_OID_DER);
    subject_public_key_info.extend(der(0x03, &[0x00, 0x30, 0x00]));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[0x01]));
    tbs.extend(algorithm(SIGNATURE_OID_DER));
    tbs.extend(name(b"issuer.example"));
    tbs.extend(der(0x30, &validity));
    tbs.extend(name(b"ocsp.example"));
    tbs.extend(der(0x30, &subject_public_key_info));
    tbs.extend(der(0xa3, &der(0x30, &extensions.concat())));

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID_DER));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

fn responder_certificate_with_key_usage(
    purpose_oid: Option<&[u8]>,
    key_usage_value: Option<&[u8]>,
) -> Vec<u8> {
    let mut extensions = Vec::new();
    if let Some(key_usage_value) = key_usage_value {
        extensions.push(extension(
            KEY_USAGE_OID_DER,
            true,
            &der(0x03, key_usage_value),
        ));
    }
    if let Some(oid) = purpose_oid {
        extensions.push(extension(
            EXTENDED_KEY_USAGE_OID_DER,
            false,
            &der(0x30, &der(0x06, oid)),
        ));
    }
    certificate(&extensions)
}

fn responder_certificate(purpose_oid: Option<&[u8]>) -> Vec<u8> {
    responder_certificate_with_key_usage(purpose_oid, Some(&[0x07, 0x80]))
}

fn issuing_ca_certificate() -> Vec<u8> {
    certificate(&[
        extension(
            BASIC_CONSTRAINTS_OID_DER,
            true,
            &der(0x30, &der(0x01, &[0xff])),
        ),
        extension(KEY_USAGE_OID_DER, true, &der(0x03, &[0x02, 0x04])),
    ])
}

fn algorithm_profiles() -> [X509CertificateAlgorithmProfile<'static>; 1] {
    [X509CertificateAlgorithmProfile::new(
        SIGNATURE_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        KEY_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        16,
        16,
        16,
        16,
    )
    .unwrap()]
}

fn time(value: &[u8; 13]) -> X509Time {
    let encoded = der(0x17, value);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(&encoded).unwrap()).unwrap()
}

fn observed_time() -> X509Time {
    time(b"260701120000Z")
}

#[test]
fn retains_all_responder_policy_evidence_for_one_exact_certificate() {
    let encoded = responder_certificate(Some(OCSP_SIGNING_OID_DER));
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();

    let evidence = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &observed_time(),
    )
    .unwrap();

    assert!(std::ptr::eq(evidence.certificate(), &certificate));
    assert!(std::ptr::eq(
        evidence.role_policy().certificate(),
        evidence.algorithm_policy().certificate()
    ));
    assert_eq!(
        evidence
            .algorithm_policy()
            .algorithm_profile()
            .profile_index(),
        0
    );
    assert_eq!(
        format!("{:?}", evidence.validity()),
        "ValidatedCertificateValidity { private: () }"
    );
    assert_eq!(
        format!("{:?}", evidence.purpose()),
        "ValidatedCertificatePurpose { private: () }"
    );
    assert_eq!(
        format!("{evidence:?}"),
        "ValidatedX509OcspResponderCertificatePolicy"
    );
}

#[test]
fn rejects_byte_equal_evidence_from_distinct_decoded_objects() {
    let encoded = responder_certificate(Some(OCSP_SIGNING_OID_DER));
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let other_certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&other_certificate, &algorithm_profiles()).unwrap();

    let error = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &observed_time(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspResponderCertificatePolicyErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP responder certificate policy error EvidenceMismatch"
    );
}

#[test]
fn rejects_an_issuing_ca_before_derived_policy() {
    let encoded = issuing_ca_certificate();
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 19]) || oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();

    let error = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &observed_time(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspResponderCertificatePolicyErrorKind::NotEndEntity
    );
}

#[test]
fn rejects_key_encipherment_only_usage() {
    let encoded =
        responder_certificate_with_key_usage(Some(OCSP_SIGNING_OID_DER), Some(&[0x05, 0x20]));
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();

    let error = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &observed_time(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspResponderCertificatePolicyErrorKind::IncompatibleKeyUsage
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP responder certificate policy error IncompatibleKeyUsage"
    );
}

#[test]
fn accepts_content_commitment_only_usage() {
    let encoded =
        responder_certificate_with_key_usage(Some(OCSP_SIGNING_OID_DER), Some(&[0x06, 0x40]));
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();

    validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &observed_time(),
    )
    .unwrap();
}

#[test]
fn accepts_absent_key_usage() {
    let encoded = responder_certificate_with_key_usage(Some(OCSP_SIGNING_OID_DER), None);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |_| false,
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();

    validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &observed_time(),
    )
    .unwrap();
}

#[test]
fn maps_expired_validity_without_disclosing_time() {
    let encoded = responder_certificate(Some(OCSP_SIGNING_OID_DER));
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();

    let error = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &time(b"280101000000Z"),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspResponderCertificatePolicyErrorKind::Validity(
            X509CertificateValidityErrorKind::Expired
        )
    );
    assert!(!error.to_string().contains("280101"));
}

#[test]
fn maps_missing_ocsp_signing_purpose() {
    let encoded = responder_certificate(None);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();

    let error = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &observed_time(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspResponderCertificatePolicyErrorKind::Purpose(
            X509CertificatePurposeErrorKind::MissingExtendedKeyUsage
        )
    );
}

#[test]
fn maps_wrong_purpose_without_disclosing_oid() {
    let encoded = responder_certificate(Some(SERVER_AUTH_OID_DER));
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();

    let error = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &observed_time(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspResponderCertificatePolicyErrorKind::Purpose(
            X509CertificatePurposeErrorKind::PurposeNotAsserted
        )
    );
    assert!(!error.to_string().contains("1.3.6.1.5.5.7.3.9"));
}
