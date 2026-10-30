use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_policy::{
    validate_certificate_algorithm_policy, validate_certificate_role_policy,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile, X509CertificateRole,
};
use x509_rsa_algorithm_policy::{
    validate_x509_rsa_algorithm_policy, ValidatedX509RsaAlgorithmPolicy,
};
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_sha384_issuer_link::{
    verify_x509_rsa_sha384_issuer_link_with_policies, VerifiedX509RsaSha384IssuerPolicyLink,
};
use x509_rsa_sha384_reviewed_anchor_link::{
    bind_x509_rsa_sha384_issuer_link_to_reviewed_anchor, X509RsaSha384ReviewedAnchorLinkErrorKind,
};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const ROOT_HEX: &str = include_str!("fixtures/microsoft_rsa_root_2017.der.hex");
const SHA384_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 12];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const NULL_DER: &[u8] = &[0x05, 0x00];

// Public certificate from Microsoft's documented PKI repository. SHA-256:
// c741f70f4b2a8d88bf2e71c14122ef53ef10eba0cfa5e64cfa20f418853073e0.
fn root_fixture() -> Vec<u8> {
    let hexadecimal = ROOT_HEX.trim().replace('\n', "");
    assert!(hexadecimal.len().is_multiple_of(2));
    hexadecimal
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn decode(input: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(input, Asn1Limits::default()).unwrap()
}

fn role_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> x509_certificate_policy::ValidatedCertificateRolePolicy<'certificate, 'encoded> {
    validate_certificate_role_policy(
        certificate,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]) || oid.equals(&[2, 5, 29, 19]),
    )
    .unwrap()
}

fn rsa_algorithm_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> ValidatedX509RsaAlgorithmPolicy<'certificate, 'encoded> {
    let profiles = [X509CertificateAlgorithmProfile::new(
        SHA384_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        4_096,
        8_192,
        4_096,
        4_096,
    )
    .unwrap()];
    let algorithm_policy = validate_certificate_algorithm_policy(certificate, &profiles).unwrap();
    validate_x509_rsa_algorithm_policy(
        algorithm_policy,
        RsaPublicKeyPolicy::new(4_096, 4_096, &[65_537]).unwrap(),
        Asn1Limits::default(),
    )
    .unwrap()
}

fn strongest_link<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
) -> VerifiedX509RsaSha384IssuerPolicyLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    verify_x509_rsa_sha384_issuer_link_with_policies(
        child,
        issuer,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        role_policy(issuer),
        rsa_algorithm_policy(issuer),
        RsaPublicOperationLimits::new(512).unwrap(),
    )
    .unwrap()
}

#[test]
fn binds_one_authenticated_sha384_link_to_the_same_exact_reviewed_anchor() {
    let encoded = root_fixture();
    let child = decode(&encoded);
    let issuer = decode(&encoded);
    let verified_link = strongest_link(&child, &issuer);
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&issuer).unwrap();

    let bound = bind_x509_rsa_sha384_issuer_link_to_reviewed_anchor(verified_link, reviewed_anchor)
        .unwrap();

    assert!(std::ptr::eq(bound.verified_issuer_link().child(), &child));
    assert!(std::ptr::eq(
        bound.verified_issuer_link().candidate_issuer(),
        &issuer
    ));
    assert!(std::ptr::eq(
        bound.reviewed_anchor().candidate_certificate(),
        &issuer
    ));
    assert_eq!(format!("{bound:?}"), "BoundX509RsaSha384ReviewedAnchorLink");
}

#[test]
fn rejects_reviewed_anchor_evidence_from_a_distinct_byte_equal_object() {
    let encoded = root_fixture();
    let child = decode(&encoded);
    let issuer = decode(&encoded);
    let separately_decoded_issuer = decode(&encoded);
    let verified_link = strongest_link(&child, &issuer);
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&separately_decoded_issuer).unwrap();

    let error = bind_x509_rsa_sha384_issuer_link_to_reviewed_anchor(verified_link, reviewed_anchor)
        .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaSha384ReviewedAnchorLinkErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA SHA-384 reviewed-anchor link error EvidenceMismatch"
    );
    assert!(!error.to_string().contains("Microsoft"));
}
