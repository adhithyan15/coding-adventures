use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_issuer_candidate_selection::select_unique_x509_issuer_candidate_by;
use x509_certificate_policy::{
    validate_certificate_algorithm_policy, validate_certificate_role_policy,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile, X509CertificateRole,
};
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_sha256_issuer_link::{
    verify_selected_x509_rsa_sha256_issuer_link_with_policies,
    VerifiedX509RsaSha256IssuerPolicyLink, X509RsaSha256IssuerLinkPolicy,
};
use x509_rsa_sha256_reviewed_anchor_link::{
    bind_x509_rsa_sha256_issuer_link_to_reviewed_anchor, X509RsaSha256ReviewedAnchorLinkErrorKind,
};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const ROOT_HEX: &str = include_str!("fixtures/isrg_root_x1.hex");
const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const NULL_DER: &[u8] = &[0x05, 0x00];

// Public test vector from https://letsencrypt.org/certs/isrgrootx1.der.
// SHA-256: 96bcec06264976f37460779acf28c5a7cfe8a3c0aae11a8ffcee05c0bddf08c6.
fn root_fixture() -> Vec<u8> {
    let hexadecimal = ROOT_HEX.trim();
    assert!(hexadecimal.len().is_multiple_of(2));
    hexadecimal
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}

fn decode(input: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(input, Asn1Limits::default()).unwrap()
}

fn link_policy() -> X509RsaSha256IssuerLinkPolicy<'static> {
    X509RsaSha256IssuerLinkPolicy::new(
        Asn1Limits::default(),
        RsaPublicKeyPolicy::new(4_096, 4_096, &[65_537]).unwrap(),
        Asn1Limits::default(),
        RsaPublicOperationLimits::new(512).unwrap(),
    )
}

fn algorithm_profiles() -> [X509CertificateAlgorithmProfile<'static>; 1] {
    [X509CertificateAlgorithmProfile::new(
        SHA256_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        4_096,
        8_192,
        4_096,
        4_096,
    )
    .unwrap()]
}

fn strongest_link<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
) -> VerifiedX509RsaSha256IssuerPolicyLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    let candidates = std::slice::from_ref(issuer);
    let selected = select_unique_x509_issuer_candidate_by(
        child,
        candidates,
        1,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
    )
    .unwrap();
    let issuer_role_policy = validate_certificate_role_policy(
        issuer,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]) || oid.equals(&[2, 5, 29, 19]),
    )
    .unwrap();
    let profiles = algorithm_profiles();
    let issuer_algorithm_policy = validate_certificate_algorithm_policy(issuer, &profiles).unwrap();

    verify_selected_x509_rsa_sha256_issuer_link_with_policies(
        selected,
        issuer_role_policy,
        issuer_algorithm_policy,
        link_policy(),
    )
    .unwrap()
}

#[test]
fn binds_one_authenticated_link_to_the_same_exact_reviewed_anchor_certificate() {
    let encoded = root_fixture();
    let child = decode(&encoded);
    let issuer = decode(&encoded);
    let verified_link = strongest_link(&child, &issuer);
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&issuer).unwrap();

    let bound = bind_x509_rsa_sha256_issuer_link_to_reviewed_anchor(verified_link, reviewed_anchor)
        .unwrap();

    assert!(std::ptr::eq(
        bound
            .verified_issuer_link()
            .role_policy_link()
            .verified_link()
            .child(),
        &child
    ));
    assert!(std::ptr::eq(
        bound
            .verified_issuer_link()
            .role_policy_link()
            .verified_link()
            .candidate_issuer(),
        &issuer
    ));
    assert!(std::ptr::eq(
        bound.reviewed_anchor().candidate_certificate(),
        &issuer
    ));
    assert_eq!(format!("{bound:?}"), "BoundX509RsaSha256ReviewedAnchorLink");
}

#[test]
fn rejects_reviewed_anchor_evidence_for_a_distinct_certificate_object() {
    let encoded = root_fixture();
    let child = decode(&encoded);
    let issuer = decode(&encoded);
    let other_issuer = decode(&encoded);
    let verified_link = strongest_link(&child, &issuer);
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&other_issuer).unwrap();

    let error = bind_x509_rsa_sha256_issuer_link_to_reviewed_anchor(verified_link, reviewed_anchor)
        .expect_err("equal bytes from distinct certificate objects must not bind");

    assert_eq!(
        error.kind(),
        X509RsaSha256ReviewedAnchorLinkErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA SHA-256 reviewed-anchor link error EvidenceMismatch"
    );
    assert!(!error.to_string().contains("ISRG"));
}
