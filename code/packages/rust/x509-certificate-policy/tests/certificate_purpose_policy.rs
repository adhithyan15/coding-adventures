use der_asn1::Asn1Limits;
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    validate_certificate_extended_key_purpose, validate_certificate_role_extensions,
    X509CertificatePurposeErrorKind, X509CertificatePurposePolicy,
    X509CertificatePurposePolicyDefinitionErrorKind, X509CertificateRole,
};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const BASIC_CONSTRAINTS_OID: &[u8] = &[0x55, 0x1d, 0x13];
const KEY_USAGE_OID: &[u8] = &[0x55, 0x1d, 0x0f];
const EXTENDED_KEY_USAGE_OID: &[u8] = &[0x55, 0x1d, 0x25];
const SERVER_AUTH_DER_OID: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];
const CLIENT_AUTH_DER_OID: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x02];
const ANY_EXTENDED_KEY_USAGE_DER_OID: &[u8] = &[0x55, 0x1d, 0x25, 0x00];

const SERVER_AUTH: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 3, 1];
const CLIENT_AUTH: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 3, 2];
const ANY_EXTENDED_KEY_USAGE: &[u64] = &[2, 5, 29, 37, 0];

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

fn basic_constraints_ca() -> Vec<u8> {
    extension(BASIC_CONSTRAINTS_OID, true, &der(0x30, &der(0x01, &[0xff])))
}

fn certificate_signing_key_usage() -> Vec<u8> {
    extension(KEY_USAGE_OID, true, &der(0x03, &[0x02, 0x04]))
}

fn extended_key_usage(purposes: &[&[u8]]) -> Vec<u8> {
    let values = purposes
        .iter()
        .flat_map(|purpose| der(0x06, purpose))
        .collect::<Vec<_>>();
    extension(EXTENDED_KEY_USAGE_OID, false, &der(0x30, &values))
}

fn role_evidence(
    extensions: &[Vec<u8>],
    role: X509CertificateRole,
) -> x509_certificate_policy::ValidatedCertificateRoleExtensions<'static> {
    let encoded = Box::leak(certificate(extensions).into_boxed_slice());
    let certificate = decode_x509_certificate(encoded, Asn1Limits::default()).unwrap();
    validate_certificate_role_extensions(&certificate, role, Asn1Limits::default()).unwrap()
}

#[test]
fn purpose_policy_rejects_an_empty_oid_without_retaining_policy_data() {
    let error = X509CertificatePurposePolicy::new(&[]).unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificatePurposePolicyDefinitionErrorKind::EmptyPurpose
    );
    assert_eq!(
        error.to_string(),
        "X.509 certificate purpose policy definition error EmptyPurpose"
    );
}

#[test]
fn exact_end_entity_purpose_is_authorized() {
    let evidence = role_evidence(
        &[extended_key_usage(&[
            CLIENT_AUTH_DER_OID,
            SERVER_AUTH_DER_OID,
        ])],
        X509CertificateRole::EndEntity,
    );
    let policy = X509CertificatePurposePolicy::new(SERVER_AUTH).unwrap();

    let _authorization = validate_certificate_extended_key_purpose(&evidence, policy).unwrap();
    assert_eq!(policy.purpose(), SERVER_AUTH);
}

#[test]
fn non_end_entity_role_is_rejected_before_a_matching_purpose() {
    let evidence = role_evidence(
        &[
            basic_constraints_ca(),
            certificate_signing_key_usage(),
            extended_key_usage(&[SERVER_AUTH_DER_OID]),
        ],
        X509CertificateRole::IssuingCertificateAuthority,
    );
    let policy = X509CertificatePurposePolicy::new(SERVER_AUTH).unwrap();

    let error = validate_certificate_extended_key_purpose(&evidence, policy).unwrap_err();
    assert_eq!(error.kind(), X509CertificatePurposeErrorKind::NotEndEntity);
}

#[test]
fn missing_extended_key_usage_fails_closed() {
    let evidence = role_evidence(&[], X509CertificateRole::EndEntity);
    let policy = X509CertificatePurposePolicy::new(SERVER_AUTH).unwrap();

    let error = validate_certificate_extended_key_purpose(&evidence, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificatePurposeErrorKind::MissingExtendedKeyUsage
    );
}

#[test]
fn a_different_exact_purpose_is_not_authorized_or_disclosed() {
    let evidence = role_evidence(
        &[extended_key_usage(&[CLIENT_AUTH_DER_OID])],
        X509CertificateRole::EndEntity,
    );
    let policy = X509CertificatePurposePolicy::new(SERVER_AUTH).unwrap();

    let error = validate_certificate_extended_key_purpose(&evidence, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificatePurposeErrorKind::PurposeNotAsserted
    );
    assert!(!error.to_string().contains("1.3.6.1.5.5.7.3.1"));
    assert!(!error.to_string().contains("1.3.6.1.5.5.7.3.2"));
}

#[test]
fn any_extended_key_usage_has_no_implicit_wildcard_semantics() {
    for (presented, requested) in [
        (ANY_EXTENDED_KEY_USAGE_DER_OID, SERVER_AUTH),
        (SERVER_AUTH_DER_OID, ANY_EXTENDED_KEY_USAGE),
    ] {
        let evidence = role_evidence(
            &[extended_key_usage(&[presented])],
            X509CertificateRole::EndEntity,
        );
        let policy = X509CertificatePurposePolicy::new(requested).unwrap();

        let error = validate_certificate_extended_key_purpose(&evidence, policy).unwrap_err();
        assert_eq!(
            error.kind(),
            X509CertificatePurposeErrorKind::PurposeNotAsserted
        );
    }

    let evidence = role_evidence(
        &[extended_key_usage(&[ANY_EXTENDED_KEY_USAGE_DER_OID])],
        X509CertificateRole::EndEntity,
    );
    let policy = X509CertificatePurposePolicy::new(ANY_EXTENDED_KEY_USAGE).unwrap();
    validate_certificate_extended_key_purpose(&evidence, policy).unwrap();
}

#[test]
fn exact_matching_does_not_confuse_adjacent_purpose_oids() {
    let evidence = role_evidence(
        &[extended_key_usage(&[SERVER_AUTH_DER_OID])],
        X509CertificateRole::EndEntity,
    );
    let policy = X509CertificatePurposePolicy::new(CLIENT_AUTH).unwrap();

    let error = validate_certificate_extended_key_purpose(&evidence, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificatePurposeErrorKind::PurposeNotAsserted
    );
}
