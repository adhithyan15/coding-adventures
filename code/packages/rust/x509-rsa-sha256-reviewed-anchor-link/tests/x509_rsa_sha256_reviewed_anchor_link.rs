use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_issuer_candidate_selection::select_unique_x509_issuer_candidate_by;
use x509_certificate_path_construction::construct_unique_x509_certificate_path_by;
use x509_certificate_path_length_policy::X509CertificatePathLengthState;
use x509_certificate_policy::{
    classify_certificate_self_issued, validate_certificate_algorithm_policy,
    validate_certificate_role_policy, X509AlgorithmParametersPolicy,
    X509CertificateAlgorithmProfile, X509CertificateRole,
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
    X509RsaSha256ConstructedReviewedIntermediatePathErrorKind,
    X509RsaSha256ReviewedAnchorLinkErrorKind, X509RsaSha256ReviewedIntermediatePathErrorKind,
};
use x509_time::{decode_x509_time, X509Time};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const ROOT_HEX: &str = include_str!("fixtures/isrg_root_x1.hex");
const LEAF_HEX: &str = include_str!(
    "../../x509-rsa-sha256-reviewed-server-link/tests/fixtures/valid_isrgrootx1_r13_precertificate.hex"
);
const INTERMEDIATE_HEX: &str =
    include_str!("../../x509-rsa-sha256-reviewed-server-link/tests/fixtures/letsencrypt_r13.hex");
const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const NULL_DER: &[u8] = &[0x05, 0x00];

// Public test vector from https://letsencrypt.org/certs/isrgrootx1.der.
// SHA-256: 96bcec06264976f37460779acf28c5a7cfe8a3c0aae11a8ffcee05c0bddf08c6.
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

fn root_fixture() -> Vec<u8> {
    fixture(ROOT_HEX)
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

fn observed_time() -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(b"\x17\x0d260101000000Z").unwrap()).unwrap()
}

fn authenticated_intermediate_path<
    'child_certificate,
    'child_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    intermediate: &'intermediate_certificate X509Certificate<'intermediate_encoded>,
) -> x509_rsa_sha256_issuer_link::ValidatedX509RsaSha256AuthenticatedIntermediatePathLength<
    'child_certificate,
    'child_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
> {
    let verified_link = strongest_link(child, intermediate);
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
    let authenticated =
        bind_x509_rsa_sha256_intermediate_issuer_policy_link(verified_link, intermediate_policy)
            .unwrap();

    advance_x509_rsa_sha256_authenticated_intermediate_path_length(
        X509CertificatePathLengthState::new(1),
        authenticated,
    )
    .unwrap()
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

#[test]
fn binds_two_authenticated_links_through_the_same_exact_intermediate() {
    let encoded = root_fixture();
    let child = decode(&encoded);
    let intermediate = decode(&encoded);
    let root = decode(&encoded);
    let authenticated_path = authenticated_intermediate_path(&child, &intermediate);
    let reviewed_parent_link = bind_x509_rsa_sha256_issuer_link_to_reviewed_anchor(
        strongest_link(&intermediate, &root),
        select_reviewed_webpki_root_anchor(&root).unwrap(),
    )
    .unwrap();

    let bound = bind_x509_rsa_sha256_authenticated_intermediate_path_to_reviewed_anchor(
        authenticated_path,
        reviewed_parent_link,
    )
    .unwrap();

    assert!(std::ptr::eq(
        bound
            .authenticated_path()
            .authenticated_intermediate()
            .issuer_policy_link()
            .role_policy_link()
            .verified_link()
            .child(),
        &child
    ));
    assert!(std::ptr::eq(
        bound
            .authenticated_path()
            .authenticated_intermediate()
            .intermediate_policy()
            .certificate(),
        &intermediate
    ));
    assert_eq!(
        bound
            .authenticated_path()
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        1
    );
    assert!(std::ptr::eq(
        bound
            .reviewed_parent_link()
            .verified_issuer_link()
            .role_policy_link()
            .verified_link()
            .child(),
        &intermediate
    ));
    assert!(std::ptr::eq(
        bound
            .reviewed_parent_link()
            .reviewed_anchor()
            .candidate_certificate(),
        &root
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509RsaSha256ReviewedIntermediatePath"
    );
}

#[test]
fn rejects_two_links_through_byte_equal_distinct_intermediates() {
    let encoded = root_fixture();
    let child = decode(&encoded);
    let intermediate = decode(&encoded);
    let detached_intermediate = decode(&encoded);
    let root = decode(&encoded);
    let authenticated_path = authenticated_intermediate_path(&child, &intermediate);
    let reviewed_parent_link = bind_x509_rsa_sha256_issuer_link_to_reviewed_anchor(
        strongest_link(&detached_intermediate, &root),
        select_reviewed_webpki_root_anchor(&root).unwrap(),
    )
    .unwrap();

    let error = bind_x509_rsa_sha256_authenticated_intermediate_path_to_reviewed_anchor(
        authenticated_path,
        reviewed_parent_link,
    )
    .expect_err("byte-equal links through distinct intermediate objects must not bind");

    assert_eq!(
        error.kind(),
        X509RsaSha256ReviewedIntermediatePathErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA SHA-256 reviewed intermediate path error EvidenceMismatch"
    );
    assert!(!error.to_string().contains("ISRG"));
}

#[test]
fn binds_the_exact_constructed_two_link_path_to_its_reviewed_terminal() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = root_fixture();
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
    let authenticated_intermediate =
        verify_constructed_x509_rsa_sha256_intermediate_issuer_policy_link(
            path.adjacent_link(0).unwrap(),
            constructed_intermediate_policy(&candidates[0]),
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
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&candidates[1]).unwrap();

    let bound = bind_x509_rsa_sha256_constructed_intermediate_path_to_reviewed_anchor(
        authenticated_path,
        authenticated_terminal,
        reviewed_anchor,
    )
    .unwrap();

    let retained_intermediate = bound.authenticated_path();
    assert_eq!(
        retained_intermediate
            .authenticated_intermediate()
            .verified_link()
            .adjacent_link()
            .issuer_depth(),
        0
    );
    assert!(std::ptr::eq(
        retained_intermediate
            .authenticated_intermediate()
            .verified_link()
            .adjacent_link()
            .constructed_path(),
        &path
    ));
    assert_eq!(
        retained_intermediate
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    assert_eq!(
        bound
            .authenticated_terminal_link()
            .adjacent_link()
            .issuer_depth(),
        1
    );
    assert!(std::ptr::eq(
        bound
            .authenticated_terminal_link()
            .adjacent_link()
            .constructed_path(),
        &path
    ));
    assert!(std::ptr::eq(
        bound.reviewed_anchor().candidate_certificate(),
        &candidates[1]
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509RsaSha256ConstructedReviewedIntermediatePath"
    );
}

#[test]
fn rejects_authenticated_links_from_distinct_construction_objects() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = root_fixture();
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let construct = || {
        construct_unique_x509_certificate_path_by(
            &leaf,
            &candidates,
            2,
            3,
            |certificate| std::ptr::eq(certificate, &candidates[1]),
            |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        )
        .unwrap()
    };
    let intermediate_path = construct();
    let terminal_path = construct();
    let authenticated_intermediate =
        verify_constructed_x509_rsa_sha256_intermediate_issuer_policy_link(
            intermediate_path.adjacent_link(0).unwrap(),
            constructed_intermediate_policy(&candidates[0]),
            link_policy(),
        )
        .unwrap();
    let authenticated_path = advance_constructed_x509_rsa_sha256_intermediate_path_length(
        X509CertificatePathLengthState::new(1),
        authenticated_intermediate,
    )
    .unwrap();
    let authenticated_terminal = verify_constructed_x509_rsa_sha256_issuer_link(
        terminal_path.adjacent_link(1).unwrap(),
        link_policy(),
    )
    .unwrap();

    let error = bind_x509_rsa_sha256_constructed_intermediate_path_to_reviewed_anchor(
        authenticated_path,
        authenticated_terminal,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .expect_err("links from separately constructed paths must not bind");

    assert_eq!(
        error.kind(),
        X509RsaSha256ConstructedReviewedIntermediatePathErrorKind::EvidenceMismatch
    );
    assert!(!error.to_string().contains("Let's Encrypt"));
}

#[test]
fn rejects_a_nonterminal_second_projection_as_an_invalid_path_shape() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = root_fixture();
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
    let authenticated_intermediate =
        verify_constructed_x509_rsa_sha256_intermediate_issuer_policy_link(
            path.adjacent_link(0).unwrap(),
            constructed_intermediate_policy(&candidates[0]),
            link_policy(),
        )
        .unwrap();
    let authenticated_path = advance_constructed_x509_rsa_sha256_intermediate_path_length(
        X509CertificatePathLengthState::new(1),
        authenticated_intermediate,
    )
    .unwrap();
    let repeated_first_link = verify_constructed_x509_rsa_sha256_issuer_link(
        path.adjacent_link(0).unwrap(),
        link_policy(),
    )
    .unwrap();

    let error = bind_x509_rsa_sha256_constructed_intermediate_path_to_reviewed_anchor(
        authenticated_path,
        repeated_first_link,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .expect_err("the second proof must authenticate the terminal adjacency");

    assert_eq!(
        error.kind(),
        X509RsaSha256ConstructedReviewedIntermediatePathErrorKind::InvalidPathShape
    );
    assert!(!error.to_string().contains("Let's Encrypt"));
}

#[test]
fn rejects_reviewed_root_evidence_for_a_byte_equal_redecode() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = root_fixture();
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let detached_root = decode(&root_encoded);
    let path = construct_unique_x509_certificate_path_by(
        &leaf,
        &candidates,
        2,
        3,
        |certificate| std::ptr::eq(certificate, &candidates[1]),
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
    )
    .unwrap();
    let authenticated_intermediate =
        verify_constructed_x509_rsa_sha256_intermediate_issuer_policy_link(
            path.adjacent_link(0).unwrap(),
            constructed_intermediate_policy(&candidates[0]),
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

    let error = bind_x509_rsa_sha256_constructed_intermediate_path_to_reviewed_anchor(
        authenticated_path,
        authenticated_terminal,
        select_reviewed_webpki_root_anchor(&detached_root).unwrap(),
    )
    .expect_err("reviewed membership for a re-decoded root must not bind");

    assert_eq!(
        error.kind(),
        X509RsaSha256ConstructedReviewedIntermediatePathErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA SHA-256 constructed reviewed intermediate path error EvidenceMismatch"
    );
    assert!(!error.to_string().contains("ISRG"));
}
