use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_policy::{
    classify_certificate_self_issued, validate_certificate_algorithm_policy,
    validate_certificate_role_policy, X509AlgorithmParametersPolicy,
    X509CertificateAlgorithmProfile, X509CertificateRole,
};
use x509_intermediate_certificate_policy::{
    validate_x509_intermediate_certificate_policy, ValidatedX509IntermediateCertificatePolicy,
};
use x509_rsa_algorithm_policy::{
    validate_x509_rsa_algorithm_policy, ValidatedX509RsaAlgorithmPolicy,
};
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_sha512_issuer_link::{
    bind_x509_rsa_sha512_intermediate_issuer_policy_link,
    verify_x509_rsa_sha512_issuer_link_with_policies,
    VerifiedX509RsaSha512IntermediateIssuerPolicyLink, VerifiedX509RsaSha512IssuerPolicyLink,
};
use x509_rsa_sha512_reviewed_anchor_link::{
    bind_x509_rsa_sha512_intermediate_path_to_reviewed_anchor,
    bind_x509_rsa_sha512_issuer_link_to_reviewed_anchor, X509RsaSha512ReviewedAnchorLinkErrorKind,
    X509RsaSha512ReviewedIntermediatePathErrorKind,
};
use x509_time::{decode_x509_time, X509Time};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const ROOT_HEX: &str = include_str!("fixtures/d_trust_br_root_ca_2_2023.der.hex");
const SHA512_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 13];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const NULL_DER: &[u8] = &[0x05, 0x00];

// Public certificate from the reviewed webpki-roots 1.0.9 snapshot, upstream
// tag v1.0.9 at commit 0a553dbc8b3f18ea05c4f881cffa3f2d005d0d30.
// SHA-256: 0552e6f83fdf65e8fa9670e666df28a4e21340b510cbe52566f97c4fb94b2bd1.
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
        SHA512_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        8_192,
        2_048,
        8_192,
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

fn observed_time() -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(b"\x17\x0d261101000000Z").unwrap()).unwrap()
}

fn intermediate_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> ValidatedX509IntermediateCertificatePolicy<'certificate, 'encoded> {
    let profiles = [X509CertificateAlgorithmProfile::new(
        SHA512_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        8_192,
        2_048,
        8_192,
    )
    .unwrap()];
    let algorithm_policy = validate_certificate_algorithm_policy(certificate, &profiles).unwrap();
    let self_issued = classify_certificate_self_issued(certificate, |issuer, subject| {
        issuer.encoded() == subject.encoded()
    });

    validate_x509_intermediate_certificate_policy(
        role_policy(certificate),
        algorithm_policy,
        self_issued,
        &observed_time(),
    )
    .unwrap()
}

fn strongest_link<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
) -> VerifiedX509RsaSha512IssuerPolicyLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    verify_x509_rsa_sha512_issuer_link_with_policies(
        child,
        issuer,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        role_policy(issuer),
        rsa_algorithm_policy(issuer),
        RsaPublicOperationLimits::new(512).unwrap(),
    )
    .unwrap()
}

fn strongest_intermediate_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
) -> VerifiedX509RsaSha512IntermediateIssuerPolicyLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    bind_x509_rsa_sha512_intermediate_issuer_policy_link(
        strongest_link(child, issuer),
        intermediate_policy(issuer),
    )
    .unwrap()
}

#[test]
fn binds_one_authenticated_sha512_link_to_the_same_exact_reviewed_anchor() {
    let encoded = root_fixture();
    let child = decode(&encoded);
    let issuer = decode(&encoded);
    let verified_link = strongest_link(&child, &issuer);
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&issuer).unwrap();

    let bound = bind_x509_rsa_sha512_issuer_link_to_reviewed_anchor(verified_link, reviewed_anchor)
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
    assert_eq!(format!("{bound:?}"), "BoundX509RsaSha512ReviewedAnchorLink");
}

#[test]
fn rejects_reviewed_anchor_evidence_from_a_distinct_byte_equal_object() {
    let encoded = root_fixture();
    let child = decode(&encoded);
    let issuer = decode(&encoded);
    let separately_decoded_issuer = decode(&encoded);
    let verified_link = strongest_link(&child, &issuer);
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&separately_decoded_issuer).unwrap();

    let error = bind_x509_rsa_sha512_issuer_link_to_reviewed_anchor(verified_link, reviewed_anchor)
        .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaSha512ReviewedAnchorLinkErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA SHA-512 reviewed-anchor link error EvidenceMismatch"
    );
    assert!(!error.to_string().contains("D-TRUST"));
}

#[test]
fn binds_two_authenticated_sha512_links_through_the_exact_intermediate() {
    let encoded = root_fixture();
    let child = decode(&encoded);
    let intermediate = decode(&encoded);
    let root = decode(&encoded);
    let authenticated_child_link = strongest_intermediate_link(&child, &intermediate);
    let authenticated_parent_link = strongest_link(&intermediate, &root);
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&root).unwrap();
    let reviewed_parent_link = bind_x509_rsa_sha512_issuer_link_to_reviewed_anchor(
        authenticated_parent_link,
        reviewed_anchor,
    )
    .unwrap();

    let reviewed_path = bind_x509_rsa_sha512_intermediate_path_to_reviewed_anchor(
        authenticated_child_link,
        reviewed_parent_link,
    )
    .unwrap();

    assert!(std::ptr::eq(
        reviewed_path
            .authenticated_child_link()
            .issuer_policy_link()
            .child(),
        &child
    ));
    assert!(std::ptr::eq(
        reviewed_path
            .authenticated_child_link()
            .issuer_policy_link()
            .candidate_issuer(),
        &intermediate
    ));
    assert!(std::ptr::eq(
        reviewed_path
            .authenticated_child_link()
            .intermediate_policy()
            .certificate(),
        &intermediate
    ));
    assert!(std::ptr::eq(
        reviewed_path
            .reviewed_parent_link()
            .verified_issuer_link()
            .child(),
        &intermediate
    ));
    assert!(std::ptr::eq(
        reviewed_path
            .reviewed_parent_link()
            .reviewed_anchor()
            .candidate_certificate(),
        &root
    ));
    assert_eq!(
        format!("{reviewed_path:?}"),
        "BoundX509RsaSha512ReviewedIntermediatePath"
    );
}

#[test]
fn rejects_two_links_with_byte_equal_distinct_intermediates() {
    let encoded = root_fixture();
    let child = decode(&encoded);
    let first_intermediate = decode(&encoded);
    let second_intermediate = decode(&encoded);
    let root = decode(&encoded);
    let authenticated_child_link = strongest_intermediate_link(&child, &first_intermediate);
    let authenticated_parent_link = strongest_link(&second_intermediate, &root);
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&root).unwrap();
    let reviewed_parent_link = bind_x509_rsa_sha512_issuer_link_to_reviewed_anchor(
        authenticated_parent_link,
        reviewed_anchor,
    )
    .unwrap();

    let error = bind_x509_rsa_sha512_intermediate_path_to_reviewed_anchor(
        authenticated_child_link,
        reviewed_parent_link,
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaSha512ReviewedIntermediatePathErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA SHA-512 reviewed intermediate-path error EvidenceMismatch"
    );
    assert!(!error.to_string().contains("D-TRUST"));
}
