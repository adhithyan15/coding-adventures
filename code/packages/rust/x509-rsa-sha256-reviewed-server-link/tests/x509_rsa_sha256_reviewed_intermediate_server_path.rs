use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use tls_server_identity::{MatchKind, ReferenceIdentity};
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_issuer_candidate_selection::select_unique_x509_issuer_candidate_by;
use x509_certificate_path_construction::{
    construct_unique_x509_certificate_path_by, ConstructedX509CertificatePath,
};
use x509_certificate_path_length_policy::X509CertificatePathLengthState;
use x509_certificate_policy::{
    classify_certificate_self_issued, validate_certificate_algorithm_policy,
    validate_certificate_role_policy, X509AlgorithmParametersPolicy,
    X509CertificateAlgorithmProfile, X509CertificatePurposePolicy, X509CertificateRole,
};
use x509_intermediate_certificate_policy::validate_x509_intermediate_certificate_policy;
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_sha256_issuer_link::{
    advance_constructed_x509_rsa_sha256_intermediate_path_length,
    advance_x509_rsa_sha256_authenticated_intermediate_path_length,
    bind_x509_rsa_sha256_intermediate_issuer_policy_link,
    verify_constructed_x509_rsa_sha256_intermediate_issuer_policy_link,
    verify_constructed_x509_rsa_sha256_issuer_link,
    verify_selected_x509_rsa_sha256_issuer_link_with_policies,
    VerifiedX509RsaSha256IssuerPolicyLink, X509RsaSha256IssuerLinkPolicy,
};
use x509_rsa_sha256_reviewed_anchor_link::{
    bind_x509_rsa_sha256_authenticated_intermediate_path_to_reviewed_anchor,
    bind_x509_rsa_sha256_constructed_intermediate_path_to_reviewed_anchor,
    bind_x509_rsa_sha256_issuer_link_to_reviewed_anchor,
    BoundX509RsaSha256ConstructedReviewedIntermediatePath,
    BoundX509RsaSha256ReviewedIntermediatePath,
};
use x509_rsa_sha256_reviewed_server_link::{
    bind_x509_rsa_sha256_constructed_reviewed_intermediate_server_path,
    bind_x509_rsa_sha256_reviewed_intermediate_server_path,
    validate_x509_rsa_sha256_constructed_reviewed_intermediate_server_anchor_policy,
    validate_x509_rsa_sha256_reviewed_intermediate_server_anchor_policy,
    validate_x509_rsa_sha256_reviewed_intermediate_server_name_constraints_policy,
    X509RsaSha256ReviewedServerLinkErrorKind,
};
use x509_server_certificate_policy::{
    validate_x509_server_certificate_policy, ValidatedX509ServerCertificatePolicy,
};
use x509_time::{decode_x509_time, X509Time};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const LEAF_HEX: &str = include_str!("fixtures/valid_isrgrootx1_r13_precertificate.hex");
const INTERMEDIATE_HEX: &str = include_str!("fixtures/letsencrypt_r13.hex");
const ROOT_HEX: &str =
    include_str!("../../x509-rsa-sha256-reviewed-anchor-link/tests/fixtures/isrg_root_x1.hex");
const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const SERVER_AUTH_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 3, 1];
const NULL_DER: &[u8] = &[0x05, 0x00];

// Public precertificate retrieved from https://crt.sh/?d=24041010541.
// SHA-256 of its DER form:
// c729b03f3825f8c91f88d788c423a3624ab8ae2b0451f0497419ad14a9b3f023.
//
// Public R13 intermediate retrieved from
// https://letsencrypt.org/certs/2024/r13.der.
// SHA-256:
// d3b128216a843f8ef1321501f5df52a5df52939ee2c19297712cd3de4d419354.
//
// The shared ISRG Root X1 vector records its own official source and digest.
fn fixture(hexadecimal: &str) -> Vec<u8> {
    let hexadecimal = hexadecimal.trim().replace('\n', "");
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

fn algorithm_profiles() -> [X509CertificateAlgorithmProfile<'static>; 1] {
    [X509CertificateAlgorithmProfile::new(
        SHA256_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        8_192,
        2_048,
        4_096,
    )
    .unwrap()]
}

fn link_policy() -> X509RsaSha256IssuerLinkPolicy<'static> {
    X509RsaSha256IssuerLinkPolicy::new(
        Asn1Limits::default(),
        RsaPublicKeyPolicy::new(2_048, 4_096, &[65_537]).unwrap(),
        Asn1Limits::default(),
        RsaPublicOperationLimits::new(512).unwrap(),
    )
}

fn observed_time() -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(b"\x17\x0d260201000000Z").unwrap()).unwrap()
}

fn constructed_intermediate_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> x509_intermediate_certificate_policy::ValidatedX509IntermediateCertificatePolicy<
    'certificate,
    'encoded,
> {
    let role_policy = validate_certificate_role_policy(
        certificate,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]) || oid.equals(&[2, 5, 29, 19]),
    )
    .unwrap();
    let profiles = algorithm_profiles();
    let algorithm_policy = validate_certificate_algorithm_policy(certificate, &profiles).unwrap();
    let self_issued = classify_certificate_self_issued(certificate, |issuer, subject| {
        issuer.encoded() == subject.encoded()
    });

    validate_x509_intermediate_certificate_policy(
        role_policy,
        algorithm_policy,
        self_issued,
        &observed_time(),
    )
    .unwrap()
}

fn constructed_reviewed_path<'path, 'target, 'target_encoded, 'candidates, 'candidate_encoded>(
    path: &'path ConstructedX509CertificatePath<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
) -> BoundX509RsaSha256ConstructedReviewedIntermediatePath<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    let intermediate = path.issuer_certificate(0).unwrap();
    let root = path.issuer_certificate(1).unwrap();
    let authenticated_intermediate =
        verify_constructed_x509_rsa_sha256_intermediate_issuer_policy_link(
            path.adjacent_link(0).unwrap(),
            constructed_intermediate_policy(intermediate),
            link_policy(),
        )
        .unwrap();
    let authenticated_path = advance_constructed_x509_rsa_sha256_intermediate_path_length(
        X509CertificatePathLengthState::new(1),
        authenticated_intermediate,
    )
    .unwrap();
    let authenticated_terminal = verify_constructed_x509_rsa_sha256_issuer_link(
        path.adjacent_link(1).unwrap(),
        link_policy(),
    )
    .unwrap();

    bind_x509_rsa_sha256_constructed_intermediate_path_to_reviewed_anchor(
        authenticated_path,
        authenticated_terminal,
        select_reviewed_webpki_root_anchor(root).unwrap(),
    )
    .unwrap()
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

fn server_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> ValidatedX509ServerCertificatePolicy<'certificate, 'encoded> {
    let role_policy = validate_certificate_role_policy(
        certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| {
            oid.equals(&[2, 5, 29, 15])
                || oid.equals(&[2, 5, 29, 19])
                || oid.equals(&[1, 3, 6, 1, 4, 1, 11129, 2, 4, 3])
        },
    )
    .unwrap();
    let profiles = algorithm_profiles();
    let algorithm_policy = validate_certificate_algorithm_policy(certificate, &profiles).unwrap();
    let reference = ReferenceIdentity::parse("valid-isrgrootx1.letsencrypt.org").unwrap();
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

fn reviewed_path<
    'child_certificate,
    'child_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
    'root_certificate,
    'root_encoded,
>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    intermediate: &'intermediate_certificate X509Certificate<'intermediate_encoded>,
    root: &'root_certificate X509Certificate<'root_encoded>,
) -> BoundX509RsaSha256ReviewedIntermediatePath<
    'child_certificate,
    'child_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
    'root_certificate,
    'root_encoded,
> {
    let verified_child_link = strongest_link(child, intermediate);
    let intermediate_role_policy = validate_certificate_role_policy(
        intermediate,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]) || oid.equals(&[2, 5, 29, 19]),
    )
    .unwrap();
    let profiles = algorithm_profiles();
    let intermediate_algorithm_policy =
        validate_certificate_algorithm_policy(intermediate, &profiles).unwrap();
    let self_issued = classify_certificate_self_issued(intermediate, |issuer, subject| {
        issuer.encoded() == subject.encoded()
    });
    let intermediate_policy = validate_x509_intermediate_certificate_policy(
        intermediate_role_policy,
        intermediate_algorithm_policy,
        self_issued,
        &observed_time(),
    )
    .unwrap();
    let authenticated_intermediate = bind_x509_rsa_sha256_intermediate_issuer_policy_link(
        verified_child_link,
        intermediate_policy,
    )
    .unwrap();
    let authenticated_path = advance_x509_rsa_sha256_authenticated_intermediate_path_length(
        X509CertificatePathLengthState::new(1),
        authenticated_intermediate,
    )
    .unwrap();
    let reviewed_parent_link = bind_x509_rsa_sha256_issuer_link_to_reviewed_anchor(
        strongest_link(intermediate, root),
        select_reviewed_webpki_root_anchor(root).unwrap(),
    )
    .unwrap();

    bind_x509_rsa_sha256_authenticated_intermediate_path_to_reviewed_anchor(
        authenticated_path,
        reviewed_parent_link,
    )
    .unwrap()
}

#[test]
fn binds_real_server_policy_to_the_same_exact_reviewed_two_link_path() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let intermediate = decode(&intermediate_encoded);
    let root = decode(&root_encoded);
    let policy = server_policy(&leaf);
    let path = reviewed_path(&leaf, &intermediate, &root);

    let bound = bind_x509_rsa_sha256_reviewed_intermediate_server_path(policy, path).unwrap();

    assert!(std::ptr::eq(bound.server_policy().certificate(), &leaf));
    assert_eq!(
        bound.server_policy().identity().match_kind(),
        MatchKind::ExactDns
    );
    let retained_path = bound.reviewed_intermediate_path();
    assert!(std::ptr::eq(
        retained_path
            .authenticated_path()
            .authenticated_intermediate()
            .issuer_policy_link()
            .role_policy_link()
            .verified_link()
            .child(),
        &leaf
    ));
    assert!(std::ptr::eq(
        retained_path
            .authenticated_path()
            .authenticated_intermediate()
            .intermediate_policy()
            .certificate(),
        &intermediate
    ));
    assert_eq!(
        retained_path
            .authenticated_path()
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    assert!(std::ptr::eq(
        retained_path
            .reviewed_parent_link()
            .reviewed_anchor()
            .candidate_certificate(),
        &root
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509RsaSha256ReviewedIntermediateServerPath"
    );
}

#[test]
fn rejects_byte_equal_distinct_server_target_from_real_reviewed_path() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let path_leaf = decode(&leaf_encoded);
    let detached_policy_leaf = decode(&leaf_encoded);
    let intermediate = decode(&intermediate_encoded);
    let root = decode(&root_encoded);
    let policy = server_policy(&detached_policy_leaf);
    let path = reviewed_path(&path_leaf, &intermediate, &root);

    let error = bind_x509_rsa_sha256_reviewed_intermediate_server_path(policy, path)
        .expect_err("byte-equal targets decoded into distinct objects must not bind");

    assert_eq!(
        error.kind(),
        X509RsaSha256ReviewedServerLinkErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA SHA-256 reviewed server link error EvidenceMismatch"
    );
    assert!(!error.to_string().contains("valid-isrgrootx1"));
}

#[test]
fn binds_server_policy_to_the_exact_constructed_authenticated_reviewed_path() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = construct_unique_x509_certificate_path_by(
        &leaf,
        &candidates,
        2,
        3,
        |certificate| std::ptr::eq(certificate, &candidates[1]),
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
    )
    .unwrap();
    let policy = server_policy(&leaf);
    let reviewed_path = constructed_reviewed_path(&path);

    let bound =
        bind_x509_rsa_sha256_constructed_reviewed_intermediate_server_path(policy, reviewed_path)
            .unwrap();

    assert!(std::ptr::eq(bound.server_policy().certificate(), &leaf));
    assert_eq!(
        bound.server_policy().identity().match_kind(),
        MatchKind::ExactDns
    );
    let retained_path = bound.reviewed_intermediate_path();
    assert!(std::ptr::eq(
        retained_path
            .authenticated_path()
            .authenticated_intermediate()
            .verified_link()
            .adjacent_link()
            .constructed_path(),
        &path
    ));
    assert_eq!(
        retained_path
            .authenticated_path()
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    assert!(std::ptr::eq(
        retained_path.reviewed_anchor().candidate_certificate(),
        &candidates[1]
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509RsaSha256ConstructedReviewedIntermediateServerPath"
    );
}

#[test]
fn rejects_byte_equal_distinct_policy_target_for_a_constructed_reviewed_path() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let path_leaf = decode(&leaf_encoded);
    let detached_policy_leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = construct_unique_x509_certificate_path_by(
        &path_leaf,
        &candidates,
        2,
        3,
        |certificate| std::ptr::eq(certificate, &candidates[1]),
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
    )
    .unwrap();
    let policy = server_policy(&detached_policy_leaf);
    let reviewed_path = constructed_reviewed_path(&path);

    let error =
        bind_x509_rsa_sha256_constructed_reviewed_intermediate_server_path(policy, reviewed_path)
            .expect_err("byte-equal targets decoded into distinct objects must not bind");

    assert_eq!(
        error.kind(),
        X509RsaSha256ReviewedServerLinkErrorKind::EvidenceMismatch
    );
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("valid-isrgrootx1"));
    assert!(!diagnostic.contains("Let's Encrypt"));
}

#[test]
fn applies_selected_anchor_policy_to_the_exact_constructed_reviewed_path() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = construct_unique_x509_certificate_path_by(
        &leaf,
        &candidates,
        2,
        3,
        |certificate| std::ptr::eq(certificate, &candidates[1]),
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
    )
    .unwrap();
    let policy = server_policy(&leaf);
    let reviewed_path = constructed_reviewed_path(&path);
    let bound =
        bind_x509_rsa_sha256_constructed_reviewed_intermediate_server_path(policy, reviewed_path)
            .unwrap();

    let validated =
        validate_x509_rsa_sha256_constructed_reviewed_intermediate_server_anchor_policy(
            bound,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

    assert!(validated.constrained_dns_names().is_none());
    let retained = validated.reviewed_intermediate_server_path();
    assert!(std::ptr::eq(retained.server_policy().certificate(), &leaf));
    assert!(std::ptr::eq(
        retained
            .reviewed_intermediate_path()
            .authenticated_path()
            .authenticated_intermediate()
            .verified_link()
            .adjacent_link()
            .constructed_path(),
        &path
    ));
    assert_eq!(
        retained
            .reviewed_intermediate_path()
            .authenticated_path()
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    assert!(std::ptr::eq(
        retained
            .reviewed_intermediate_path()
            .reviewed_anchor()
            .candidate_certificate(),
        &candidates[1]
    ));
    assert_eq!(
        format!("{validated:?}"),
        "ValidatedX509RsaSha256ConstructedReviewedIntermediateServerAnchorPolicy"
    );
}

#[test]
fn applies_selected_anchor_policy_to_the_complete_real_two_link_server_path() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let intermediate = decode(&intermediate_encoded);
    let root = decode(&root_encoded);
    let policy = server_policy(&leaf);
    let path = reviewed_path(&leaf, &intermediate, &root);
    let bound = bind_x509_rsa_sha256_reviewed_intermediate_server_path(policy, path).unwrap();

    let validated = validate_x509_rsa_sha256_reviewed_intermediate_server_anchor_policy(
        bound,
        Asn1Limits::default(),
        Asn1Limits::default(),
    )
    .unwrap();

    assert_eq!(validated.constrained_dns_names(), None);
    let retained = validated.reviewed_intermediate_server_path();
    assert!(std::ptr::eq(retained.server_policy().certificate(), &leaf));
    assert!(std::ptr::eq(
        retained
            .reviewed_intermediate_path()
            .reviewed_parent_link()
            .reviewed_anchor()
            .candidate_certificate(),
        &root
    ));
    assert_eq!(
        format!("{validated:?}"),
        "ValidatedX509RsaSha256ReviewedIntermediateServerAnchorPolicy"
    );
}

#[test]
fn retains_the_complete_real_path_when_the_intermediate_has_no_name_constraints() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let intermediate = decode(&intermediate_encoded);
    let root = decode(&root_encoded);
    let policy = server_policy(&leaf);
    let path = reviewed_path(&leaf, &intermediate, &root);
    let bound = bind_x509_rsa_sha256_reviewed_intermediate_server_path(policy, path).unwrap();
    let anchor_policy = validate_x509_rsa_sha256_reviewed_intermediate_server_anchor_policy(
        bound,
        Asn1Limits::default(),
        Asn1Limits::default(),
    )
    .unwrap();

    let validated = validate_x509_rsa_sha256_reviewed_intermediate_server_name_constraints_policy(
        anchor_policy,
        Asn1Limits::default(),
        Asn1Limits::default(),
    )
    .unwrap();

    assert_eq!(validated.constrained_dns_names(), None);
    let retained = validated
        .anchor_policy()
        .reviewed_intermediate_server_path();
    assert!(std::ptr::eq(retained.server_policy().certificate(), &leaf));
    assert!(std::ptr::eq(
        retained
            .reviewed_intermediate_path()
            .authenticated_path()
            .authenticated_intermediate()
            .intermediate_policy()
            .certificate(),
        &intermediate
    ));
    assert!(std::ptr::eq(
        retained
            .reviewed_intermediate_path()
            .reviewed_parent_link()
            .reviewed_anchor()
            .candidate_certificate(),
        &root
    ));
    assert_eq!(
        format!("{validated:?}"),
        "ValidatedX509RsaSha256ReviewedIntermediateServerNameConstraintsPolicy"
    );
}
