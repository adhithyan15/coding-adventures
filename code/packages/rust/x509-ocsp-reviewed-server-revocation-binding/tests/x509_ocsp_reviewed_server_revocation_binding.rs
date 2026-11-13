use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use sha1::sum1;
use sha256::sha256;
use tls_server_identity::ReferenceIdentity;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_path_construction::{
    construct_unique_x509_certificate_path_by,
    construct_unique_x509_certificate_path_with_name_policy, ConstructedX509CertificatePath,
};
use x509_certificate_path_length_policy::X509CertificatePathLengthState;
use x509_certificate_path_reviewed_anchor_binding::{
    bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor,
    bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy,
    initialize_name_policy_reviewed_constructed_x509_certificate_path_length,
};
use x509_certificate_policy::{
    classify_certificate_self_issued, validate_certificate_algorithm_policy,
    validate_certificate_role_policy, ValidatedCertificateAlgorithmPolicy,
    ValidatedCertificateValidity, X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile,
    X509CertificatePurposePolicy, X509CertificateRole,
};
use x509_intermediate_certificate_policy::validate_x509_intermediate_certificate_policy;
use x509_name_value_policy::{
    X509NameValueComparisonRule, X509NameValuePolicy, X509NameValuePolicyEntry,
};
use x509_ocsp_authenticated_responder::{
    authenticate_x509_ocsp_rsa_responder, authenticate_x509_ocsp_rsa_sha256_responder,
    X509OcspRsaResponderPolicy, X509OcspRsaSha256ResponderPolicy,
};
use x509_ocsp_authenticated_target_revocation::{
    authenticate_x509_ocsp_rsa_routed_issuer_target_revocation,
    authenticate_x509_ocsp_rsa_sha256_target_revocation,
    authenticate_x509_ocsp_rsa_target_revocation,
};
use x509_ocsp_cert_id_binding::{bind_x509_ocsp_cert_id_to_certificates, X509OcspCertIdHashInput};
use x509_ocsp_issuer_bound_responder::{
    bind_x509_ocsp_authenticated_responder_to_issuer,
    bind_x509_ocsp_authenticated_rsa_responder_to_issuer,
    verify_x509_ocsp_authenticated_rsa_responder_routed_issuer_link,
    IssuerBoundX509OcspRsaRoutedIssuerResponder,
};
use x509_ocsp_responder_certificate_policy::validate_x509_ocsp_responder_certificate_policy;
use x509_ocsp_responder_id_binding::bind_x509_ocsp_responder_id_to_certificate;
use x509_ocsp_response::{
    decode_x509_ocsp_basic_response_bytes, decode_x509_ocsp_response, X509OcspBasicResponse,
};
use x509_ocsp_reviewed_server_revocation_binding::{
    bind_x509_ocsp_revocation_to_reviewed_server_path,
    bind_x509_ocsp_revocation_to_routed_reviewed_server_path,
    bind_x509_ocsp_routed_issuer_reviewed_intermediate_revocation,
    bind_x509_ocsp_routed_issuer_reviewed_no_check_intermediate_revocation,
    bind_x509_ocsp_routed_issuer_reviewed_responder_revocation,
    bind_x509_ocsp_routed_issuer_to_accumulated_name_policy_reviewed_server_path,
    bind_x509_ocsp_routed_issuer_to_routed_reviewed_server_path,
    bind_x509_ocsp_routed_response_to_routed_reviewed_server_path,
    reify_x509_ocsp_reviewed_responder_path, reify_x509_ocsp_routed_issuer_reviewed_responder_path,
    validate_x509_ocsp_reviewed_responder_anchor_dns_policy,
    validate_x509_ocsp_reviewed_responder_intermediate_dns_policy,
    validate_x509_ocsp_reviewed_responder_no_check_policy,
    validate_x509_ocsp_routed_issuer_reviewed_responder_anchor_dns_policy,
    validate_x509_ocsp_routed_issuer_reviewed_responder_intermediate_dns_policy,
    validate_x509_ocsp_routed_issuer_reviewed_responder_no_check_policy,
    X509OcspReviewedIntermediateRevocationBindingErrorKind,
    X509OcspReviewedResponderRevocationBindingErrorKind,
    X509OcspReviewedServerRevocationBindingErrorKind,
};
use x509_ocsp_revocation_policy::validate_bound_x509_ocsp_revocation;
use x509_ocsp_rsa_algorithm_policy::{
    X509OcspRsaAlgorithmParametersPolicy, X509OcspRsaAlgorithmProfile,
};
use x509_ocsp_rsa_signature_routing::{X509OcspRsaSignatureRoute, X509OcspRsaSignatureVerifier};
use x509_rsa_algorithm_policy::{
    validate_x509_rsa_algorithm_policy, ValidatedX509RsaAlgorithmPolicy,
};
use x509_rsa_constructed_path_signature_routing::{
    accumulate_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_lengths,
    bind_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_signature,
    bind_x509_rsa_constructed_intermediate_path_to_reviewed_anchor,
    bind_x509_rsa_constructed_path_intermediate_policy,
    bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor,
    bind_x509_rsa_constructed_reviewed_intermediate_server_path,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_anchor_name_constraints_policy,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_intermediate_name_constraints_policy,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_anchor_policy,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_intermediate_name_constraints_policy,
    validate_x509_rsa_constructed_reviewed_intermediate_server_anchor_policy,
    validate_x509_rsa_constructed_reviewed_intermediate_server_name_constraints_policy,
    verify_x509_rsa_constructed_path_policy_signature,
    BoundX509RsaConstructedReviewedIntermediatePath,
};
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_sha256_issuer_link::{
    advance_constructed_x509_rsa_sha256_intermediate_path_length,
    verify_constructed_x509_rsa_sha256_intermediate_issuer_policy_link,
    verify_constructed_x509_rsa_sha256_issuer_link, verify_x509_rsa_sha256_issuer_link,
    X509RsaSha256IssuerLinkPolicy,
};
use x509_rsa_sha256_reviewed_anchor_link::{
    bind_x509_rsa_sha256_constructed_intermediate_path_to_reviewed_anchor,
    BoundX509RsaSha256ConstructedReviewedIntermediatePath,
};
use x509_rsa_sha256_reviewed_server_link::{
    bind_x509_rsa_sha256_constructed_reviewed_intermediate_server_path,
    validate_x509_rsa_sha256_constructed_reviewed_intermediate_server_anchor_policy,
    validate_x509_rsa_sha256_constructed_reviewed_intermediate_server_name_constraints_policy,
};
use x509_rsa_signature_routing::{X509RsaSignatureRoute, X509RsaSignatureVerifier};
use x509_server_certificate_policy::validate_x509_server_certificate_policy;
use x509_time::{decode_x509_time, X509Time};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const LEAF_HEX: &str = include_str!("fixtures/apple-www-leaf.der.hex");
const INTERMEDIATE_HEX: &str = include_str!("fixtures/apple-ev-intermediate.der.hex");
const ROOT_HEX: &str = include_str!("fixtures/digicert-global-root-g2.der.hex");
const OCSP_RESPONSE_HEX: &str = include_str!("fixtures/apple-stapled-ocsp-response.der.hex");

const LEAF_SHA256: &str = "cd563f5716c254cc1a574cb9f101a923ca50b2822c94c7c85575fff4b809e760";
const INTERMEDIATE_SHA256: &str =
    "340ca5ba402d140b65a2c976e7ae8128a1505c29d190e0e034f59ccae7a92bc2";
const RESPONDER_SHA256: &str = "b3b8febf1451170c65a5211252b74b1a972d09cfabfd5d3484de72aa0dd7110d";
const ROOT_SHA256: &str = "cb3ccbb76031e5e0138f8dd39a23f9de47ffc35e43c1144cea27d46a5ab1cb5f";
const OCSP_RESPONSE_SHA256: &str =
    "757b42d0666a438b039a6c0e7ab646db5c70913c4e52dd2aceb645763d16ae79";

const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const SHA1_OID: &[u64] = &[1, 3, 14, 3, 2, 26];
const SERVER_AUTH_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 3, 1];
const COUNTRY_NAME_OID: &[u64] = &[2, 5, 4, 6];
const ORGANIZATION_NAME_OID: &[u64] = &[2, 5, 4, 10];
const ORGANIZATIONAL_UNIT_NAME_OID: &[u64] = &[2, 5, 4, 11];
const COMMON_NAME_OID: &[u64] = &[2, 5, 4, 3];
const NULL_DER: &[u8] = &[0x05, 0x00];
const ALLOWED_EXPONENTS: &[u64] = &[65_537];
const FIXTURE_PROOF_STACK_BYTES: usize = 16 * 1024 * 1024;

fn run_fixture_proof_test(test: fn()) {
    std::thread::Builder::new()
        .name("x509-ocsp-reviewed-server-revocation-fixture".into())
        .stack_size(FIXTURE_PROOF_STACK_BYTES)
        .spawn(test)
        .expect("fixture proof thread must start")
        .join()
        .expect("fixture proof thread must complete");
}

fn fixture(hexadecimal: &str) -> Vec<u8> {
    let digits = hexadecimal
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    assert!(digits.len().is_multiple_of(2));
    digits
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

fn decode_response(input: &[u8]) -> X509OcspBasicResponse<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(input).unwrap();
    let response = decode_x509_ocsp_response(&mut decoder, root).unwrap();
    decode_x509_ocsp_basic_response_bytes(&mut decoder, response.response_bytes().unwrap()).unwrap()
}

fn observed_time() -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(b"\x17\x0d261025120000Z").unwrap()).unwrap()
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
        RsaPublicKeyPolicy::new(2_048, 4_096, ALLOWED_EXPONENTS).unwrap(),
        Asn1Limits::default(),
        RsaPublicOperationLimits::new(512).unwrap(),
    )
}

fn algorithm_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    profiles: &[X509CertificateAlgorithmProfile<'static>],
) -> ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded> {
    validate_certificate_algorithm_policy(certificate, profiles).unwrap()
}

fn rsa_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    profiles: &[X509CertificateAlgorithmProfile<'static>],
) -> ValidatedX509RsaAlgorithmPolicy<'certificate, 'encoded> {
    validate_x509_rsa_algorithm_policy(
        algorithm_policy(certificate, profiles),
        RsaPublicKeyPolicy::new(2_048, 4_096, ALLOWED_EXPONENTS).unwrap(),
        Asn1Limits::default(),
    )
    .unwrap()
}

fn issuing_ca_role_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> x509_certificate_policy::ValidatedCertificateRolePolicy<'certificate, 'encoded> {
    validate_certificate_role_policy(
        certificate,
        X509CertificateRole::IssuingCertificateAuthority,
        Asn1Limits::default(),
        |_| true,
    )
    .unwrap()
}

fn route() -> X509RsaSignatureRoute {
    X509RsaSignatureRoute::new(0, X509RsaSignatureVerifier::RsaPkcs1V15Sha256)
}

fn operation_limits() -> RsaPublicOperationLimits {
    RsaPublicOperationLimits::new(512).unwrap()
}

fn server_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
) -> x509_server_certificate_policy::ValidatedX509ServerCertificatePolicy<'certificate, 'encoded> {
    let role_policy = validate_certificate_role_policy(
        certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]) || oid.equals(&[2, 5, 29, 19]),
    )
    .unwrap();
    let profiles = algorithm_profiles();
    let algorithm_policy = validate_certificate_algorithm_policy(certificate, &profiles).unwrap();
    let reference = ReferenceIdentity::parse("www.apple.com").unwrap();
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

fn intermediate_policy<'certificate, 'encoded>(
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
            intermediate_policy(intermediate),
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

fn routed_constructed_reviewed_path<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
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
    let profiles = algorithm_profiles();
    let intermediate = path.issuer_certificate(0).unwrap();
    let root = path.issuer_certificate(1).unwrap();
    let intermediate_signature = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(0).unwrap(),
        algorithm_policy(path.target_certificate(), &profiles),
        issuing_ca_role_policy(intermediate),
        rsa_policy(intermediate, &profiles),
        &[route()],
        operation_limits(),
    )
    .unwrap();
    let intermediate = bind_x509_rsa_constructed_path_intermediate_policy(
        intermediate_signature,
        intermediate_policy(intermediate),
    )
    .unwrap();
    let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
        path.adjacent_link(1).unwrap(),
        algorithm_policy(intermediate.intermediate_policy().certificate(), &profiles),
        issuing_ca_role_policy(root),
        rsa_policy(root, &profiles),
        &[route()],
        operation_limits(),
    )
    .unwrap();
    let terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
        terminal_signature,
        select_reviewed_webpki_root_anchor(root).unwrap(),
    )
    .unwrap();

    bind_x509_rsa_constructed_intermediate_path_to_reviewed_anchor(intermediate, terminal).unwrap()
}

fn issuer_link<'child, 'child_encoded, 'issuer, 'issuer_encoded>(
    child: &'child X509Certificate<'child_encoded>,
    issuer: &'issuer X509Certificate<'issuer_encoded>,
) -> x509_rsa_sha256_issuer_link::VerifiedX509RsaSha256IssuerLink<
    'child,
    'child_encoded,
    'issuer,
    'issuer_encoded,
> {
    verify_x509_rsa_sha256_issuer_link(
        child,
        issuer,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        link_policy(),
    )
    .unwrap()
}

fn issuer_bound_responder<'response, 'response_encoded, 'issuer, 'issuer_encoded>(
    response: &'response X509OcspBasicResponse<'response_encoded>,
    issuer: &'issuer X509Certificate<'issuer_encoded>,
) -> x509_ocsp_issuer_bound_responder::IssuerBoundX509OcspRsaSha256Responder<
    'response,
    'response_encoded,
    'response,
    'response_encoded,
    'issuer,
    'issuer_encoded,
> {
    let responder = response.get(0).unwrap();
    let responder_id =
        bind_x509_ocsp_responder_id_to_certificate(response, responder, |left, right| {
            left.encoded() == right.encoded()
        })
        .unwrap();
    let role_policy = validate_certificate_role_policy(
        responder,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| {
            oid.equals(&[2, 5, 29, 15])
                || oid.equals(&[2, 5, 29, 19])
                || oid.equals(&[2, 5, 29, 37])
        },
    )
    .unwrap();
    let profiles = algorithm_profiles();
    let responder_algorithm_policy =
        validate_certificate_algorithm_policy(responder, &profiles).unwrap();
    let certificate_policy = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        responder_algorithm_policy,
        &observed_time(),
    )
    .unwrap();
    let authenticated = authenticate_x509_ocsp_rsa_sha256_responder(
        responder_id,
        certificate_policy,
        X509OcspRsaSha256ResponderPolicy::new(
            RsaPublicKeyPolicy::new(2_048, 4_096, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            RsaPublicOperationLimits::new(512).unwrap(),
        ),
    )
    .unwrap();
    bind_x509_ocsp_authenticated_responder_to_issuer(authenticated, issuer_link(responder, issuer))
        .unwrap()
}

fn issuer_bound_routed_responder<'response, 'response_encoded, 'issuer, 'issuer_encoded>(
    response: &'response X509OcspBasicResponse<'response_encoded>,
    issuer: &'issuer X509Certificate<'issuer_encoded>,
) -> x509_ocsp_issuer_bound_responder::IssuerBoundX509OcspRsaResponder<
    'response,
    'response_encoded,
    'response,
    'response_encoded,
    'issuer,
    'issuer_encoded,
> {
    let responder = response.get(0).unwrap();
    let responder_id =
        bind_x509_ocsp_responder_id_to_certificate(response, responder, |left, right| {
            left.encoded() == right.encoded()
        })
        .unwrap();
    let role_policy = validate_certificate_role_policy(
        responder,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| {
            oid.equals(&[2, 5, 29, 15])
                || oid.equals(&[2, 5, 29, 19])
                || oid.equals(&[2, 5, 29, 37])
        },
    )
    .unwrap();
    let profiles = algorithm_profiles();
    let responder_algorithm_policy =
        validate_certificate_algorithm_policy(responder, &profiles).unwrap();
    let certificate_policy = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        responder_algorithm_policy,
        &observed_time(),
    )
    .unwrap();
    let response_profiles = [X509OcspRsaAlgorithmProfile::new(
        SHA256_WITH_RSA_ENCRYPTION_OID,
        X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        8_192,
        2_048,
        4_096,
        ALLOWED_EXPONENTS,
    )
    .unwrap()];
    let response_routes = [X509OcspRsaSignatureRoute::new(
        0,
        X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha256,
    )];
    let authenticated = authenticate_x509_ocsp_rsa_responder(
        responder_id,
        certificate_policy,
        X509OcspRsaResponderPolicy::new(
            RsaPublicKeyPolicy::new(2_048, 4_096, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            &response_profiles,
            &response_routes,
            RsaPublicOperationLimits::new(512).unwrap(),
        ),
    )
    .unwrap();
    bind_x509_ocsp_authenticated_rsa_responder_to_issuer(
        authenticated,
        issuer_link(responder, issuer),
    )
    .unwrap()
}

fn issuer_bound_routed_issuer_responder<'response, 'response_encoded, 'issuer, 'issuer_encoded>(
    response: &'response X509OcspBasicResponse<'response_encoded>,
    issuer: &'issuer X509Certificate<'issuer_encoded>,
) -> IssuerBoundX509OcspRsaRoutedIssuerResponder<
    'response,
    'response_encoded,
    'response,
    'response_encoded,
    'issuer,
    'issuer_encoded,
> {
    let responder = response.get(0).unwrap();
    let responder_id =
        bind_x509_ocsp_responder_id_to_certificate(response, responder, |left, right| {
            left.encoded() == right.encoded()
        })
        .unwrap();
    let role_policy = validate_certificate_role_policy(
        responder,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| {
            oid.equals(&[2, 5, 29, 15])
                || oid.equals(&[2, 5, 29, 19])
                || oid.equals(&[2, 5, 29, 37])
        },
    )
    .unwrap();
    let profiles = algorithm_profiles();
    let responder_algorithm_policy =
        validate_certificate_algorithm_policy(responder, &profiles).unwrap();
    let certificate_policy = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        responder_algorithm_policy,
        &observed_time(),
    )
    .unwrap();
    let response_profiles = [X509OcspRsaAlgorithmProfile::new(
        SHA256_WITH_RSA_ENCRYPTION_OID,
        X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        8_192,
        2_048,
        4_096,
        ALLOWED_EXPONENTS,
    )
    .unwrap()];
    let response_routes = [X509OcspRsaSignatureRoute::new(
        0,
        X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha256,
    )];
    let authenticated = authenticate_x509_ocsp_rsa_responder(
        responder_id,
        certificate_policy,
        X509OcspRsaResponderPolicy::new(
            RsaPublicKeyPolicy::new(2_048, 4_096, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            &response_profiles,
            &response_routes,
            RsaPublicOperationLimits::new(512).unwrap(),
        ),
    )
    .unwrap();

    verify_x509_ocsp_authenticated_rsa_responder_routed_issuer_link(
        authenticated,
        algorithm_policy(responder, &profiles),
        issuing_ca_role_policy(issuer),
        rsa_policy(issuer, &profiles),
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        &[route()],
        operation_limits(),
    )
    .unwrap()
}

macro_rules! authenticate_revocation {
    ($binding:ident, $authenticated:ident, $response:expr, $target:expr, $issuer:expr) => {
        let issuer_bound = issuer_bound_responder($response, $issuer);
        let $binding = bind_x509_ocsp_cert_id_to_certificates(
            $response.response_data().get(0).unwrap(),
            $target,
            $issuer,
            |algorithm, input: X509OcspCertIdHashInput<'_>| {
                algorithm
                    .algorithm()
                    .equals(SHA1_OID)
                    .then(|| sum1(input.bytes()))
            },
        )
        .unwrap();
        let revocation = validate_bound_x509_ocsp_revocation(&$binding, observed_time()).unwrap();
        let $authenticated = authenticate_x509_ocsp_rsa_sha256_target_revocation(
            issuer_bound,
            revocation,
            issuer_link($target, $issuer),
        )
        .unwrap();
    };
}

macro_rules! authenticate_routed_revocation {
    ($binding:ident, $authenticated:ident, $response:expr, $target:expr, $issuer:expr) => {
        let issuer_bound = issuer_bound_routed_responder($response, $issuer);
        let $binding = bind_x509_ocsp_cert_id_to_certificates(
            $response.response_data().get(0).unwrap(),
            $target,
            $issuer,
            |algorithm, input: X509OcspCertIdHashInput<'_>| {
                algorithm
                    .algorithm()
                    .equals(SHA1_OID)
                    .then(|| sum1(input.bytes()))
            },
        )
        .unwrap();
        let revocation = validate_bound_x509_ocsp_revocation(&$binding, observed_time()).unwrap();
        let $authenticated = authenticate_x509_ocsp_rsa_target_revocation(
            issuer_bound,
            revocation,
            issuer_link($target, $issuer),
        )
        .unwrap();
    };
}

macro_rules! authenticate_routed_issuer_revocation {
    ($binding:ident, $authenticated:ident, $response:expr, $target:expr, $issuer:expr) => {
        let issuer_bound = issuer_bound_routed_issuer_responder($response, $issuer);
        let $binding = bind_x509_ocsp_cert_id_to_certificates(
            $response.response_data().get(0).unwrap(),
            $target,
            $issuer,
            |algorithm, input: X509OcspCertIdHashInput<'_>| {
                algorithm
                    .algorithm()
                    .equals(SHA1_OID)
                    .then(|| sum1(input.bytes()))
            },
        )
        .unwrap();
        let revocation = validate_bound_x509_ocsp_revocation(&$binding, observed_time()).unwrap();
        let profiles = algorithm_profiles();
        let $authenticated = authenticate_x509_ocsp_rsa_routed_issuer_target_revocation(
            issuer_bound,
            revocation,
            algorithm_policy($target, &profiles),
            rsa_policy($issuer, &profiles),
            |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
            &[route()],
            operation_limits(),
        )
        .unwrap();
    };
}

macro_rules! reviewed_server_path {
    ($validated:ident, $path:ident, $target:expr, $candidates:expr) => {
        let $path = construct_unique_x509_certificate_path_by(
            $target,
            $candidates,
            2,
            3,
            |certificate| std::ptr::eq(certificate, &$candidates[1]),
            |child_issuer, candidate_subject| {
                child_issuer.encoded() == candidate_subject.encoded()
            },
        )
        .unwrap();
        let reviewed = constructed_reviewed_path(&$path);
        let server = bind_x509_rsa_sha256_constructed_reviewed_intermediate_server_path(
            server_policy($target),
            reviewed,
        )
        .unwrap();
        let anchor = validate_x509_rsa_sha256_constructed_reviewed_intermediate_server_anchor_policy(
            server,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
        let $validated =
            validate_x509_rsa_sha256_constructed_reviewed_intermediate_server_name_constraints_policy(
                anchor,
                Asn1Limits::default(),
                Asn1Limits::default(),
            )
            .unwrap();
    };
}

macro_rules! routed_reviewed_server_path {
    ($validated:ident, $path:ident, $target:expr, $candidates:expr) => {
        let $path = construct_unique_x509_certificate_path_by(
            $target,
            $candidates,
            2,
            3,
            |certificate| std::ptr::eq(certificate, &$candidates[1]),
            |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        )
        .unwrap();
        let reviewed = routed_constructed_reviewed_path(&$path);
        let server = bind_x509_rsa_constructed_reviewed_intermediate_server_path(
            server_policy($target),
            reviewed,
        )
        .unwrap();
        let anchor = validate_x509_rsa_constructed_reviewed_intermediate_server_anchor_policy(
            server,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
        let $validated =
            validate_x509_rsa_constructed_reviewed_intermediate_server_name_constraints_policy(
                anchor,
                Asn1Limits::default(),
                Asn1Limits::default(),
            )
            .unwrap();
    };
}

macro_rules! accumulated_name_policy_reviewed_server_path {
    ($validated:ident, $target:expr, $candidates:expr) => {
        let name_policy_entries = [
            X509NameValuePolicyEntry::new(
                COUNTRY_NAME_OID,
                X509NameValueComparisonRule::ExactDer,
            ),
            X509NameValuePolicyEntry::new(
                ORGANIZATION_NAME_OID,
                X509NameValueComparisonRule::ExactDer,
            ),
            X509NameValuePolicyEntry::new(
                ORGANIZATIONAL_UNIT_NAME_OID,
                X509NameValueComparisonRule::ExactDer,
            ),
            X509NameValuePolicyEntry::new(
                COMMON_NAME_OID,
                X509NameValueComparisonRule::ExactDer,
            ),
        ];
        let name_policy = X509NameValuePolicy::new(&name_policy_entries).unwrap();
        let constructed = construct_unique_x509_certificate_path_with_name_policy(
            $target,
            $candidates,
            2,
            3,
            |certificate| std::ptr::eq(certificate, &$candidates[1]),
            name_policy,
        )
        .unwrap();
        let reviewed = bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor(
            constructed,
            select_reviewed_webpki_root_anchor(&$candidates[1]).unwrap(),
        )
        .unwrap();
        let reviewed_role =
            bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy(
                reviewed,
                issuing_ca_role_policy(&$candidates[1]),
            )
            .unwrap();
        let initialized =
            initialize_name_policy_reviewed_constructed_x509_certificate_path_length(
                reviewed_role,
            );
        let retained_path = initialized
            .reviewed_path()
            .reviewed_path()
            .constructed_path_with_name_policy()
            .constructed_path();
        let profiles = algorithm_profiles();
        let terminal_signature = verify_x509_rsa_constructed_path_policy_signature(
            retained_path.adjacent_link(1).unwrap(),
            algorithm_policy(&$candidates[0], &profiles),
            issuing_ca_role_policy(&$candidates[1]),
            rsa_policy(&$candidates[1], &profiles),
            &[route()],
            operation_limits(),
        )
        .unwrap();
        let reviewed_terminal = bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor(
            terminal_signature,
            select_reviewed_webpki_root_anchor(&$candidates[1]).unwrap(),
        )
        .unwrap();
        let intermediate_policies = [intermediate_policy(&$candidates[0])];
        let accumulated =
            accumulate_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
                &initialized,
                &reviewed_terminal,
                &[],
                &intermediate_policies,
                1,
            )
            .unwrap();
        let target_signature = verify_x509_rsa_constructed_path_policy_signature(
            retained_path.adjacent_link(0).unwrap(),
            algorithm_policy($target, &profiles),
            issuing_ca_role_policy(&$candidates[0]),
            rsa_policy(&$candidates[0], &profiles),
            &[route()],
            operation_limits(),
        )
        .unwrap();
        let target_closure =
            bind_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_signature(
                accumulated,
                target_signature,
                server_policy($target),
            )
            .unwrap();
        let anchor_policy =
            validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_anchor_policy(
                target_closure,
                Asn1Limits::default(),
                Asn1Limits::default(),
            )
            .unwrap();
        let target_name_policy =
            validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_intermediate_name_constraints_policy(
                anchor_policy,
                Asn1Limits::default(),
                Asn1Limits::default(),
            )
            .unwrap();
        let subordinate_policy =
            validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_intermediate_name_constraints_policy(
                target_name_policy,
                Asn1Limits::default(),
                Asn1Limits::default(),
            )
            .unwrap();
        let $validated =
            validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_anchor_name_constraints_policy(
                subordinate_policy,
                Asn1Limits::default(),
                Asn1Limits::default(),
            )
            .unwrap();
    };
}

#[test]
fn public_fixtures_match_the_recorded_capture() {
    for (encoded, expected) in [
        (fixture(LEAF_HEX), LEAF_SHA256),
        (fixture(INTERMEDIATE_HEX), INTERMEDIATE_SHA256),
        (fixture(ROOT_HEX), ROOT_SHA256),
        (fixture(OCSP_RESPONSE_HEX), OCSP_RESPONSE_SHA256),
    ] {
        assert_eq!(sha256(&encoded).as_slice(), fixture(expected).as_slice());
    }

    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let response = decode_response(&response_encoded);
    assert_eq!(response.len(), 1);
    assert_eq!(
        sha256(response.get(0).unwrap().encoded()).as_slice(),
        fixture(RESPONDER_SHA256).as_slice()
    );
}

#[test]
fn binds_authenticated_ocsp_to_the_same_exact_reviewed_server_path() {
    run_fixture_proof_test(binds_authenticated_ocsp_to_the_same_exact_reviewed_server_path_inner);
}

fn binds_authenticated_ocsp_to_the_same_exact_reviewed_server_path_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_revocation!(binding, authenticated, &response, &leaf, &candidates[0]);
    reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let bound =
        bind_x509_ocsp_revocation_to_reviewed_server_path(authenticated, reviewed_server).unwrap();
    let responder_path = reify_x509_ocsp_reviewed_responder_path(bound);
    let bound = responder_path.reviewed_server_revocation();

    assert!(std::ptr::eq(
        bound
            .authenticated_revocation()
            .target_issuer_link()
            .child(),
        &leaf
    ));
    assert!(std::ptr::eq(
        bound
            .reviewed_server_path()
            .anchor_policy()
            .reviewed_intermediate_server_path()
            .reviewed_intermediate_path()
            .reviewed_anchor()
            .candidate_certificate(),
        &candidates[1]
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509OcspReviewedServerRevocation"
    );
    assert_eq!(
        format!("{:?}", bound.target_validity_at_observation()),
        "ValidatedCertificateValidity { private: () }"
    );
    assert_eq!(
        bound.issuer_validity_at_observation(),
        ValidatedCertificateValidity::clone(&bound.issuer_validity_at_observation())
    );
    assert!(std::ptr::eq(
        bound
            .authenticated_revocation()
            .issuer_bound_responder()
            .responder_issuer_link()
            .candidate_issuer(),
        bound
            .reviewed_server_path()
            .anchor_policy()
            .reviewed_intermediate_server_path()
            .reviewed_intermediate_path()
            .authenticated_path()
            .authenticated_intermediate()
            .intermediate_policy()
            .certificate()
    ));
    assert_eq!(
        format!("{responder_path:?}"),
        "BoundX509OcspReviewedResponderPath"
    );
    let responder_anchor_policy = validate_x509_ocsp_reviewed_responder_anchor_dns_policy(
        responder_path,
        Asn1Limits::default(),
        Asn1Limits::default(),
    )
    .unwrap();
    assert_eq!(responder_anchor_policy.constrained_dns_names(), None);
    assert!(std::ptr::eq(
        responder_anchor_policy
            .responder_path()
            .reviewed_server_revocation()
            .authenticated_revocation()
            .issuer_bound_responder()
            .authenticated_responder()
            .responder_certificate_policy()
            .certificate(),
        response.get(0).unwrap()
    ));
    assert_eq!(
        format!("{responder_anchor_policy:?}"),
        "ValidatedX509OcspReviewedResponderAnchorDnsPolicy"
    );
    let responder_intermediate_policy =
        validate_x509_ocsp_reviewed_responder_intermediate_dns_policy(
            responder_anchor_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
    assert_eq!(responder_intermediate_policy.constrained_dns_names(), None);
    assert!(std::ptr::eq(
        responder_intermediate_policy
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .authenticated_revocation()
            .issuer_bound_responder()
            .authenticated_responder()
            .responder_certificate_policy()
            .certificate(),
        response.get(0).unwrap()
    ));
    assert_eq!(
        format!("{responder_intermediate_policy:?}"),
        "ValidatedX509OcspReviewedResponderIntermediateDnsPolicy"
    );
    let responder_no_check_policy =
        validate_x509_ocsp_reviewed_responder_no_check_policy(responder_intermediate_policy)
            .unwrap();
    assert!(std::ptr::eq(
        responder_no_check_policy
            .intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .authenticated_revocation()
            .issuer_bound_responder()
            .authenticated_responder()
            .responder_certificate_policy()
            .certificate(),
        response.get(0).unwrap()
    ));
    assert_eq!(
        format!("{responder_no_check_policy:?}"),
        "ValidatedX509OcspReviewedResponderNoCheckPolicy"
    );
    let _ = binding;
    let _ = path;
}

#[test]
fn rejects_a_byte_equal_distinct_target_before_issuer_comparison() {
    run_fixture_proof_test(rejects_a_byte_equal_distinct_target_before_issuer_comparison_inner);
}

fn rejects_a_byte_equal_distinct_target_before_issuer_comparison_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let ocsp_leaf = decode(&leaf_encoded);
    let path_leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_revocation!(
        binding,
        authenticated,
        &response,
        &ocsp_leaf,
        &candidates[0]
    );
    reviewed_server_path!(reviewed_server, path, &path_leaf, &candidates);

    let error = bind_x509_ocsp_revocation_to_reviewed_server_path(authenticated, reviewed_server)
        .expect_err("byte-equal distinct target objects must not bind");

    assert_eq!(
        error.kind(),
        X509OcspReviewedServerRevocationBindingErrorKind::TargetMismatch
    );
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("apple"));
    assert!(!diagnostic.contains("6c869e"));
    let _ = binding;
    let _ = path;
}

#[test]
fn rejects_a_byte_equal_distinct_issuer_after_matching_target() {
    run_fixture_proof_test(rejects_a_byte_equal_distinct_issuer_after_matching_target_inner);
}

fn rejects_a_byte_equal_distinct_issuer_after_matching_target_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let ocsp_issuer = decode(&intermediate_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_revocation!(binding, authenticated, &response, &leaf, &ocsp_issuer);
    reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let error = bind_x509_ocsp_revocation_to_reviewed_server_path(authenticated, reviewed_server)
        .expect_err("byte-equal distinct issuer objects must not bind");

    assert_eq!(
        error.kind(),
        X509OcspReviewedServerRevocationBindingErrorKind::IssuerMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP reviewed-server revocation binding error IssuerMismatch"
    );
    let _ = binding;
    let _ = path;
}

#[test]
fn binds_authenticated_ocsp_to_the_same_exact_routed_reviewed_server_path() {
    run_fixture_proof_test(
        binds_authenticated_ocsp_to_the_same_exact_routed_reviewed_server_path_inner,
    );
}

fn binds_authenticated_ocsp_to_the_same_exact_routed_reviewed_server_path_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_revocation!(binding, authenticated, &response, &leaf, &candidates[0]);
    routed_reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let bound =
        bind_x509_ocsp_revocation_to_routed_reviewed_server_path(authenticated, reviewed_server)
            .unwrap();

    assert!(std::ptr::eq(
        bound
            .authenticated_revocation()
            .target_issuer_link()
            .child(),
        &leaf
    ));
    assert!(std::ptr::eq(
        bound
            .routed_reviewed_server_path()
            .anchor_policy()
            .reviewed_server_path()
            .reviewed_path()
            .reviewed_terminal()
            .reviewed_anchor()
            .candidate_certificate(),
        &candidates[1]
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509OcspRoutedReviewedServerRevocation"
    );
    assert_eq!(
        format!("{:?}", bound.target_validity_at_observation()),
        "ValidatedCertificateValidity { private: () }"
    );
    assert_eq!(
        bound.issuer_validity_at_observation(),
        ValidatedCertificateValidity::clone(&bound.issuer_validity_at_observation())
    );
    let _ = binding;
    let _ = path;
}

#[test]
fn routed_binding_rejects_a_byte_equal_distinct_target_before_issuer_comparison() {
    run_fixture_proof_test(
        routed_binding_rejects_a_byte_equal_distinct_target_before_issuer_comparison_inner,
    );
}

fn routed_binding_rejects_a_byte_equal_distinct_target_before_issuer_comparison_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let ocsp_leaf = decode(&leaf_encoded);
    let path_leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_revocation!(
        binding,
        authenticated,
        &response,
        &ocsp_leaf,
        &candidates[0]
    );
    routed_reviewed_server_path!(reviewed_server, path, &path_leaf, &candidates);

    let error =
        bind_x509_ocsp_revocation_to_routed_reviewed_server_path(authenticated, reviewed_server)
            .expect_err("byte-equal distinct target objects must not bind");

    assert_eq!(
        error.kind(),
        X509OcspReviewedServerRevocationBindingErrorKind::TargetMismatch
    );
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("apple"));
    assert!(!diagnostic.contains("6c869e"));
    let _ = binding;
    let _ = path;
}

#[test]
fn routed_binding_rejects_a_byte_equal_distinct_issuer_after_matching_target() {
    run_fixture_proof_test(
        routed_binding_rejects_a_byte_equal_distinct_issuer_after_matching_target_inner,
    );
}

fn routed_binding_rejects_a_byte_equal_distinct_issuer_after_matching_target_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let ocsp_issuer = decode(&intermediate_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_revocation!(binding, authenticated, &response, &leaf, &ocsp_issuer);
    routed_reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let error =
        bind_x509_ocsp_revocation_to_routed_reviewed_server_path(authenticated, reviewed_server)
            .expect_err("byte-equal distinct issuer objects must not bind");

    assert_eq!(
        error.kind(),
        X509OcspReviewedServerRevocationBindingErrorKind::IssuerMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP reviewed-server revocation binding error IssuerMismatch"
    );
    let _ = binding;
    let _ = path;
}

#[test]
fn binds_routed_response_to_the_same_exact_routed_reviewed_server_path() {
    run_fixture_proof_test(
        binds_routed_response_to_the_same_exact_routed_reviewed_server_path_inner,
    );
}

fn binds_routed_response_to_the_same_exact_routed_reviewed_server_path_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_revocation!(binding, authenticated, &response, &leaf, &candidates[0]);
    routed_reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let bound = bind_x509_ocsp_routed_response_to_routed_reviewed_server_path(
        authenticated,
        reviewed_server,
    )
    .unwrap();

    assert_eq!(
        bound
            .authenticated_revocation()
            .issuer_bound_responder()
            .authenticated_responder()
            .response_signature()
            .verifier(),
        X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha256
    );
    assert!(std::ptr::eq(
        bound
            .authenticated_revocation()
            .target_issuer_link()
            .child(),
        &leaf
    ));
    assert!(std::ptr::eq(
        bound
            .routed_reviewed_server_path()
            .anchor_policy()
            .reviewed_server_path()
            .reviewed_path()
            .reviewed_terminal()
            .reviewed_anchor()
            .candidate_certificate(),
        &candidates[1]
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509OcspRoutedResponseReviewedServerRevocation"
    );
    let _ = bound.target_validity_at_observation();
    let _ = bound.issuer_validity_at_observation();
    let _ = binding;
    let _ = path;
}

#[test]
fn routed_response_binding_rejects_a_byte_equal_distinct_target() {
    run_fixture_proof_test(routed_response_binding_rejects_a_byte_equal_distinct_target_inner);
}

fn routed_response_binding_rejects_a_byte_equal_distinct_target_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let ocsp_leaf = decode(&leaf_encoded);
    let path_leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_revocation!(
        binding,
        authenticated,
        &response,
        &ocsp_leaf,
        &candidates[0]
    );
    routed_reviewed_server_path!(reviewed_server, path, &path_leaf, &candidates);

    let error = bind_x509_ocsp_routed_response_to_routed_reviewed_server_path(
        authenticated,
        reviewed_server,
    )
    .expect_err("byte-equal distinct target objects must not bind");

    assert_eq!(
        error.kind(),
        X509OcspReviewedServerRevocationBindingErrorKind::TargetMismatch
    );
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("apple"));
    assert!(!diagnostic.contains("6c869e"));
    let _ = binding;
    let _ = path;
}

#[test]
fn routed_response_binding_rejects_a_byte_equal_distinct_issuer() {
    run_fixture_proof_test(routed_response_binding_rejects_a_byte_equal_distinct_issuer_inner);
}

fn routed_response_binding_rejects_a_byte_equal_distinct_issuer_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let ocsp_issuer = decode(&intermediate_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_revocation!(binding, authenticated, &response, &leaf, &ocsp_issuer);
    routed_reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let error = bind_x509_ocsp_routed_response_to_routed_reviewed_server_path(
        authenticated,
        reviewed_server,
    )
    .expect_err("byte-equal distinct issuer objects must not bind");

    assert_eq!(
        error.kind(),
        X509OcspReviewedServerRevocationBindingErrorKind::IssuerMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP reviewed-server revocation binding error IssuerMismatch"
    );
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("apple"));
    assert!(!diagnostic.contains("6c869e"));
    let _ = binding;
    let _ = path;
}

#[test]
fn binds_routed_issuer_revocation_to_the_same_exact_routed_reviewed_server_path() {
    run_fixture_proof_test(
        binds_routed_issuer_revocation_to_the_same_exact_routed_reviewed_server_path_inner,
    );
}

fn binds_routed_issuer_revocation_to_the_same_exact_routed_reviewed_server_path_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_issuer_revocation!(
        binding,
        authenticated,
        &response,
        &leaf,
        &candidates[0]
    );
    routed_reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let bound =
        bind_x509_ocsp_routed_issuer_to_routed_reviewed_server_path(authenticated, reviewed_server)
            .unwrap();
    let responder_path = reify_x509_ocsp_routed_issuer_reviewed_responder_path(bound);

    assert_eq!(
        format!("{responder_path:?}"),
        "BoundX509OcspReviewedResponderPath"
    );
    let responder_anchor_policy =
        validate_x509_ocsp_routed_issuer_reviewed_responder_anchor_dns_policy(
            responder_path,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
    assert_eq!(responder_anchor_policy.constrained_dns_names(), None);
    assert_eq!(
        format!("{responder_anchor_policy:?}"),
        "ValidatedX509OcspReviewedResponderAnchorDnsPolicy"
    );
    assert!(std::ptr::eq(
        responder_anchor_policy
            .responder_path()
            .reviewed_server_revocation()
            .authenticated_revocation()
            .issuer_bound_responder()
            .authenticated_responder()
            .responder_certificate_policy()
            .certificate(),
        response.get(0).unwrap()
    ));
    let responder_intermediate_policy =
        validate_x509_ocsp_routed_issuer_reviewed_responder_intermediate_dns_policy(
            responder_anchor_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
    assert_eq!(responder_intermediate_policy.constrained_dns_names(), None);
    assert_eq!(
        format!("{responder_intermediate_policy:?}"),
        "ValidatedX509OcspReviewedResponderIntermediateDnsPolicy"
    );
    assert!(std::ptr::eq(
        responder_intermediate_policy
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .authenticated_revocation()
            .issuer_bound_responder()
            .authenticated_responder()
            .responder_certificate_policy()
            .certificate(),
        response.get(0).unwrap()
    ));
    assert!(std::ptr::eq(
        responder_intermediate_policy
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .routed_reviewed_server_path()
            .anchor_policy()
            .reviewed_server_path()
            .reviewed_path()
            .authenticated_intermediate()
            .authenticated_intermediate()
            .intermediate_policy()
            .certificate(),
        &candidates[0]
    ));
    let responder_no_check_policy =
        validate_x509_ocsp_routed_issuer_reviewed_responder_no_check_policy(
            responder_intermediate_policy,
        )
        .unwrap();
    assert_eq!(
        format!("{responder_no_check_policy:?}"),
        "ValidatedX509OcspReviewedResponderNoCheckPolicy"
    );
    assert!(std::ptr::eq(
        responder_no_check_policy
            .intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation()
            .authenticated_revocation()
            .issuer_bound_responder()
            .authenticated_responder()
            .responder_certificate_policy()
            .certificate(),
        response.get(0).unwrap()
    ));
    let bound = responder_no_check_policy
        .intermediate_dns_policy()
        .anchor_policy()
        .responder_path()
        .reviewed_server_revocation();

    assert_eq!(
        bound
            .authenticated_revocation()
            .issuer_bound_responder()
            .authenticated_responder()
            .response_signature()
            .verifier(),
        X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha256
    );
    assert_eq!(
        bound
            .authenticated_revocation()
            .issuer_bound_responder()
            .responder_issuer_signature()
            .verifier(),
        X509RsaSignatureVerifier::RsaPkcs1V15Sha256
    );
    assert_eq!(
        bound
            .authenticated_revocation()
            .target_issuer_signature()
            .verifier(),
        X509RsaSignatureVerifier::RsaPkcs1V15Sha256
    );
    assert!(std::ptr::eq(
        bound
            .authenticated_revocation()
            .target_issuer_signature()
            .child_algorithm_policy()
            .certificate(),
        &leaf
    ));
    assert!(std::ptr::eq(
        bound
            .routed_reviewed_server_path()
            .anchor_policy()
            .reviewed_server_path()
            .reviewed_path()
            .reviewed_terminal()
            .reviewed_anchor()
            .candidate_certificate(),
        &candidates[1]
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509OcspRoutedIssuerReviewedServerRevocation"
    );
    let _ = bound.target_validity_at_observation();
    let _ = bound.issuer_validity_at_observation();
    let _ = binding;
    let _ = path;
}

#[test]
fn binds_routed_issuer_revocation_to_the_same_exact_strong_name_policy_path() {
    run_fixture_proof_test(
        binds_routed_issuer_revocation_to_the_same_exact_strong_name_policy_path_inner,
    );
}

fn binds_routed_issuer_revocation_to_the_same_exact_strong_name_policy_path_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_issuer_revocation!(
        binding,
        authenticated,
        &response,
        &leaf,
        &candidates[0]
    );
    accumulated_name_policy_reviewed_server_path!(strong_path, &leaf, &candidates);

    let bound = bind_x509_ocsp_routed_issuer_to_accumulated_name_policy_reviewed_server_path(
        authenticated,
        strong_path,
    )
    .unwrap();

    assert!(std::ptr::eq(
        bound
            .strong_name_policy_path()
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure()
            .server_policy()
            .certificate(),
        &leaf
    ));
    assert!(std::ptr::eq(
        bound
            .strong_name_policy_path()
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure()
            .target_signature()
            .signature()
            .issuer_rsa_algorithm_policy()
            .certificate(),
        &candidates[0]
    ));
    assert_eq!(
        format!("{bound:?}"),
        "BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation"
    );
    assert_eq!(
        format!("{:?}", bound.target_validity_at_observation()),
        "ValidatedCertificateValidity { private: () }"
    );
    assert_eq!(
        bound.issuer_validity_at_observation(),
        ValidatedCertificateValidity::clone(&bound.issuer_validity_at_observation())
    );
    let _ = binding;
}

#[test]
fn strong_name_policy_binding_rejects_a_byte_equal_distinct_target_before_issuer() {
    run_fixture_proof_test(
        strong_name_policy_binding_rejects_a_byte_equal_distinct_target_before_issuer_inner,
    );
}

fn strong_name_policy_binding_rejects_a_byte_equal_distinct_target_before_issuer_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let ocsp_leaf = decode(&leaf_encoded);
    let path_leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_issuer_revocation!(
        binding,
        authenticated,
        &response,
        &ocsp_leaf,
        &candidates[0]
    );
    accumulated_name_policy_reviewed_server_path!(strong_path, &path_leaf, &candidates);

    let error = bind_x509_ocsp_routed_issuer_to_accumulated_name_policy_reviewed_server_path(
        authenticated,
        strong_path,
    )
    .expect_err("byte-equal distinct target objects must not bind");

    assert_eq!(
        error.kind(),
        X509OcspReviewedServerRevocationBindingErrorKind::TargetMismatch
    );
    assert!(!format!("{error:?} {error}").contains("apple"));
    let _ = binding;
}

#[test]
fn strong_name_policy_binding_rejects_a_byte_equal_distinct_issuer_after_target() {
    run_fixture_proof_test(
        strong_name_policy_binding_rejects_a_byte_equal_distinct_issuer_after_target_inner,
    );
}

fn strong_name_policy_binding_rejects_a_byte_equal_distinct_issuer_after_target_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let ocsp_issuer = decode(&intermediate_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_issuer_revocation!(binding, authenticated, &response, &leaf, &ocsp_issuer);
    accumulated_name_policy_reviewed_server_path!(strong_path, &leaf, &candidates);

    let error = bind_x509_ocsp_routed_issuer_to_accumulated_name_policy_reviewed_server_path(
        authenticated,
        strong_path,
    )
    .expect_err("byte-equal distinct issuer objects must not bind");

    assert_eq!(
        error.kind(),
        X509OcspReviewedServerRevocationBindingErrorKind::IssuerMismatch
    );
    assert!(!format!("{error:?} {error}").contains("apple"));
    let _ = binding;
}

#[test]
fn routed_responder_revocation_binding_rejects_another_exact_target() {
    run_fixture_proof_test(routed_responder_revocation_binding_rejects_another_exact_target_inner);
}

#[allow(clippy::err_expect)] // The non-copyable success proof deliberately has no Debug.
fn routed_responder_revocation_binding_rejects_another_exact_target_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_issuer_revocation!(
        server_binding,
        server_revocation,
        &response,
        &leaf,
        &candidates[0]
    );
    authenticate_routed_issuer_revocation!(
        detached_binding,
        detached_revocation,
        &response,
        &leaf,
        &candidates[0]
    );
    routed_reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let bound = bind_x509_ocsp_routed_issuer_to_routed_reviewed_server_path(
        server_revocation,
        reviewed_server,
    )
    .unwrap();
    let responder_path = reify_x509_ocsp_routed_issuer_reviewed_responder_path(bound);
    let anchor_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_anchor_dns_policy(
        responder_path,
        Asn1Limits::default(),
        Asn1Limits::default(),
    )
    .unwrap();
    let intermediate_dns_policy =
        validate_x509_ocsp_routed_issuer_reviewed_responder_intermediate_dns_policy(
            anchor_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

    let error = bind_x509_ocsp_routed_issuer_reviewed_responder_revocation(
        intermediate_dns_policy,
        detached_revocation,
    )
    .err()
    .expect("the leaf revocation proof must not bind to the routed responder");

    assert_eq!(
        error.kind(),
        X509OcspReviewedResponderRevocationBindingErrorKind::TargetMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP reviewed-responder revocation binding error TargetMismatch"
    );
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("apple"));
    assert!(!diagnostic.contains("6c869e"));
    let _ = server_binding;
    let _ = detached_binding;
    let _ = path;
}

#[test]
fn routed_intermediate_revocation_binding_rejects_another_exact_target() {
    run_fixture_proof_test(
        routed_intermediate_revocation_binding_rejects_another_exact_target_inner,
    );
}

#[allow(clippy::err_expect)] // The non-copyable success proof deliberately has no Debug.
fn routed_intermediate_revocation_binding_rejects_another_exact_target_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_issuer_revocation!(
        server_binding,
        server_revocation,
        &response,
        &leaf,
        &candidates[0]
    );
    authenticate_routed_issuer_revocation!(
        detached_binding,
        detached_revocation,
        &response,
        &leaf,
        &candidates[0]
    );
    routed_reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let bound = bind_x509_ocsp_routed_issuer_to_routed_reviewed_server_path(
        server_revocation,
        reviewed_server,
    )
    .unwrap();
    let error =
        bind_x509_ocsp_routed_issuer_reviewed_intermediate_revocation(bound, detached_revocation)
            .err()
            .expect("the leaf revocation proof must not bind to the retained intermediate");

    assert_eq!(
        error.kind(),
        X509OcspReviewedIntermediateRevocationBindingErrorKind::TargetMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP reviewed-intermediate revocation binding error TargetMismatch"
    );
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("apple"));
    assert!(!diagnostic.contains("6c869e"));
    let _ = server_binding;
    let _ = detached_binding;
    let _ = path;
}

#[test]
fn routed_no_check_intermediate_revocation_binding_rejects_another_exact_target() {
    run_fixture_proof_test(
        routed_no_check_intermediate_revocation_binding_rejects_another_exact_target_inner,
    );
}

#[allow(clippy::err_expect)] // The non-copyable success proof deliberately has no Debug.
fn routed_no_check_intermediate_revocation_binding_rejects_another_exact_target_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_issuer_revocation!(
        server_binding,
        server_revocation,
        &response,
        &leaf,
        &candidates[0]
    );
    authenticate_routed_issuer_revocation!(
        detached_binding,
        detached_revocation,
        &response,
        &leaf,
        &candidates[0]
    );
    routed_reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let bound = bind_x509_ocsp_routed_issuer_to_routed_reviewed_server_path(
        server_revocation,
        reviewed_server,
    )
    .unwrap();
    let responder_path = reify_x509_ocsp_routed_issuer_reviewed_responder_path(bound);
    let anchor_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_anchor_dns_policy(
        responder_path,
        Asn1Limits::default(),
        Asn1Limits::default(),
    )
    .unwrap();
    let intermediate_dns_policy =
        validate_x509_ocsp_routed_issuer_reviewed_responder_intermediate_dns_policy(
            anchor_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
    let no_check_policy = validate_x509_ocsp_routed_issuer_reviewed_responder_no_check_policy(
        intermediate_dns_policy,
    )
    .unwrap();

    let error = bind_x509_ocsp_routed_issuer_reviewed_no_check_intermediate_revocation(
        no_check_policy,
        detached_revocation,
    )
    .err()
    .expect("the leaf revocation proof must not bind as intermediate status");

    assert_eq!(
        error.kind(),
        X509OcspReviewedIntermediateRevocationBindingErrorKind::TargetMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP reviewed-intermediate revocation binding error TargetMismatch"
    );
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("apple"));
    assert!(!diagnostic.contains("6c869e"));
    let _ = server_binding;
    let _ = detached_binding;
    let _ = path;
}

#[test]
fn routed_issuer_binding_rejects_a_byte_equal_distinct_target_before_issuer() {
    run_fixture_proof_test(
        routed_issuer_binding_rejects_a_byte_equal_distinct_target_before_issuer_inner,
    );
}

fn routed_issuer_binding_rejects_a_byte_equal_distinct_target_before_issuer_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let ocsp_leaf = decode(&leaf_encoded);
    let path_leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_issuer_revocation!(
        binding,
        authenticated,
        &response,
        &ocsp_leaf,
        &candidates[0]
    );
    routed_reviewed_server_path!(reviewed_server, path, &path_leaf, &candidates);

    let error =
        bind_x509_ocsp_routed_issuer_to_routed_reviewed_server_path(authenticated, reviewed_server)
            .expect_err("byte-equal distinct target objects must not bind");

    assert_eq!(
        error.kind(),
        X509OcspReviewedServerRevocationBindingErrorKind::TargetMismatch
    );
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("apple"));
    assert!(!diagnostic.contains("6c869e"));
    let _ = binding;
    let _ = path;
}

#[test]
fn routed_issuer_binding_rejects_a_byte_equal_distinct_issuer_after_target() {
    run_fixture_proof_test(
        routed_issuer_binding_rejects_a_byte_equal_distinct_issuer_after_target_inner,
    );
}

fn routed_issuer_binding_rejects_a_byte_equal_distinct_issuer_after_target_inner() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let response_encoded = fixture(OCSP_RESPONSE_HEX);
    let leaf = decode(&leaf_encoded);
    let ocsp_issuer = decode(&intermediate_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    let response = decode_response(&response_encoded);
    authenticate_routed_issuer_revocation!(binding, authenticated, &response, &leaf, &ocsp_issuer);
    routed_reviewed_server_path!(reviewed_server, path, &leaf, &candidates);

    let error =
        bind_x509_ocsp_routed_issuer_to_routed_reviewed_server_path(authenticated, reviewed_server)
            .expect_err("byte-equal distinct issuer objects must not bind");

    assert_eq!(
        error.kind(),
        X509OcspReviewedServerRevocationBindingErrorKind::IssuerMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP reviewed-server revocation binding error IssuerMismatch"
    );
    let _ = binding;
    let _ = path;
}
