use der_asn1::Asn1Limits;
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    validate_certificate_role_extensions, X509CertificateRole, X509CertificateRoleErrorKind,
};
use x509_key_usage::KeyUsageBit;

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const BASIC_CONSTRAINTS_OID: &[u8] = &[0x55, 0x1d, 0x13];
const KEY_USAGE_OID: &[u8] = &[0x55, 0x1d, 0x0f];
const EXTENDED_KEY_USAGE_OID: &[u8] = &[0x55, 0x1d, 0x25];
const SERVER_AUTH_OID: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];

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

fn basic_constraints(is_ca: bool, critical: bool) -> Vec<u8> {
    let value = if is_ca {
        der(0x30, &der(0x01, &[0xff]))
    } else {
        der(0x30, &[])
    };
    extension(BASIC_CONSTRAINTS_OID, critical, &value)
}

fn key_usage(bit: u8, critical: bool) -> Vec<u8> {
    let (unused_bits, byte) = match bit {
        0 => (7, 0x80),
        5 => (2, 0x04),
        _ => panic!("unsupported test bit"),
    };
    extension(KEY_USAGE_OID, critical, &der(0x03, &[unused_bits, byte]))
}

fn extended_key_usage(critical: bool) -> Vec<u8> {
    extension(
        EXTENDED_KEY_USAGE_OID,
        critical,
        &der(0x30, &der(0x06, SERVER_AUTH_OID)),
    )
}

fn validate(
    extensions: &[Vec<u8>],
    role: X509CertificateRole,
) -> Result<
    x509_certificate_policy::ValidatedCertificateRoleExtensions<'_>,
    x509_certificate_policy::X509CertificateRoleError,
> {
    let encoded = certificate(extensions);
    let encoded = Box::leak(encoded.into_boxed_slice());
    let certificate = decode_x509_certificate(encoded, Asn1Limits::default()).unwrap();
    validate_certificate_role_extensions(&certificate, role, Asn1Limits::default())
}

#[test]
fn accepts_strict_issuing_ca_and_retains_bounded_evidence() {
    let extensions = [
        basic_constraints(true, true),
        key_usage(5, false),
        extended_key_usage(false),
    ];
    let evidence = validate(
        &extensions,
        X509CertificateRole::IssuingCertificateAuthority,
    )
    .unwrap();

    assert_eq!(
        evidence.role(),
        X509CertificateRole::IssuingCertificateAuthority
    );
    assert!(evidence.basic_constraints().unwrap().is_ca());
    assert!(evidence
        .key_usage()
        .unwrap()
        .contains(KeyUsageBit::KeyCertSign));
    assert!(evidence
        .extended_key_usage()
        .unwrap()
        .contains(&[1, 3, 6, 1, 5, 5, 7, 3, 1]));
}

#[test]
fn issuing_ca_requires_basic_constraints() {
    let error = validate(
        &[key_usage(5, true)],
        X509CertificateRole::IssuingCertificateAuthority,
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificateRoleErrorKind::MissingBasicConstraints
    );
    assert_eq!(error.extension_index(), None);
}

#[test]
fn issuing_ca_requires_critical_true_basic_constraints() {
    for (extensions, expected) in [
        (
            vec![basic_constraints(true, false), key_usage(5, true)],
            X509CertificateRoleErrorKind::BasicConstraintsNotCritical,
        ),
        (
            vec![basic_constraints(false, true), key_usage(5, true)],
            X509CertificateRoleErrorKind::CertificateAuthorityNotAsserted,
        ),
    ] {
        let error = validate(
            &extensions,
            X509CertificateRole::IssuingCertificateAuthority,
        )
        .unwrap_err();

        assert_eq!(error.kind(), expected);
        assert_eq!(error.extension_index(), Some(0));
    }
}

#[test]
fn issuing_ca_rejects_malformed_basic_constraints_without_disclosure() {
    let error = validate(
        &[
            extension(BASIC_CONSTRAINTS_OID, true, b"secret-payload"),
            key_usage(5, true),
        ],
        X509CertificateRole::IssuingCertificateAuthority,
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificateRoleErrorKind::InvalidBasicConstraints
    );
    assert_eq!(error.extension_index(), Some(0));
    assert!(!error.to_string().contains("secret-payload"));
    assert!(!error.to_string().contains("2.5.29"));
}

#[test]
fn issuing_ca_requires_certificate_signing_key_usage() {
    for (extensions, expected, index) in [
        (
            vec![basic_constraints(true, true)],
            X509CertificateRoleErrorKind::MissingKeyUsage,
            None,
        ),
        (
            vec![basic_constraints(true, true), key_usage(0, true)],
            X509CertificateRoleErrorKind::CertificateSigningNotAsserted,
            Some(1),
        ),
    ] {
        let error = validate(
            &extensions,
            X509CertificateRole::IssuingCertificateAuthority,
        )
        .unwrap_err();

        assert_eq!(error.kind(), expected);
        assert_eq!(error.extension_index(), index);
    }
}

#[test]
fn issuing_ca_rejects_malformed_key_usage() {
    let error = validate(
        &[
            basic_constraints(true, true),
            extension(KEY_USAGE_OID, false, &[0x03, 0x01, 0x00]),
        ],
        X509CertificateRole::IssuingCertificateAuthority,
    )
    .unwrap_err();

    assert_eq!(error.kind(), X509CertificateRoleErrorKind::InvalidKeyUsage);
    assert_eq!(error.extension_index(), Some(1));
}

#[test]
fn accepts_end_entity_without_role_extensions() {
    let evidence = validate(&[], X509CertificateRole::EndEntity).unwrap();

    assert_eq!(evidence.role(), X509CertificateRole::EndEntity);
    assert_eq!(evidence.basic_constraints(), None);
    assert_eq!(evidence.key_usage(), None);
    assert_eq!(evidence.extended_key_usage(), None);
}

#[test]
fn accepts_end_entity_role_evidence_without_authorizing_tls_usage() {
    let extensions = [
        basic_constraints(false, false),
        key_usage(0, true),
        extended_key_usage(true),
    ];
    let evidence = validate(&extensions, X509CertificateRole::EndEntity).unwrap();

    assert!(!evidence.basic_constraints().unwrap().is_ca());
    assert!(evidence
        .key_usage()
        .unwrap()
        .contains(KeyUsageBit::DigitalSignature));
    assert_eq!(evidence.extended_key_usage().unwrap().len(), 1);
}

#[test]
fn end_entity_rejects_ca_and_certificate_signing_assertions() {
    for (extensions, expected, index) in [
        (
            vec![basic_constraints(true, true)],
            X509CertificateRoleErrorKind::EndEntityAssertsCertificateAuthority,
            0,
        ),
        (
            vec![key_usage(5, true)],
            X509CertificateRoleErrorKind::EndEntityAssertsCertificateSigning,
            0,
        ),
    ] {
        let error = validate(&extensions, X509CertificateRole::EndEntity).unwrap_err();

        assert_eq!(error.kind(), expected);
        assert_eq!(error.extension_index(), Some(index));
    }
}

#[test]
fn every_present_known_role_extension_must_decode() {
    for role_extensions in [
        vec![extension(EXTENDED_KEY_USAGE_OID, false, &[0x30, 0x00])],
        vec![
            basic_constraints(true, true),
            key_usage(5, true),
            extension(EXTENDED_KEY_USAGE_OID, true, &[0x30, 0x00]),
        ],
    ] {
        let role = if role_extensions.len() == 1 {
            X509CertificateRole::EndEntity
        } else {
            X509CertificateRole::IssuingCertificateAuthority
        };
        let error = validate(&role_extensions, role).unwrap_err();

        assert_eq!(
            error.kind(),
            X509CertificateRoleErrorKind::InvalidExtendedKeyUsage
        );
        assert_eq!(error.extension_index(), Some(role_extensions.len() - 1));
    }
}

#[test]
fn one_semantic_budget_is_shared_across_known_extensions() {
    let encoded = certificate(&[
        basic_constraints(true, true),
        key_usage(5, true),
        extended_key_usage(false),
    ]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let limits = Asn1Limits {
        max_total_elements: 4,
        ..Asn1Limits::default()
    };

    let error = validate_certificate_role_extensions(
        &certificate,
        X509CertificateRole::IssuingCertificateAuthority,
        limits,
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificateRoleErrorKind::InvalidExtendedKeyUsage
    );
    assert_eq!(error.extension_index(), Some(2));
}
