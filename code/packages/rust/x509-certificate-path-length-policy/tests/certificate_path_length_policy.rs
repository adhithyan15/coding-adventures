use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_path_length_policy::{
    X509CertificatePathLengthErrorKind, X509CertificatePathLengthState,
};
use x509_certificate_policy::{
    classify_certificate_self_issued, validate_certificate_algorithm_policy,
    validate_certificate_role_extensions, validate_certificate_role_policy,
    ValidatedCertificateRoleExtensions, X509AlgorithmParametersPolicy,
    X509CertificateAlgorithmProfile, X509CertificateRole,
};
use x509_intermediate_certificate_policy::{
    validate_x509_intermediate_certificate_policy, ValidatedX509IntermediateCertificatePolicy,
};
use x509_time::{decode_x509_time, X509Time};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const SIGNATURE_OID_ARCS: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const KEY_OID_ARCS: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const BASIC_CONSTRAINTS_OID: &[u8] = &[0x55, 0x1d, 0x13];
const KEY_USAGE_OID: &[u8] = &[0x55, 0x1d, 0x0f];
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
        tbs.extend(der(0xa3, &der(0x30, &extensions.concat())));
    }

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

fn basic_constraints(path_length: Option<&[u8]>) -> Vec<u8> {
    let mut fields = der(0x01, &[0xff]);
    if let Some(path_length) = path_length {
        fields.extend(der(0x02, path_length));
    }
    extension(BASIC_CONSTRAINTS_OID, true, &der(0x30, &fields))
}

fn key_cert_sign() -> Vec<u8> {
    extension(KEY_USAGE_OID, true, &der(0x03, &[2, 0x04]))
}

fn role_evidence(
    role: X509CertificateRole,
    path_length: Option<&[u8]>,
) -> ValidatedCertificateRoleExtensions<'static> {
    let certificate = decoded_certificate(role, path_length);
    validate_certificate_role_extensions(&certificate, role, Asn1Limits::default()).unwrap()
}

fn decoded_certificate(
    role: X509CertificateRole,
    path_length: Option<&[u8]>,
) -> X509Certificate<'static> {
    let extensions = match role {
        X509CertificateRole::IssuingCertificateAuthority => {
            vec![basic_constraints(path_length), key_cert_sign()]
        }
        X509CertificateRole::EndEntity => Vec::new(),
    };
    let encoded = Box::leak(certificate(&extensions).into_boxed_slice());
    decode_x509_certificate(encoded, Asn1Limits::default()).unwrap()
}

fn algorithm_profiles() -> [X509CertificateAlgorithmProfile<'static>; 1] {
    [X509CertificateAlgorithmProfile::new(
        SIGNATURE_OID_ARCS,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        KEY_OID_ARCS,
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

fn validated_intermediate_policy(
    is_self_issued: bool,
    path_length: Option<&[u8]>,
) -> ValidatedX509IntermediateCertificatePolicy<'static, 'static> {
    let certificate = Box::leak(Box::new(decoded_certificate(
        X509CertificateRole::IssuingCertificateAuthority,
        path_length,
    )));
    let role_policy = validate_certificate_role_policy(
        certificate,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 19]) || oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(certificate, &algorithm_profiles()).unwrap();
    let classification =
        classify_certificate_self_issued(certificate, |_issuer, _subject| is_self_issued);

    validate_x509_intermediate_certificate_policy(
        role_policy,
        algorithm_policy,
        classification,
        &time(b"260701120000Z"),
    )
    .unwrap()
}

#[test]
fn complete_non_self_issued_intermediate_policy_advances_and_tightens_state() {
    let intermediate = validated_intermediate_policy(false, Some(&[0x00]));

    let state = X509CertificatePathLengthState::new(3)
        .advance_validated_intermediate_ca(&intermediate)
        .unwrap();

    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 0);
}

#[test]
fn complete_self_issued_intermediate_policy_skips_only_the_decrement() {
    let intermediate = validated_intermediate_policy(true, Some(&[0x02]));

    let state = X509CertificatePathLengthState::new(3)
        .advance_validated_intermediate_ca(&intermediate)
        .unwrap();

    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 2);
}

#[test]
fn classified_self_issued_ca_skips_decrement_for_the_same_bound_certificate() {
    let certificate = decoded_certificate(X509CertificateRole::IssuingCertificateAuthority, None);
    let classification = classify_certificate_self_issued(&certificate, |issuer, subject| {
        issuer.encoded() == subject.encoded()
    });

    let state = X509CertificatePathLengthState::new(1)
        .advance_classified_intermediate_ca(&classification, Asn1Limits::default())
        .unwrap();

    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 1);
}

#[test]
fn classified_non_self_issued_ca_consumes_the_budget() {
    let certificate = decoded_certificate(X509CertificateRole::IssuingCertificateAuthority, None);
    let classification = classify_certificate_self_issued(&certificate, |_issuer, _subject| false);

    let state = X509CertificatePathLengthState::new(1)
        .advance_classified_intermediate_ca(&classification, Asn1Limits::default())
        .unwrap();

    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 0);
}

#[test]
fn classified_end_entity_fails_before_path_length_transition() {
    let certificate = decoded_certificate(X509CertificateRole::EndEntity, None);
    let classification = classify_certificate_self_issued(&certificate, |_issuer, _subject| true);

    let error = X509CertificatePathLengthState::new(1)
        .advance_classified_intermediate_ca(&classification, Asn1Limits::default())
        .unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificatePathLengthErrorKind::NotIssuingCertificateAuthority
    );
}

#[test]
fn non_self_issued_intermediates_consume_the_budget_and_exhaustion_fails_closed() {
    let ca = role_evidence(X509CertificateRole::IssuingCertificateAuthority, None);
    let state = X509CertificatePathLengthState::new(1);
    let state = state.advance_intermediate_ca(&ca, false).unwrap();

    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 0);
    let error = state.advance_intermediate_ca(&ca, false).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificatePathLengthErrorKind::MaximumExceeded
    );
    assert_eq!(
        error.to_string(),
        "X.509 certificate path length policy error MaximumExceeded"
    );
}

#[test]
fn self_issued_intermediate_does_not_consume_the_budget() {
    let ca = role_evidence(X509CertificateRole::IssuingCertificateAuthority, None);
    let state = X509CertificatePathLengthState::new(1)
        .advance_intermediate_ca(&ca, true)
        .unwrap();

    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 1);
    let state = state.advance_intermediate_ca(&ca, false).unwrap();
    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 0);
}

#[test]
fn self_issued_intermediate_still_tightens_from_its_constraint() {
    let zero = role_evidence(
        X509CertificateRole::IssuingCertificateAuthority,
        Some(&[0x00]),
    );
    let unconstrained = role_evidence(X509CertificateRole::IssuingCertificateAuthority, None);
    let state = X509CertificatePathLengthState::new(3)
        .advance_intermediate_ca(&zero, true)
        .unwrap();

    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 0);
    assert_eq!(
        state
            .advance_intermediate_ca(&unconstrained, false)
            .unwrap_err()
            .kind(),
        X509CertificatePathLengthErrorKind::MaximumExceeded
    );
}

#[test]
fn each_constraint_can_only_tighten_the_remaining_budget() {
    let two = role_evidence(
        X509CertificateRole::IssuingCertificateAuthority,
        Some(&[0x02]),
    );
    let ten = role_evidence(
        X509CertificateRole::IssuingCertificateAuthority,
        Some(&[0x0a]),
    );
    let unconstrained = role_evidence(X509CertificateRole::IssuingCertificateAuthority, None);

    let state = X509CertificatePathLengthState::new(5)
        .advance_intermediate_ca(&two, false)
        .unwrap();
    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 2);

    let state = state.advance_intermediate_ca(&ten, true).unwrap();
    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 2);

    let state = state
        .advance_intermediate_ca(&unconstrained, false)
        .unwrap();
    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 1);
}

#[test]
fn path_length_zero_allows_the_current_final_intermediate_but_no_next_one() {
    let zero = role_evidence(
        X509CertificateRole::IssuingCertificateAuthority,
        Some(&[0x00]),
    );
    let unconstrained = role_evidence(X509CertificateRole::IssuingCertificateAuthority, None);

    let state = X509CertificatePathLengthState::new(4)
        .advance_intermediate_ca(&zero, false)
        .unwrap();
    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 0);
    assert_eq!(
        state
            .advance_intermediate_ca(&unconstrained, false)
            .unwrap_err()
            .kind(),
        X509CertificatePathLengthErrorKind::MaximumExceeded
    );
}

#[test]
fn constraint_larger_than_the_local_counter_cannot_loosen_it() {
    let larger_than_u64 = role_evidence(
        X509CertificateRole::IssuingCertificateAuthority,
        Some(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    );
    let state = X509CertificatePathLengthState::new(3)
        .advance_intermediate_ca(&larger_than_u64, true)
        .unwrap();

    assert_eq!(state.remaining_non_self_issued_intermediate_cas(), 3);
}

#[test]
fn requires_issuing_ca_role_evidence_without_disclosing_input() {
    let end_entity = role_evidence(X509CertificateRole::EndEntity, None);
    let error = X509CertificatePathLengthState::new(3)
        .advance_intermediate_ca(&end_entity, false)
        .unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificatePathLengthErrorKind::NotIssuingCertificateAuthority
    );
    assert_eq!(
        format!("{error:?}"),
        "X509CertificatePathLengthError { kind: NotIssuingCertificateAuthority }"
    );
}
