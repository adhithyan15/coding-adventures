use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    classify_certificate_self_issued, validate_certificate_algorithm_policy,
    validate_certificate_role_policy, X509AlgorithmParametersPolicy,
    X509CertificateAlgorithmProfile, X509CertificateRole, X509CertificateValidityErrorKind,
};
use x509_intermediate_certificate_policy::{
    validate_x509_intermediate_certificate_policy, X509IntermediateCertificatePolicyErrorKind,
};
use x509_time::{decode_x509_time, X509Time};

const SIGNATURE_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const SIGNATURE_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const KEY_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const KEY_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];
const BASIC_CONSTRAINTS_OID_DER: &[u8] = &[0x55, 0x1d, 0x13];
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

fn certificate(issuing_ca: bool) -> Vec<u8> {
    let mut validity = der(0x17, b"260101000000Z");
    validity.extend(der(0x17, b"270101000000Z"));

    let mut subject_public_key_info = algorithm(KEY_OID_DER);
    subject_public_key_info.extend(der(0x03, &[0x00, 0x30, 0x00]));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[0x01]));
    tbs.extend(algorithm(SIGNATURE_OID_DER));
    tbs.extend(name(b"issuer.example"));
    tbs.extend(der(0x30, &validity));
    tbs.extend(name(b"intermediate.example"));
    tbs.extend(der(0x30, &subject_public_key_info));
    if issuing_ca {
        let extensions = [
            extension(
                BASIC_CONSTRAINTS_OID_DER,
                true,
                &der(0x30, &der(0x01, &[0xff])),
            ),
            extension(KEY_USAGE_OID_DER, true, &der(0x03, &[0x02, 0x04])),
        ];
        tbs.extend(der(0xa3, &der(0x30, &extensions.concat())));
    }

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID_DER));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
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

#[test]
fn retains_all_intermediate_policy_evidence_for_one_exact_certificate() {
    let encoded = certificate(true);
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
    let classification = classify_certificate_self_issued(&certificate, |_issuer, _subject| false);

    let evidence = validate_x509_intermediate_certificate_policy(
        role_policy,
        algorithm_policy,
        classification,
        &time(b"260701120000Z"),
    )
    .unwrap();

    assert!(std::ptr::eq(evidence.certificate(), &certificate));
    assert!(std::ptr::eq(
        evidence.role_policy().certificate(),
        evidence.algorithm_policy().certificate()
    ));
    assert!(std::ptr::eq(
        evidence.role_policy().certificate(),
        evidence.self_issued_classification().certificate()
    ));
    assert!(!evidence.self_issued_classification().is_self_issued());
    assert_eq!(
        evidence
            .algorithm_policy()
            .algorithm_profile()
            .profile_index(),
        0
    );
    let _validity = evidence.validity();
    assert_eq!(
        format!("{evidence:?}"),
        "ValidatedX509IntermediateCertificatePolicy"
    );
}

#[test]
fn rejects_algorithm_evidence_from_a_byte_equal_distinct_object() {
    let encoded = certificate(true);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let other = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |_| true,
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&other, &algorithm_profiles()).unwrap();
    let classification = classify_certificate_self_issued(&certificate, |_, _| false);

    let error = validate_x509_intermediate_certificate_policy(
        role_policy,
        algorithm_policy,
        classification,
        &time(b"260701120000Z"),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509IntermediateCertificatePolicyErrorKind::EvidenceMismatch
    );
}

#[test]
fn rejects_self_issued_classification_from_a_byte_equal_distinct_object() {
    let encoded = certificate(true);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let other = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |_| true,
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();
    let classification = classify_certificate_self_issued(&other, |_, _| false);

    let error = validate_x509_intermediate_certificate_policy(
        role_policy,
        algorithm_policy,
        classification,
        &time(b"260701120000Z"),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509IntermediateCertificatePolicyErrorKind::EvidenceMismatch
    );
}

#[test]
fn rejects_end_entity_role_before_validity_policy() {
    let encoded = certificate(false);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |_| true,
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();
    let classification = classify_certificate_self_issued(&certificate, |_, _| false);

    let error = validate_x509_intermediate_certificate_policy(
        role_policy,
        algorithm_policy,
        classification,
        &time(b"250101000000Z"),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509IntermediateCertificatePolicyErrorKind::NotIssuingCertificateAuthority
    );
}

#[test]
fn maps_and_redacts_explicit_time_validity_failures() {
    for (observation, expected) in [
        (
            b"250101000000Z",
            X509CertificateValidityErrorKind::NotYetValid,
        ),
        (b"280101000000Z", X509CertificateValidityErrorKind::Expired),
    ] {
        let encoded = certificate(true);
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let role_policy = validate_certificate_role_policy(
            &certificate,
            X509CertificateRole::IssuingCertificateAuthority,
            Asn1Limits::default(),
            |_| true,
        )
        .unwrap();
        let algorithm_policy =
            validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();
        let classification = classify_certificate_self_issued(&certificate, |_, _| true);

        let error = validate_x509_intermediate_certificate_policy(
            role_policy,
            algorithm_policy,
            classification,
            &time(observation),
        )
        .unwrap_err();

        assert_eq!(
            error.kind(),
            X509IntermediateCertificatePolicyErrorKind::Validity(expected)
        );
        assert_eq!(
            error.to_string(),
            format!("X.509 intermediate certificate policy error Validity({expected:?})")
        );
        let debug = format!("{error:?}");
        assert!(!debug.contains("issuer.example"));
        assert!(!debug.contains("intermediate.example"));
        assert!(!debug.contains("250101"));
        assert!(!debug.contains("280101"));
    }
}

#[test]
fn inclusive_validity_endpoints_are_accepted() {
    for observation in [b"260101000000Z", b"270101000000Z"] {
        let encoded = certificate(true);
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let role_policy = validate_certificate_role_policy(
            &certificate,
            X509CertificateRole::IssuingCertificateAuthority,
            Asn1Limits::default(),
            |_| true,
        )
        .unwrap();
        let algorithm_policy =
            validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();
        let classification = classify_certificate_self_issued(&certificate, |_, _| true);

        validate_x509_intermediate_certificate_policy(
            role_policy,
            algorithm_policy,
            classification,
            &time(observation),
        )
        .unwrap();
    }
}
