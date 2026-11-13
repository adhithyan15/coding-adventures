use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use tls_server_identity::ReferenceIdentity;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_path_construction::{
    construct_unique_x509_certificate_path_by,
    construct_unique_x509_certificate_path_with_name_policy, ConstructedX509CertificatePath,
};
use x509_certificate_path_length_policy::{
    X509CertificatePathLengthErrorKind, X509CertificatePathLengthState,
};
use x509_certificate_path_reviewed_anchor_binding::{
    bind_constructed_x509_certificate_path_to_reviewed_anchor,
    bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor,
    bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy,
    bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy,
    initialize_name_policy_reviewed_constructed_x509_certificate_path_length,
    initialize_reviewed_constructed_x509_certificate_path_length,
};
use x509_certificate_policy::{
    classify_certificate_self_issued, validate_certificate_algorithm_policy,
    validate_certificate_role_policy, ValidatedCertificateAlgorithmPolicy,
    ValidatedCertificateRolePolicy, X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile,
    X509CertificatePurposePolicy, X509CertificateRole,
};
use x509_intermediate_certificate_policy::{
    validate_x509_intermediate_certificate_policy, ValidatedX509IntermediateCertificatePolicy,
};
use x509_name_value_policy::{
    X509NameValueComparisonRule, X509NameValuePolicy, X509NameValuePolicyEntry,
};
use x509_rsa_algorithm_policy::{
    validate_x509_rsa_algorithm_policy, ValidatedX509RsaAlgorithmPolicy,
};
use x509_rsa_constructed_path_signature_routing::{
    accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths,
    advance_initialized_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_length,
    advance_initialized_reviewed_x509_rsa_constructed_path_intermediate_path_length,
    advance_next_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_length,
    advance_next_reviewed_x509_rsa_constructed_path_intermediate_path_length,
    advance_x509_rsa_constructed_path_intermediate_path_length,
    bind_accumulated_reviewed_x509_rsa_constructed_path_target_signature,
    bind_x509_rsa_constructed_intermediate_path_to_reviewed_anchor,
    bind_x509_rsa_constructed_path_intermediate_policy,
    bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor,
    bind_x509_rsa_constructed_reviewed_intermediate_server_path,
    validate_accumulated_reviewed_x509_rsa_constructed_path_subordinate_anchor_name_constraints_policy,
    validate_accumulated_reviewed_x509_rsa_constructed_path_subordinate_intermediate_name_constraints_policy,
    validate_accumulated_reviewed_x509_rsa_constructed_path_target_anchor_policy,
    validate_accumulated_reviewed_x509_rsa_constructed_path_target_intermediate_name_constraints_policy,
    validate_x509_rsa_constructed_reviewed_intermediate_server_anchor_policy,
    validate_x509_rsa_constructed_reviewed_intermediate_server_name_constraints_policy,
    verify_x509_rsa_constructed_path_policy_signature,
    BoundX509RsaConstructedReviewedIntermediatePath,
    X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind,
    X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind,
    X509RsaConstructedPathIntermediatePolicyErrorKind,
    X509RsaConstructedPathNextIntermediatePathLengthErrorKind,
    X509RsaConstructedPathReviewedAnchorErrorKind,
    X509RsaConstructedPathReviewedIntermediatePathErrorKind,
    X509RsaConstructedPathReviewedServerPolicyErrorKind,
    X509RsaConstructedPathSignatureRoutingErrorKind,
    X509RsaConstructedPathTargetSignatureClosureErrorKind,
};
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_signature_routing::{
    VerifiedX509RsaPolicySignature, X509RsaSignatureRoute, X509RsaSignatureRoutingErrorKind,
    X509RsaSignatureVerifier,
};
use x509_server_certificate_policy::{
    validate_x509_server_certificate_policy, ValidatedX509ServerCertificatePolicy,
};
use x509_time::{decode_x509_time, X509Time};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const TARGET_HEX: &str = include_str!(
    "../../x509-rsa-sha256-reviewed-server-link/tests/fixtures/valid_isrgrootx1_r13_precertificate.hex"
);
const INTERMEDIATE_HEX: &str =
    include_str!("../../x509-rsa-sha256-reviewed-server-link/tests/fixtures/letsencrypt_r13.hex");
const ROOT_HEX: &str =
    include_str!("../../x509-rsa-sha256-reviewed-anchor-link/tests/fixtures/isrg_root_x1.hex");
// Fixed public-certificate chain observed from sectigo.com on 2026-11-11.
// Tests never contact the network; the checked-in DER exercises a real
// leaf -> intermediate -> intermediate -> reviewed-root RSA chain.
const SECTIGO_TARGET_HEX: &str = include_str!("fixtures/sectigo_com_leaf.der.hex");
const SECTIGO_TARGET_NEAREST_INTERMEDIATE_HEX: &str =
    include_str!("fixtures/sectigo_public_server_authentication_ca_ov_r36.der.hex");
const SECTIGO_ANCHOR_NEAREST_INTERMEDIATE_HEX: &str =
    include_str!("fixtures/sectigo_public_server_authentication_root_r46.der.hex");
const USERTRUST_ROOT_HEX: &str =
    include_str!("fixtures/usertrust_rsa_certification_authority.der.hex");

const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const SHA384_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 12];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const SERVER_AUTH_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 3, 1];
const COUNTRY_NAME_OID: &[u64] = &[2, 5, 4, 6];
const LOCALITY_NAME_OID: &[u64] = &[2, 5, 4, 7];
const STATE_OR_PROVINCE_NAME_OID: &[u64] = &[2, 5, 4, 8];
const ORGANIZATION_NAME_OID: &[u64] = &[2, 5, 4, 10];
const COMMON_NAME_OID: &[u64] = &[2, 5, 4, 3];
const NULL_DER: &[u8] = &[0x05, 0x00];

fn fixture(hexadecimal: &str) -> Vec<u8> {
    let hexadecimal = hexadecimal.trim().replace('\n', "");
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

fn profile() -> X509CertificateAlgorithmProfile<'static> {
    X509CertificateAlgorithmProfile::new(
        SHA256_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        8_192,
        2_048,
        4_096,
    )
    .unwrap()
}

fn sha384_profile() -> X509CertificateAlgorithmProfile<'static> {
    X509CertificateAlgorithmProfile::new(
        SHA384_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        8_192,
        2_048,
        4_096,
    )
    .unwrap()
}

fn algorithm_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    profiles: &[X509CertificateAlgorithmProfile<'static>],
) -> ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded> {
    validate_certificate_algorithm_policy(certificate, profiles).unwrap()
}

fn role_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    role: X509CertificateRole,
) -> ValidatedCertificateRolePolicy<'certificate, 'encoded> {
    validate_certificate_role_policy(certificate, role, Asn1Limits::default(), |_| true).unwrap()
}

fn rsa_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    profiles: &[X509CertificateAlgorithmProfile<'static>],
) -> ValidatedX509RsaAlgorithmPolicy<'certificate, 'encoded> {
    validate_x509_rsa_algorithm_policy(
        algorithm_policy(certificate, profiles),
        RsaPublicKeyPolicy::new(2_048, 4_096, &[65_537]).unwrap(),
        Asn1Limits::default(),
    )
    .unwrap()
}

fn constructed_chain<'target, 'target_encoded, 'candidates, 'candidate_encoded>(
    target: &'target X509Certificate<'target_encoded>,
    candidates: &'candidates [X509Certificate<'candidate_encoded>],
) -> ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded> {
    construct_unique_x509_certificate_path_by(
        target,
        candidates,
        2,
        3,
        |certificate| std::ptr::eq(certificate, &candidates[1]),
        |issuer, subject| issuer.encoded() == subject.encoded(),
    )
    .unwrap()
}

fn constructed_three_issuer_path_by_test_order<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    target: &'target X509Certificate<'target_encoded>,
    candidates: &'candidates [X509Certificate<'candidate_encoded>],
) -> ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded> {
    let mut comparison_index = 0usize;
    construct_unique_x509_certificate_path_by(
        target,
        candidates,
        3,
        4,
        |certificate| std::ptr::eq(certificate, &candidates[2]),
        |_, _| {
            let path_depth = comparison_index / candidates.len();
            let candidate_index = comparison_index % candidates.len();
            comparison_index += 1;
            candidate_index == path_depth
        },
    )
    .unwrap()
}

fn route() -> X509RsaSignatureRoute {
    X509RsaSignatureRoute::new(0, X509RsaSignatureVerifier::RsaPkcs1V15Sha256)
}

fn sha384_route() -> X509RsaSignatureRoute {
    X509RsaSignatureRoute::new(1, X509RsaSignatureVerifier::RsaPkcs1V15Sha384)
}

fn limits() -> RsaPublicOperationLimits {
    RsaPublicOperationLimits::new(512).unwrap()
}

fn exact_der_name_policy<'policy>(
    entries: &'policy [X509NameValuePolicyEntry<'policy>],
) -> X509NameValuePolicy<'policy> {
    X509NameValuePolicy::new(entries).unwrap()
}

fn observed_time() -> X509Time {
    time(b"\x17\x0d260201000000Z")
}

fn time(encoded: &[u8]) -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(encoded).unwrap()).unwrap()
}

fn intermediate_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    profiles: &[X509CertificateAlgorithmProfile<'static>],
    is_self_issued: bool,
) -> ValidatedX509IntermediateCertificatePolicy<'certificate, 'encoded> {
    let role_policy = role_policy(
        certificate,
        X509CertificateRole::IssuingCertificateAuthority,
    );
    let algorithm_policy = algorithm_policy(certificate, profiles);
    let self_issued =
        classify_certificate_self_issued(certificate, |_issuer, _subject| is_self_issued);

    validate_x509_intermediate_certificate_policy(
        role_policy,
        algorithm_policy,
        self_issued,
        &observed_time(),
    )
    .unwrap()
}

fn server_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> ValidatedX509ServerCertificatePolicy<'certificate, 'encoded> {
    server_policy_for(
        certificate,
        "valid-isrgrootx1.letsencrypt.org",
        &observed_time(),
    )
}

fn server_policy_for<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    reference_name: &str,
    observation_time: &X509Time,
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
    let profiles = [profile()];
    let algorithm_policy = validate_certificate_algorithm_policy(certificate, &profiles).unwrap();
    let reference = ReferenceIdentity::parse(reference_name).unwrap();
    let purpose = X509CertificatePurposePolicy::new(SERVER_AUTH_OID).unwrap();

    validate_x509_server_certificate_policy(
        role_policy,
        algorithm_policy,
        &reference,
        observation_time,
        purpose,
        Asn1Limits::default(),
    )
    .unwrap()
}

fn reviewed_path<'path, 'target, 'target_encoded, 'candidates, 'candidate_encoded>(
    path: &'path ConstructedX509CertificatePath<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
) -> BoundX509RsaConstructedReviewedIntermediatePath<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    let profiles = [profile()];
    let intermediate = path.issuer_certificate(0).unwrap();
    let root = path.issuer_certificate(1).unwrap();
    let intermediate_signature = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(path.target_certificate(), &profiles),
        role_policy(
            intermediate,
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(intermediate, &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let intermediate = bind_x509_rsa_constructed_path_intermediate_policy(
        intermediate_signature,
        intermediate_policy(intermediate, &profiles, false),
    )
    .unwrap();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(1).unwrap(),
        algorithm_policy(intermediate.intermediate_policy().certificate(), &profiles),
        role_policy(root, X509CertificateRole::IssuingCertificateAuthority),
        rsa_policy(root, &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(root).unwrap(),
    )
    .unwrap();

    bind_x509_rsa_constructed_intermediate_path_to_reviewed_anchor(intermediate, terminal).unwrap()
}

#[test]
fn authenticates_target_and_candidate_children_and_retains_exact_evidence() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    for (depth, child, issuer) in [
        (0, &target, &candidates[0]),
        (1, &candidates[0], &candidates[1]),
    ] {
        let verified = verify_x509_rsa_constructed_path_policy_signature(
            path.adjacent_link(depth).unwrap(),
            algorithm_policy(child, &profiles),
            role_policy(issuer, X509CertificateRole::IssuingCertificateAuthority),
            rsa_policy(issuer, &profiles),
            &[route()],
            limits(),
        )
        .unwrap();

        assert!(std::ptr::eq(
            verified.adjacent_link().constructed_path(),
            &path
        ));
        assert!(std::ptr::eq(
            verified.adjacent_link().issuer_certificate(),
            issuer
        ));
        assert!(std::ptr::eq(
            verified.issuer_role_policy().certificate(),
            issuer
        ));
        let VerifiedX509RsaPolicySignature::Sha256(signature) = verified.signature() else {
            panic!("selected the wrong verifier")
        };
        assert!(std::ptr::eq(
            signature.child_algorithm_policy().certificate(),
            child
        ));
        assert!(std::ptr::eq(
            signature.issuer_rsa_algorithm_policy().certificate(),
            issuer
        ));
        assert!(std::ptr::eq(signature.signature().certificate(), child));

        if depth == 0 {
            assert!(std::ptr::eq(
                verified.adjacent_link().child_target_certificate().unwrap(),
                child
            ));
            assert!(verified
                .adjacent_link()
                .child_candidate_certificate()
                .is_none());
        } else {
            assert!(verified
                .adjacent_link()
                .child_target_certificate()
                .is_none());
            assert!(std::ptr::eq(
                verified
                    .adjacent_link()
                    .child_candidate_certificate()
                    .unwrap(),
                child
            ));
        }
    }
}

#[test]
fn rejects_byte_equal_detached_child_before_route_selection() {
    let target_encoded = fixture(TARGET_HEX);
    let detached_target_encoded = target_encoded.clone();
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let detached_target = decode(&detached_target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    assert_eq!(target.encoded(), detached_target.encoded());
    let error = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(&detached_target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[],
        limits(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathSignatureRoutingErrorKind::ChildEvidenceMismatch
    );
}

#[test]
fn rejects_byte_equal_detached_role_before_route_selection() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let detached_intermediate_encoded = intermediate_encoded.clone();
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let detached_intermediate = decode(&detached_intermediate_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    assert_eq!(candidates[0].encoded(), detached_intermediate.encoded());
    let error = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &detached_intermediate,
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[],
        limits(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathSignatureRoutingErrorKind::IssuerRoleEvidenceMismatch
    );
}

#[test]
fn rejects_non_ca_issuer_before_route_selection() {
    let child_encoded = fixture(ROOT_HEX);
    let end_entity_encoded = fixture(TARGET_HEX);
    let child = decode(&child_encoded);
    let candidates = vec![decode(&end_entity_encoded)];
    let path = construct_unique_x509_certificate_path_by(
        &child,
        &candidates,
        1,
        2,
        |certificate| std::ptr::eq(certificate, &candidates[0]),
        |_, _| true,
    )
    .unwrap();
    let profiles = [profile()];

    let error = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(&child, &profiles),
        role_policy(&candidates[0], X509CertificateRole::EndEntity),
        rsa_policy(&candidates[0], &profiles),
        &[],
        limits(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathSignatureRoutingErrorKind::IssuerNotCertificateAuthority
    );
}

#[test]
fn rejects_byte_equal_detached_rsa_policy_before_route_selection() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let detached_intermediate_encoded = intermediate_encoded.clone();
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let detached_intermediate = decode(&detached_intermediate_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let error = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&detached_intermediate, &profiles),
        &[],
        limits(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathSignatureRoutingErrorKind::IssuerAlgorithmEvidenceMismatch
    );
}

#[test]
fn propagates_route_failure_only_after_all_evidence_gates() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let error = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[],
        limits(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathSignatureRoutingErrorKind::Signature(
            X509RsaSignatureRoutingErrorKind::UnsupportedProfileRoute
        )
    );
}

#[test]
fn binds_routed_intermediate_policy_and_advances_path_length() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let verified = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let authenticated = bind_x509_rsa_constructed_path_intermediate_policy(
        verified,
        intermediate_policy(&candidates[0], &profiles, false),
    )
    .unwrap();
    let advanced = advance_x509_rsa_constructed_path_intermediate_path_length(
        X509CertificatePathLengthState::new(1),
        authenticated,
    )
    .unwrap();

    assert_eq!(
        advanced
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    assert!(std::ptr::eq(
        advanced
            .authenticated_intermediate()
            .verified_signature()
            .adjacent_link()
            .constructed_path(),
        &path
    ));
    assert_eq!(
        advanced
            .authenticated_intermediate()
            .verified_signature()
            .adjacent_link()
            .issuer_depth(),
        0
    );
    assert!(std::ptr::eq(
        advanced
            .authenticated_intermediate()
            .intermediate_policy()
            .certificate(),
        &candidates[0]
    ));
    assert_eq!(
        format!("{advanced:?}"),
        "ValidatedX509RsaConstructedPathIntermediatePathLength"
    );
}

#[test]
fn advances_policy_backed_initialized_path_length_without_detaching_name_policy() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let profiles = [profile()];
    let policy_entries = [
        X509NameValuePolicyEntry::new(COUNTRY_NAME_OID, X509NameValueComparisonRule::ExactDer),
        X509NameValuePolicyEntry::new(ORGANIZATION_NAME_OID, X509NameValueComparisonRule::ExactDer),
        X509NameValuePolicyEntry::new(COMMON_NAME_OID, X509NameValueComparisonRule::ExactDer),
    ];
    let name_policy = exact_der_name_policy(&policy_entries);
    let path = construct_unique_x509_certificate_path_with_name_policy(
        &target,
        &candidates,
        2,
        3,
        |certificate| std::ptr::eq(certificate, &candidates[1]),
        name_policy,
    )
    .unwrap();
    let reviewed_path = bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path =
        bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
            reviewed_path,
            role_policy(
                &candidates[1],
                X509CertificateRole::IssuingCertificateAuthority,
            ),
        )
        .unwrap();
    let initialized = initialize_name_policy_reviewed_constructed_x509_certificate_path_length(
        reviewed_role_path,
    );
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path_with_name_policy()
        .constructed_path();

    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();

    let advanced =
        advance_initialized_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_length(
            &initialized,
            reviewed_terminal,
            intermediate_policy(&candidates[0], &profiles, false),
        )
        .unwrap();

    assert!(std::ptr::eq(
        advanced.initialized_path_length(),
        &initialized
    ));
    assert!(std::ptr::eq(
        advanced
            .initialized_path_length()
            .reviewed_path()
            .reviewed_path()
            .constructed_path_with_name_policy()
            .name_policy()
            .entries(),
        name_policy.entries()
    ));
    assert_eq!(
        advanced
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    assert_eq!(
        advanced
            .reviewed_terminal()
            .verified_terminal_signature()
            .adjacent_link()
            .issuer_depth(),
        1
    );
    assert!(std::ptr::eq(
        advanced.intermediate_policy().certificate(),
        &candidates[0]
    ));
    assert_eq!(
        format!("{advanced:?}"),
        "AdvancedInitializedNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength"
    );
}

#[test]
fn policy_backed_initialized_transition_rejects_another_construction() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let profiles = [profile()];
    let policy_entries = [
        X509NameValuePolicyEntry::new(COUNTRY_NAME_OID, X509NameValueComparisonRule::ExactDer),
        X509NameValuePolicyEntry::new(ORGANIZATION_NAME_OID, X509NameValueComparisonRule::ExactDer),
        X509NameValuePolicyEntry::new(COMMON_NAME_OID, X509NameValueComparisonRule::ExactDer),
    ];
    let name_policy = exact_der_name_policy(&policy_entries);
    let path = construct_unique_x509_certificate_path_with_name_policy(
        &target,
        &candidates,
        2,
        3,
        |certificate| std::ptr::eq(certificate, &candidates[1]),
        name_policy,
    )
    .unwrap();
    let detached_path = construct_unique_x509_certificate_path_with_name_policy(
        &target,
        &candidates,
        2,
        3,
        |certificate| std::ptr::eq(certificate, &candidates[1]),
        name_policy,
    )
    .unwrap();
    let reviewed_path = bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path =
        bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
            reviewed_path,
            role_policy(
                &candidates[1],
                X509CertificateRole::IssuingCertificateAuthority,
            ),
        )
        .unwrap();
    let initialized = initialize_name_policy_reviewed_constructed_x509_certificate_path_length(
        reviewed_role_path,
    );
    let detached_reviewed_path =
        bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor(
            detached_path,
            select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
        )
        .unwrap();
    let detached_reviewed_role_path =
        bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
            detached_reviewed_path,
            role_policy(
                &candidates[1],
                X509CertificateRole::IssuingCertificateAuthority,
            ),
        )
        .unwrap();
    let detached_initialized =
        initialize_name_policy_reviewed_constructed_x509_certificate_path_length(
            detached_reviewed_role_path,
        );
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path_with_name_policy()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();

    let error =
        advance_initialized_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_length(
            &detached_initialized,
            reviewed_terminal,
            intermediate_policy(&candidates[0], &profiles, false),
        )
        .unwrap_err();

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind::EvidenceMismatch
    );
}

#[test]
fn advances_construction_initialized_path_length_with_anchor_nearest_intermediate() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();

    let advanced = advance_initialized_reviewed_x509_rsa_constructed_path_intermediate_path_length(
        &initialized,
        reviewed_terminal,
        intermediate_policy(&candidates[0], &profiles, false),
    )
    .unwrap();

    assert!(std::ptr::eq(
        advanced.initialized_path_length(),
        &initialized
    ));
    assert_eq!(
        advanced
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    assert_eq!(
        advanced
            .reviewed_terminal()
            .verified_terminal_signature()
            .adjacent_link()
            .issuer_depth(),
        1
    );
    assert!(std::ptr::eq(
        advanced.intermediate_policy().certificate(),
        &candidates[0]
    ));
    assert_eq!(
        format!("{advanced:?}"),
        "AdvancedInitializedReviewedX509RsaConstructedPathIntermediatePathLength"
    );
}

#[test]
fn advances_anchor_nearest_intermediate_first_on_longer_construction() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let mut extra_intermediate_encoded = fixture(ROOT_HEX);
    let last_signature_byte = extra_intermediate_encoded
        .last_mut()
        .expect("root fixture has a signature");
    *last_signature_byte ^= 1;
    let terminal_root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![
        decode(&extra_intermediate_encoded),
        decode(&intermediate_encoded),
        decode(&terminal_root_encoded),
    ];
    let path = constructed_three_issuer_path_by_test_order(&target, &candidates);
    let profiles = [profile()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    assert_eq!(retained_path.issuer_count(), 3);
    let terminal_link = retained_path.adjacent_link(2).unwrap();
    assert!(std::ptr::eq(
        terminal_link.child_candidate_certificate().unwrap(),
        &candidates[1]
    ));
    assert!(std::ptr::eq(
        terminal_link.issuer_certificate(),
        &candidates[2]
    ));
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        terminal_link,
        algorithm_policy(&candidates[1], &profiles),
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[2], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();

    let advanced = advance_initialized_reviewed_x509_rsa_constructed_path_intermediate_path_length(
        &initialized,
        reviewed_terminal,
        intermediate_policy(&candidates[1], &profiles, false),
    )
    .unwrap();

    assert_eq!(
        advanced
            .reviewed_terminal()
            .verified_terminal_signature()
            .adjacent_link()
            .issuer_depth(),
        2
    );
    assert!(std::ptr::eq(
        advanced.intermediate_policy().certificate(),
        &candidates[1]
    ));
    assert_eq!(
        advanced
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
}

#[test]
fn advances_the_immediately_next_authenticated_intermediate() {
    let target_encoded = fixture(SECTIGO_TARGET_HEX);
    let intermediate_encoded = fixture(SECTIGO_TARGET_NEAREST_INTERMEDIATE_HEX);
    let anchor_nearest_encoded = fixture(SECTIGO_ANCHOR_NEAREST_INTERMEDIATE_HEX);
    let terminal_root_encoded = fixture(USERTRUST_ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![
        decode(&intermediate_encoded),
        decode(&anchor_nearest_encoded),
        decode(&terminal_root_encoded),
    ];
    let path = constructed_three_issuer_path_by_test_order(&target, &candidates);
    let profiles = [profile(), sha384_profile()];
    let routes = [route(), sha384_route()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(2).unwrap(),
        algorithm_policy(&candidates[1], &profiles),
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[2], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let first = advance_initialized_reviewed_x509_rsa_constructed_path_intermediate_path_length(
        &initialized,
        reviewed_terminal,
        intermediate_policy(&candidates[1], &profiles, false),
    )
    .unwrap();
    let next_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &routes,
        limits(),
    )
    .unwrap();

    let second = advance_next_reviewed_x509_rsa_constructed_path_intermediate_path_length(
        first,
        next_signature,
        intermediate_policy(&candidates[0], &profiles, false),
    )
    .unwrap();

    assert_eq!(
        second
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    assert_eq!(
        second
            .previous_step()
            .reviewed_terminal()
            .verified_terminal_signature()
            .adjacent_link()
            .issuer_depth(),
        2
    );
    assert_eq!(second.next_signature().adjacent_link().issuer_depth(), 1);
    assert!(std::ptr::eq(
        second.next_signature().adjacent_link().issuer_certificate(),
        &candidates[1]
    ));
    assert!(std::ptr::eq(
        second.intermediate_policy().certificate(),
        &candidates[0]
    ));
    assert_eq!(
        format!("{second:?}"),
        "AdvancedNextReviewedX509RsaConstructedPathIntermediatePathLength"
    );
}

#[test]
fn advances_the_policy_backed_immediately_next_authenticated_intermediate() {
    let target_encoded = fixture(SECTIGO_TARGET_HEX);
    let intermediate_encoded = fixture(SECTIGO_TARGET_NEAREST_INTERMEDIATE_HEX);
    let anchor_nearest_encoded = fixture(SECTIGO_ANCHOR_NEAREST_INTERMEDIATE_HEX);
    let terminal_root_encoded = fixture(USERTRUST_ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![
        decode(&intermediate_encoded),
        decode(&anchor_nearest_encoded),
        decode(&terminal_root_encoded),
    ];
    let profiles = [profile(), sha384_profile()];
    let routes = [route(), sha384_route()];
    let policy_entries = [
        X509NameValuePolicyEntry::new(COUNTRY_NAME_OID, X509NameValueComparisonRule::ExactDer),
        X509NameValuePolicyEntry::new(LOCALITY_NAME_OID, X509NameValueComparisonRule::ExactDer),
        X509NameValuePolicyEntry::new(
            STATE_OR_PROVINCE_NAME_OID,
            X509NameValueComparisonRule::ExactDer,
        ),
        X509NameValuePolicyEntry::new(ORGANIZATION_NAME_OID, X509NameValueComparisonRule::ExactDer),
        X509NameValuePolicyEntry::new(COMMON_NAME_OID, X509NameValueComparisonRule::ExactDer),
    ];
    let name_policy = exact_der_name_policy(&policy_entries);
    let path = construct_unique_x509_certificate_path_with_name_policy(
        &target,
        &candidates,
        3,
        4,
        |certificate| std::ptr::eq(certificate, &candidates[2]),
        name_policy,
    )
    .unwrap();
    let reviewed_path = bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path =
        bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
            reviewed_path,
            role_policy(
                &candidates[2],
                X509CertificateRole::IssuingCertificateAuthority,
            ),
        )
        .unwrap();
    let initialized = initialize_name_policy_reviewed_constructed_x509_certificate_path_length(
        reviewed_role_path,
    );
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path_with_name_policy()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(2).unwrap(),
        algorithm_policy(&candidates[1], &profiles),
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[2], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let first =
        advance_initialized_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_length(
            &initialized,
            reviewed_terminal,
            intermediate_policy(&candidates[1], &profiles, false),
        )
        .unwrap();
    let next_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &routes,
        limits(),
    )
    .unwrap();

    let second =
        advance_next_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_length(
            first,
            next_signature,
            intermediate_policy(&candidates[0], &profiles, false),
        )
        .unwrap();

    assert!(std::ptr::eq(
        second.previous_step().initialized_path_length(),
        &initialized
    ));
    assert!(std::ptr::eq(
        second
            .previous_step()
            .initialized_path_length()
            .reviewed_path()
            .reviewed_path()
            .constructed_path_with_name_policy()
            .name_policy()
            .entries(),
        name_policy.entries()
    ));
    assert_eq!(
        second
            .previous_step()
            .reviewed_terminal()
            .verified_terminal_signature()
            .adjacent_link()
            .issuer_depth(),
        2
    );
    assert_eq!(second.next_signature().adjacent_link().issuer_depth(), 1);
    assert!(std::ptr::eq(
        second.next_signature().adjacent_link().issuer_certificate(),
        &candidates[1]
    ));
    assert!(std::ptr::eq(
        second.intermediate_policy().certificate(),
        &candidates[0]
    ));
    assert_eq!(
        second
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    assert_eq!(
        format!("{second:?}"),
        "AdvancedNextNamePolicyReviewedX509RsaConstructedPathIntermediatePathLength"
    );
}

#[test]
fn policy_backed_next_transition_rejects_another_construction() {
    let target_encoded = fixture(SECTIGO_TARGET_HEX);
    let intermediate_encoded = fixture(SECTIGO_TARGET_NEAREST_INTERMEDIATE_HEX);
    let anchor_nearest_encoded = fixture(SECTIGO_ANCHOR_NEAREST_INTERMEDIATE_HEX);
    let terminal_root_encoded = fixture(USERTRUST_ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![
        decode(&intermediate_encoded),
        decode(&anchor_nearest_encoded),
        decode(&terminal_root_encoded),
    ];
    let profiles = [profile(), sha384_profile()];
    let routes = [route(), sha384_route()];
    let policy_entries = [
        X509NameValuePolicyEntry::new(COUNTRY_NAME_OID, X509NameValueComparisonRule::ExactDer),
        X509NameValuePolicyEntry::new(LOCALITY_NAME_OID, X509NameValueComparisonRule::ExactDer),
        X509NameValuePolicyEntry::new(
            STATE_OR_PROVINCE_NAME_OID,
            X509NameValueComparisonRule::ExactDer,
        ),
        X509NameValuePolicyEntry::new(ORGANIZATION_NAME_OID, X509NameValueComparisonRule::ExactDer),
        X509NameValuePolicyEntry::new(COMMON_NAME_OID, X509NameValueComparisonRule::ExactDer),
    ];
    let name_policy = exact_der_name_policy(&policy_entries);
    let path = construct_unique_x509_certificate_path_with_name_policy(
        &target,
        &candidates,
        3,
        4,
        |certificate| std::ptr::eq(certificate, &candidates[2]),
        name_policy,
    )
    .unwrap();
    let detached_path = constructed_three_issuer_path_by_test_order(&target, &candidates);
    let reviewed_path = bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path =
        bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
            reviewed_path,
            role_policy(
                &candidates[2],
                X509CertificateRole::IssuingCertificateAuthority,
            ),
        )
        .unwrap();
    let initialized = initialize_name_policy_reviewed_constructed_x509_certificate_path_length(
        reviewed_role_path,
    );
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path_with_name_policy()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(2).unwrap(),
        algorithm_policy(&candidates[1], &profiles),
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[2], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let first =
        advance_initialized_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_length(
            &initialized,
            reviewed_terminal,
            intermediate_policy(&candidates[1], &profiles, false),
        )
        .unwrap();
    let detached_next_signature = verify_x509_rsa_constructed_path_policy_signature(
        detached_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &routes,
        limits(),
    )
    .unwrap();

    let error =
        match advance_next_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_length(
            first,
            detached_next_signature,
            intermediate_policy(&candidates[0], &profiles, false),
        ) {
            Err(error) => error,
            Ok(_) => panic!("detached construction must not extend policy-backed evidence"),
        };

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathNextIntermediatePathLengthErrorKind::EvidenceMismatch
    );
}

#[test]
fn accumulates_every_ordered_intermediate_with_bounded_retained_evidence() {
    let target_encoded = fixture(SECTIGO_TARGET_HEX);
    let intermediate_encoded = fixture(SECTIGO_TARGET_NEAREST_INTERMEDIATE_HEX);
    let anchor_nearest_encoded = fixture(SECTIGO_ANCHOR_NEAREST_INTERMEDIATE_HEX);
    let terminal_root_encoded = fixture(USERTRUST_ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![
        decode(&intermediate_encoded),
        decode(&anchor_nearest_encoded),
        decode(&terminal_root_encoded),
    ];
    let path = constructed_three_issuer_path_by_test_order(&target, &candidates);
    let detached_path = constructed_three_issuer_path_by_test_order(&target, &candidates);
    let profiles = [profile(), sha384_profile()];
    let routes = [route(), sha384_route()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(2).unwrap(),
        algorithm_policy(&candidates[1], &profiles),
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[2], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let next_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let detached_signature = verify_x509_rsa_constructed_path_policy_signature(
        detached_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let skipped_target_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let policies = [
        intermediate_policy(&candidates[1], &profiles, false),
        intermediate_policy(&candidates[0], &profiles, false),
    ];
    let reversed_policies = [
        intermediate_policy(&candidates[0], &profiles, false),
        intermediate_policy(&candidates[1], &profiles, false),
    ];
    let ordered_signatures = [next_signature];
    let detached_signatures = [detached_signature];
    let skipped_signatures = [skipped_target_signature];

    let error = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &ordered_signatures,
        &policies,
        0,
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::InvalidIntermediateLimit
    );

    let error = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &ordered_signatures,
        &policies,
        1,
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::IntermediateLimitExceeded
    );

    let error = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &[],
        &policies,
        2,
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::EvidenceCountMismatch
    );

    let error = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &detached_signatures,
        &policies,
        2,
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::EvidenceMismatch
    );

    let error = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &skipped_signatures,
        &policies,
        2,
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::InvalidPathShape
    );

    let error = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &ordered_signatures,
        &reversed_policies,
        2,
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        X509RsaConstructedPathIntermediatePathLengthAccumulationErrorKind::IntermediatePolicyMismatch
    );

    let accumulated = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &ordered_signatures,
        &policies,
        2,
    )
    .unwrap();

    assert!(std::ptr::eq(
        accumulated.initialized_path_length(),
        &initialized
    ));
    assert!(std::ptr::eq(
        accumulated.reviewed_terminal(),
        &reviewed_terminal
    ));
    assert_eq!(accumulated.processed_intermediate_count(), 2);
    assert_eq!(accumulated.ordered_signatures().len(), 1);
    assert!(std::ptr::eq(
        accumulated.ordered_signatures()[0]
            .adjacent_link()
            .issuer_certificate(),
        &candidates[1]
    ));
    assert_eq!(accumulated.intermediate_policies().len(), 2);
    assert!(std::ptr::eq(
        accumulated.intermediate_policies()[1].certificate(),
        &candidates[0]
    ));
    assert_eq!(
        accumulated
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    assert_eq!(
        format!("{accumulated:?}"),
        "AccumulatedReviewedX509RsaConstructedPathIntermediatePathLength"
    );

    let target_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let closed = bind_accumulated_reviewed_x509_rsa_constructed_path_target_signature(
        accumulated,
        target_signature,
        server_policy_for(&target, "sectigo.com", &time(b"\x17\x0d261112000000Z")),
    )
    .unwrap();
    let anchor_policy =
        validate_accumulated_reviewed_x509_rsa_constructed_path_target_anchor_policy(
            closed,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
    let intermediate_name_policy =
        validate_accumulated_reviewed_x509_rsa_constructed_path_target_intermediate_name_constraints_policy(
            anchor_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

    assert_eq!(intermediate_name_policy.constrained_intermediate_count(), 0);
    assert_eq!(intermediate_name_policy.constrained_dns_names(), None);
    assert_eq!(intermediate_name_policy.constrained_names(), None);
    assert_eq!(
        intermediate_name_policy
            .anchor_policy()
            .target_signature_closure()
            .accumulated_intermediates()
            .processed_intermediate_count(),
        2
    );
    assert_eq!(
        format!("{intermediate_name_policy:?}"),
        "ValidatedAccumulatedReviewedX509RsaConstructedPathTargetIntermediateNameConstraintsPolicy"
    );

    let subordinate_name_policy =
        validate_accumulated_reviewed_x509_rsa_constructed_path_subordinate_intermediate_name_constraints_policy(
            intermediate_name_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

    assert_eq!(subordinate_name_policy.constraint_application_count(), 0);
    assert_eq!(subordinate_name_policy.dns_name_evaluation_count(), 0);
    assert_eq!(subordinate_name_policy.ip_address_evaluation_count(), 0);
    assert_eq!(
        subordinate_name_policy
            .target_policy()
            .anchor_policy()
            .target_signature_closure()
            .accumulated_intermediates()
            .processed_intermediate_count(),
        2
    );
    assert_eq!(
        format!("{subordinate_name_policy:?}"),
        "ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateIntermediateNameConstraintsPolicy"
    );

    let subordinate_anchor_policy =
        validate_accumulated_reviewed_x509_rsa_constructed_path_subordinate_anchor_name_constraints_policy(
            subordinate_name_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

    assert_eq!(subordinate_anchor_policy.constraint_application_count(), 0);
    assert_eq!(subordinate_anchor_policy.dns_name_evaluation_count(), 0);
    assert_eq!(
        subordinate_anchor_policy
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure()
            .accumulated_intermediates()
            .processed_intermediate_count(),
        2
    );
    assert_eq!(
        format!("{subordinate_anchor_policy:?}"),
        "ValidatedAccumulatedReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy"
    );
}

#[test]
fn accepts_an_empty_follow_on_slice_for_one_intermediate() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let policies = [intermediate_policy(&candidates[0], &profiles, false)];

    let accumulated = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &[],
        &policies,
        1,
    )
    .unwrap();

    assert_eq!(accumulated.processed_intermediate_count(), 1);
    assert!(accumulated.ordered_signatures().is_empty());
    assert!(std::ptr::eq(
        accumulated.intermediate_policies()[0].certificate(),
        &candidates[0]
    ));
}

#[test]
fn closes_accumulated_intermediates_with_exact_target_signature_and_policy() {
    let target_encoded = fixture(TARGET_HEX);
    let detached_target_encoded = target_encoded.clone();
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let detached_target = decode(&detached_target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let detached_path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let intermediate_policies = [intermediate_policy(&candidates[0], &profiles, false)];
    let no_follow_on_signatures = [];
    let accumulated = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &no_follow_on_signatures,
        &intermediate_policies,
        1,
    )
    .unwrap();

    let non_target_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let error = bind_accumulated_reviewed_x509_rsa_constructed_path_target_signature(
        accumulated,
        non_target_signature,
        server_policy(&target),
    )
    .expect_err("a nonzero-depth proof cannot close the target link");
    assert_eq!(
        error.kind(),
        X509RsaConstructedPathTargetSignatureClosureErrorKind::InvalidPathShape
    );

    let accumulated = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &no_follow_on_signatures,
        &intermediate_policies,
        1,
    )
    .unwrap();

    let detached_target_signature = verify_x509_rsa_constructed_path_policy_signature(
        detached_path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let error = bind_accumulated_reviewed_x509_rsa_constructed_path_target_signature(
        accumulated,
        detached_target_signature,
        server_policy(&target),
    )
    .expect_err("a signature from another construction remains detached evidence");
    assert_eq!(
        error.kind(),
        X509RsaConstructedPathTargetSignatureClosureErrorKind::EvidenceMismatch
    );

    let accumulated = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &no_follow_on_signatures,
        &intermediate_policies,
        1,
    )
    .unwrap();
    let target_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let error = bind_accumulated_reviewed_x509_rsa_constructed_path_target_signature(
        accumulated,
        target_signature,
        server_policy(&detached_target),
    )
    .expect_err("byte-equal target policy from another decode remains detached evidence");
    assert_eq!(
        error.kind(),
        X509RsaConstructedPathTargetSignatureClosureErrorKind::TargetPolicyMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA constructed-path target signature closure error TargetPolicyMismatch"
    );

    let accumulated = accumulate_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
        &initialized,
        &reviewed_terminal,
        &no_follow_on_signatures,
        &intermediate_policies,
        1,
    )
    .unwrap();
    let target_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let closed = bind_accumulated_reviewed_x509_rsa_constructed_path_target_signature(
        accumulated,
        target_signature,
        server_policy(&target),
    )
    .unwrap();

    assert_eq!(
        closed
            .accumulated_intermediates()
            .processed_intermediate_count(),
        1
    );
    assert_eq!(closed.target_signature().adjacent_link().issuer_depth(), 0);
    assert!(std::ptr::eq(
        closed
            .target_signature()
            .adjacent_link()
            .child_target_certificate()
            .unwrap(),
        &target
    ));
    assert!(std::ptr::eq(closed.server_policy().certificate(), &target));
    assert_eq!(
        format!("{closed:?}"),
        "BoundAccumulatedReviewedX509RsaConstructedPathTargetSignature"
    );

    let anchor_policy =
        validate_accumulated_reviewed_x509_rsa_constructed_path_target_anchor_policy(
            closed,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
    assert_eq!(anchor_policy.constrained_dns_names(), None);
    assert_eq!(
        anchor_policy
            .target_signature_closure()
            .accumulated_intermediates()
            .processed_intermediate_count(),
        1
    );
    assert_eq!(
        format!("{anchor_policy:?}"),
        "ValidatedAccumulatedReviewedX509RsaConstructedPathTargetAnchorPolicy"
    );
}

#[test]
fn rejects_next_signature_from_a_distinct_construction() {
    let target_encoded = fixture(SECTIGO_TARGET_HEX);
    let intermediate_encoded = fixture(SECTIGO_TARGET_NEAREST_INTERMEDIATE_HEX);
    let anchor_nearest_encoded = fixture(SECTIGO_ANCHOR_NEAREST_INTERMEDIATE_HEX);
    let terminal_root_encoded = fixture(USERTRUST_ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![
        decode(&intermediate_encoded),
        decode(&anchor_nearest_encoded),
        decode(&terminal_root_encoded),
    ];
    let initialized_path = constructed_three_issuer_path_by_test_order(&target, &candidates);
    let next_signature_path = constructed_three_issuer_path_by_test_order(&target, &candidates);
    let profiles = [profile(), sha384_profile()];
    let routes = [route(), sha384_route()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        initialized_path,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(2).unwrap(),
        algorithm_policy(&candidates[1], &profiles),
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[2], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let first = advance_initialized_reviewed_x509_rsa_constructed_path_intermediate_path_length(
        &initialized,
        reviewed_terminal,
        intermediate_policy(&candidates[1], &profiles, false),
    )
    .unwrap();
    let next_signature = verify_x509_rsa_constructed_path_policy_signature(
        next_signature_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &routes,
        limits(),
    )
    .unwrap();

    let error = match advance_next_reviewed_x509_rsa_constructed_path_intermediate_path_length(
        first,
        next_signature,
        intermediate_policy(&candidates[0], &profiles, false),
    ) {
        Err(error) => error,
        Ok(_) => panic!("byte-identical construction objects remain detached evidence"),
    };

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathNextIntermediatePathLengthErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA constructed-path next intermediate path-length error EvidenceMismatch"
    );
}

#[test]
fn rejects_a_skipped_target_nearer_depth() {
    let target_encoded = fixture(SECTIGO_TARGET_HEX);
    let intermediate_encoded = fixture(SECTIGO_TARGET_NEAREST_INTERMEDIATE_HEX);
    let anchor_nearest_encoded = fixture(SECTIGO_ANCHOR_NEAREST_INTERMEDIATE_HEX);
    let terminal_root_encoded = fixture(USERTRUST_ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![
        decode(&intermediate_encoded),
        decode(&anchor_nearest_encoded),
        decode(&terminal_root_encoded),
    ];
    let path = constructed_three_issuer_path_by_test_order(&target, &candidates);
    let profiles = [profile(), sha384_profile()];
    let routes = [route(), sha384_route()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(2).unwrap(),
        algorithm_policy(&candidates[1], &profiles),
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[2], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let first = advance_initialized_reviewed_x509_rsa_constructed_path_intermediate_path_length(
        &initialized,
        reviewed_terminal,
        intermediate_policy(&candidates[1], &profiles, false),
    )
    .unwrap();
    let skipped_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &routes,
        limits(),
    )
    .unwrap();

    let error = match advance_next_reviewed_x509_rsa_constructed_path_intermediate_path_length(
        first,
        skipped_signature,
        intermediate_policy(&candidates[0], &profiles, false),
    ) {
        Err(error) => error,
        Ok(_) => panic!("ordered traversal must not skip an adjacency depth"),
    };

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathNextIntermediatePathLengthErrorKind::InvalidPathShape
    );
}

#[test]
fn rejects_next_policy_not_bound_to_the_authenticated_child() {
    let target_encoded = fixture(SECTIGO_TARGET_HEX);
    let intermediate_encoded = fixture(SECTIGO_TARGET_NEAREST_INTERMEDIATE_HEX);
    let anchor_nearest_encoded = fixture(SECTIGO_ANCHOR_NEAREST_INTERMEDIATE_HEX);
    let terminal_root_encoded = fixture(USERTRUST_ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![
        decode(&intermediate_encoded),
        decode(&anchor_nearest_encoded),
        decode(&terminal_root_encoded),
    ];
    let path = constructed_three_issuer_path_by_test_order(&target, &candidates);
    let profiles = [profile(), sha384_profile()];
    let routes = [route(), sha384_route()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(2).unwrap(),
        algorithm_policy(&candidates[1], &profiles),
        role_policy(
            &candidates[2],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[2], &profiles),
        &routes,
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[2]).unwrap(),
    )
    .unwrap();
    let first = advance_initialized_reviewed_x509_rsa_constructed_path_intermediate_path_length(
        &initialized,
        reviewed_terminal,
        intermediate_policy(&candidates[1], &profiles, false),
    )
    .unwrap();
    let next_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &routes,
        limits(),
    )
    .unwrap();

    let error = match advance_next_reviewed_x509_rsa_constructed_path_intermediate_path_length(
        first,
        next_signature,
        intermediate_policy(&candidates[1], &profiles, false),
    ) {
        Err(error) => error,
        Ok(_) => panic!("policy for the previous issuer is not policy for the next child"),
    };

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathNextIntermediatePathLengthErrorKind::IntermediatePolicyMismatch
    );
}

#[test]
fn rejects_initialized_path_length_and_intermediate_from_distinct_constructions() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let initialized_path = constructed_chain(&target, &candidates);
    let authenticated_path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        initialized_path,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        authenticated_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();

    let error =
        match advance_initialized_reviewed_x509_rsa_constructed_path_intermediate_path_length(
            &initialized,
            reviewed_terminal,
            intermediate_policy(&candidates[0], &profiles, false),
        ) {
            Err(error) => error,
            Ok(_) => panic!("byte-identical construction objects remain detached evidence"),
        };

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA constructed-path initialized intermediate path-length error EvidenceMismatch"
    );
}

#[test]
fn rejects_intermediate_policy_not_bound_to_terminal_child() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let reviewed_path = bind_constructed_x509_certificate_path_to_reviewed_anchor(
        path,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let reviewed_role_path = bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
        reviewed_path,
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
    )
    .unwrap();
    let initialized =
        initialize_reviewed_constructed_x509_certificate_path_length(reviewed_role_path);
    let retained_path = initialized
        .reviewed_path()
        .reviewed_path()
        .constructed_path();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        retained_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();

    let error =
        match advance_initialized_reviewed_x509_rsa_constructed_path_intermediate_path_length(
            &initialized,
            reviewed_terminal,
            intermediate_policy(&candidates[1], &profiles, true),
        ) {
            Err(error) => error,
            Ok(_) => panic!("reviewed-root policy is not policy for its terminal child"),
        };

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathInitializedIntermediatePathLengthErrorKind::IntermediatePolicyMismatch
    );
}

#[test]
fn rejects_terminal_issuer_as_intermediate() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let verified = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let error = bind_x509_rsa_constructed_path_intermediate_policy(
        verified,
        intermediate_policy(&candidates[1], &profiles, true),
    )
    .expect_err("the construction terminal is not an intermediate");

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathIntermediatePolicyErrorKind::TerminalIssuer
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA constructed-path intermediate policy error TerminalIssuer"
    );
}

#[test]
fn rejects_byte_equal_detached_intermediate_policy() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let detached_intermediate_encoded = intermediate_encoded.clone();
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let detached_intermediate = decode(&detached_intermediate_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];
    assert_eq!(detached_intermediate.encoded(), candidates[0].encoded());

    let verified = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let error = bind_x509_rsa_constructed_path_intermediate_policy(
        verified,
        intermediate_policy(&detached_intermediate, &profiles, false),
    )
    .expect_err("a byte-equal independent decode is detached evidence");

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathIntermediatePolicyErrorKind::EvidenceMismatch
    );
}

#[test]
fn rejects_exhausted_path_length_for_non_self_issued_intermediate() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let verified = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let authenticated = bind_x509_rsa_constructed_path_intermediate_policy(
        verified,
        intermediate_policy(&candidates[0], &profiles, false),
    )
    .unwrap();
    let error = advance_x509_rsa_constructed_path_intermediate_path_length(
        X509CertificatePathLengthState::new(0),
        authenticated,
    )
    .expect_err("the non-self-issued intermediate consumes capacity");

    assert_eq!(
        error.kind(),
        X509CertificatePathLengthErrorKind::MaximumExceeded
    );
}

#[test]
fn binds_routed_terminal_signature_to_exact_reviewed_anchor() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let verified = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let bound = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        verified,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();

    assert_eq!(
        bound
            .verified_terminal_signature()
            .adjacent_link()
            .issuer_depth(),
        1
    );
    assert!(std::ptr::eq(
        bound
            .verified_terminal_signature()
            .adjacent_link()
            .issuer_certificate(),
        bound.reviewed_anchor().candidate_certificate()
    ));
    assert!(std::ptr::eq(
        bound
            .verified_terminal_signature()
            .issuer_role_policy()
            .certificate(),
        bound.reviewed_anchor().candidate_certificate()
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509RsaConstructedPathTerminalToReviewedAnchor"
    );
}

#[test]
fn rejects_routed_nonterminal_signature_as_reviewed_anchor() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let verified = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let error = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        verified,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .expect_err("a nonterminal adjacency must not establish the reviewed terminus");

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathReviewedAnchorErrorKind::NonTerminalIssuer
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA constructed-path reviewed-anchor error NonTerminalIssuer"
    );
}

#[test]
fn rejects_byte_equal_detached_reviewed_anchor() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let detached_root_encoded = root_encoded.clone();
    let target = decode(&target_encoded);
    let detached_root = decode(&detached_root_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];
    assert_eq!(detached_root.encoded(), candidates[1].encoded());

    let verified = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let error = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        verified,
        select_reviewed_webpki_root_anchor(&detached_root).unwrap(),
    )
    .expect_err("byte-equal reviewed-root evidence from another decode is detached");

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathReviewedAnchorErrorKind::EvidenceMismatch
    );
}

#[test]
fn binds_complete_routed_two_link_path_and_derives_initial_path_length() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let intermediate_signature = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let intermediate = bind_x509_rsa_constructed_path_intermediate_policy(
        intermediate_signature,
        intermediate_policy(&candidates[0], &profiles, false),
    )
    .unwrap();

    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();

    let bound =
        bind_x509_rsa_constructed_intermediate_path_to_reviewed_anchor(intermediate, terminal)
            .unwrap();

    assert_eq!(
        bound
            .authenticated_intermediate()
            .path_length_state()
            .remaining_non_self_issued_intermediate_cas(),
        0
    );
    let intermediate_link = bound
        .authenticated_intermediate()
        .authenticated_intermediate()
        .verified_signature()
        .adjacent_link();
    let terminal_link = bound
        .reviewed_terminal()
        .verified_terminal_signature()
        .adjacent_link();
    assert_eq!(intermediate_link.issuer_depth(), 0);
    assert_eq!(terminal_link.issuer_depth(), 1);
    assert!(std::ptr::eq(
        intermediate_link.constructed_path(),
        terminal_link.constructed_path()
    ));
    assert!(std::ptr::eq(
        intermediate_link.issuer_certificate(),
        terminal_link.child_candidate_certificate().unwrap()
    ));
    assert!(std::ptr::eq(
        terminal_link.issuer_certificate(),
        bound
            .reviewed_terminal()
            .reviewed_anchor()
            .candidate_certificate()
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509RsaConstructedReviewedIntermediatePath"
    );
}

#[test]
fn binds_complete_server_policy_to_exact_routed_reviewed_target() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);

    let bound = bind_x509_rsa_constructed_reviewed_intermediate_server_path(
        server_policy(&target),
        reviewed_path(&path),
    )
    .unwrap();

    let constructed_target = bound
        .reviewed_path()
        .authenticated_intermediate()
        .authenticated_intermediate()
        .verified_signature()
        .adjacent_link()
        .constructed_path()
        .target_certificate();
    assert!(std::ptr::eq(
        bound.server_policy().certificate(),
        constructed_target
    ));
    assert!(std::ptr::eq(constructed_target, &target));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509RsaConstructedReviewedIntermediateServerPath"
    );
}

#[test]
fn retains_exact_routed_server_path_through_unconstrained_anchor_and_intermediate_policy() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);
    let bound = bind_x509_rsa_constructed_reviewed_intermediate_server_path(
        server_policy(&target),
        reviewed_path(&path),
    )
    .unwrap();

    let anchor_policy = validate_x509_rsa_constructed_reviewed_intermediate_server_anchor_policy(
        bound,
        Asn1Limits::default(),
        Asn1Limits::default(),
    )
    .unwrap();
    let validated =
        validate_x509_rsa_constructed_reviewed_intermediate_server_name_constraints_policy(
            anchor_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

    assert_eq!(validated.constrained_dns_names(), None);
    assert_eq!(validated.constrained_names(), None);
    assert!(std::ptr::eq(
        validated
            .anchor_policy()
            .reviewed_server_path()
            .server_policy()
            .certificate(),
        &target
    ));
    assert!(std::ptr::eq(
        validated
            .anchor_policy()
            .reviewed_server_path()
            .reviewed_path()
            .reviewed_terminal()
            .reviewed_anchor()
            .candidate_certificate(),
        &candidates[1]
    ));
    assert_eq!(
        format!("{validated:?}"),
        "ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy"
    );
}

#[test]
fn rejects_server_policy_for_byte_equal_detached_target() {
    let target_encoded = fixture(TARGET_HEX);
    let detached_target_encoded = target_encoded.clone();
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let detached_target = decode(&detached_target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let path = constructed_chain(&target, &candidates);

    assert_eq!(target.encoded(), detached_target.encoded());
    let error = bind_x509_rsa_constructed_reviewed_intermediate_server_path(
        server_policy(&detached_target),
        reviewed_path(&path),
    )
    .expect_err("byte-equal independently decoded target policy must not attach");

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathReviewedServerPolicyErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA constructed-path reviewed server policy error EvidenceMismatch"
    );
}

#[test]
fn rejects_two_link_proofs_from_distinct_construction_objects() {
    let target_encoded = fixture(TARGET_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let intermediate_path = constructed_chain(&target, &candidates);
    let terminal_path = constructed_chain(&target, &candidates);
    let profiles = [profile()];

    let intermediate_signature = verify_x509_rsa_constructed_path_policy_signature(
        intermediate_path.adjacent_link(0).unwrap(),
        algorithm_policy(&target, &profiles),
        role_policy(
            &candidates[0],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[0], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let intermediate = bind_x509_rsa_constructed_path_intermediate_policy(
        intermediate_signature,
        intermediate_policy(&candidates[0], &profiles, false),
    )
    .unwrap();

    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        terminal_path.adjacent_link(1).unwrap(),
        algorithm_policy(&candidates[0], &profiles),
        role_policy(
            &candidates[1],
            X509CertificateRole::IssuingCertificateAuthority,
        ),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        limits(),
    )
    .unwrap();
    let terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();

    let error =
        bind_x509_rsa_constructed_intermediate_path_to_reviewed_anchor(intermediate, terminal)
            .expect_err("byte-identical selection from another path object is detached traversal");

    assert_eq!(
        error.kind(),
        X509RsaConstructedPathReviewedIntermediatePathErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 RSA constructed-path reviewed intermediate path error EvidenceMismatch"
    );
}
