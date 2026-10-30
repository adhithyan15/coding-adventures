use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_path_construction::{
    construct_unique_x509_certificate_path_by, ConstructedX509CertificatePath,
};
use x509_certificate_policy::{
    validate_certificate_algorithm_policy, validate_certificate_role_policy,
    ValidatedCertificateAlgorithmPolicy, ValidatedCertificateRolePolicy,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile, X509CertificateRole,
};
use x509_rsa_algorithm_policy::{
    validate_x509_rsa_algorithm_policy, ValidatedX509RsaAlgorithmPolicy,
};
use x509_rsa_constructed_path_signature_routing::{
    verify_x509_rsa_constructed_path_policy_signature,
    X509RsaConstructedPathSignatureRoutingErrorKind,
};
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_signature_routing::{
    VerifiedX509RsaPolicySignature, X509RsaSignatureRoute, X509RsaSignatureRoutingErrorKind,
    X509RsaSignatureVerifier,
};

const TARGET_HEX: &str = include_str!(
    "../../x509-rsa-sha256-reviewed-server-link/tests/fixtures/valid_isrgrootx1_r13_precertificate.hex"
);
const INTERMEDIATE_HEX: &str =
    include_str!("../../x509-rsa-sha256-reviewed-server-link/tests/fixtures/letsencrypt_r13.hex");
const ROOT_HEX: &str =
    include_str!("../../x509-rsa-sha256-reviewed-anchor-link/tests/fixtures/isrg_root_x1.hex");

const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
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

fn route() -> X509RsaSignatureRoute {
    X509RsaSignatureRoute::new(0, X509RsaSignatureVerifier::RsaPkcs1V15Sha256)
}

fn limits() -> RsaPublicOperationLimits {
    RsaPublicOperationLimits::new(512).unwrap()
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
