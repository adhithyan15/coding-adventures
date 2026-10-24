use der_asn1::{Asn1Decoder, Asn1Limits};
use tls_server_identity::{MatchKind, ReferenceIdentity};
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_path_construction::{
    construct_unique_x509_certificate_path_by, ConstructedX509CertificatePath,
};
use x509_certificate_path_server_policy_binding::{
    bind_constructed_x509_certificate_path_to_server_policy,
    X509CertificatePathServerPolicyBindingErrorKind,
};
use x509_certificate_policy::{
    validate_certificate_algorithm_policy, validate_certificate_role_policy,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile, X509CertificatePurposePolicy,
    X509CertificateRole,
};
use x509_server_certificate_policy::{
    validate_x509_server_certificate_policy, ValidatedX509ServerCertificatePolicy,
};
use x509_time::{decode_x509_time, X509Time};

const SIGNATURE_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const SIGNATURE_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const KEY_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const KEY_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];
const EXTENDED_KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x25];
const SUBJECT_ALT_NAME_OID_DER: &[u8] = &[0x55, 0x1d, 0x11];
const SERVER_AUTH_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];
const SERVER_AUTH_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 3, 1];
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

fn certificate(issuer: &[u8], subject: &[u8], extensions: &[Vec<u8>]) -> Vec<u8> {
    let mut validity = der(0x17, b"260101000000Z");
    validity.extend(der(0x17, b"270101000000Z"));

    let mut subject_public_key_info = algorithm(KEY_OID_DER);
    subject_public_key_info.extend(der(0x03, &[0x00, 0x30, 0x00]));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[0x01]));
    tbs.extend(algorithm(SIGNATURE_OID_DER));
    tbs.extend(name(issuer));
    tbs.extend(der(0x30, &validity));
    tbs.extend(name(subject));
    tbs.extend(der(0x30, &subject_public_key_info));
    if !extensions.is_empty() {
        tbs.extend(der(0xa3, &der(0x30, &extensions.concat())));
    }

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID_DER));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

fn server_certificate() -> Vec<u8> {
    certificate(
        b"issuer.example",
        b"api.example.test",
        &[
            extension(KEY_USAGE_OID_DER, true, &der(0x03, &[0x07, 0x80])),
            extension(
                EXTENDED_KEY_USAGE_OID_DER,
                false,
                &der(0x30, &der(0x06, SERVER_AUTH_OID_DER)),
            ),
            extension(
                SUBJECT_ALT_NAME_OID_DER,
                false,
                &der(0x30, &der(0x82, b"api.example.test")),
            ),
        ],
    )
}

fn issuer_certificate() -> Vec<u8> {
    certificate(b"root.example", b"issuer.example", &[])
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

fn observed_time() -> X509Time {
    let encoded = der(0x17, b"260701120000Z");
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(&encoded).unwrap()).unwrap()
}

fn server_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> ValidatedX509ServerCertificatePolicy<'certificate, 'encoded> {
    let role_policy = validate_certificate_role_policy(
        certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(certificate, &algorithm_profiles()).unwrap();
    let reference = ReferenceIdentity::parse("api.example.test").unwrap();
    let purpose = X509CertificatePurposePolicy::new(SERVER_AUTH_OID).unwrap();

    validate_x509_server_certificate_policy(
        role_policy,
        algorithm_policy,
        &reference,
        &observed_time(),
        purpose,
        Asn1Limits::default(),
    )
    .unwrap()
}

fn constructed_path<'target, 'target_encoded, 'candidates, 'candidate_encoded>(
    target: &'target X509Certificate<'target_encoded>,
    candidates: &'candidates [X509Certificate<'candidate_encoded>],
) -> ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded> {
    construct_unique_x509_certificate_path_by(
        target,
        candidates,
        4,
        4,
        |certificate| std::ptr::eq(certificate, &candidates[0]),
        |issuer, subject| issuer.encoded() == subject.encoded(),
    )
    .unwrap()
}

#[test]
fn binds_complete_server_policy_to_the_exact_constructed_target() {
    let target_encoded = server_certificate();
    let issuer_encoded = issuer_certificate();
    let target = decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let candidates = [decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap()];
    let path = constructed_path(&target, &candidates);
    let policy = server_policy(&target);

    let bound = bind_constructed_x509_certificate_path_to_server_policy(path, policy).unwrap();

    assert_eq!(bound.constructed_path().certificate_count(), 2);
    assert!(std::ptr::eq(
        bound.constructed_path().target_certificate(),
        bound.server_policy().certificate()
    ));
    assert_eq!(
        bound.server_policy().identity().match_kind(),
        MatchKind::ExactDns
    );
    assert_eq!(
        format!("{bound:?}"),
        "BoundConstructedX509CertificatePathToServerPolicy"
    );
}

#[test]
fn rejects_byte_equal_server_policy_for_a_distinct_target_decode() {
    let target_encoded = server_certificate();
    let issuer_encoded = issuer_certificate();
    let target = decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let independently_decoded_target =
        decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let candidates = [decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap()];
    let path = constructed_path(&target, &candidates);
    let wrong_policy = server_policy(&independently_decoded_target);

    let error = bind_constructed_x509_certificate_path_to_server_policy(path, wrong_policy)
        .expect_err("byte-equal certificates from distinct decodes must not bind");

    assert_eq!(
        error.kind(),
        X509CertificatePathServerPolicyBindingErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 certificate path server-policy binding error EvidenceMismatch"
    );
    assert!(!error.to_string().contains("api.example.test"));
}
