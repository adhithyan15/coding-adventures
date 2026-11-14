use der_asn1::{Asn1Decoder, Asn1Limits};
use http1_request_host_authority::{
    validate_http1_request_host_authority, Http1RequestHostAuthorityLimits,
};
use https_destination_name_binding::bind_https_dns_destination;
use https_reviewed_server_path_binding::{
    bind_https_dns_destination_to_reviewed_server_path, HttpsReviewedServerPathBindingError,
};
use rsa_public_operation::RsaPublicOperationLimits;
use tls13_server_name::TlsServerName;
use tls_server_identity::ReferenceIdentity;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_path_construction::construct_unique_x509_certificate_path_with_name_policy;
use x509_certificate_path_reviewed_anchor_binding::{
    bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor,
    bind_name_policy_reviewed_constructed_x509_certificate_path_to_anchor_role_policy,
    initialize_name_policy_reviewed_constructed_x509_certificate_path_length,
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
    accumulate_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_lengths,
    bind_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_signature,
    bind_x509_rsa_constructed_path_terminal_to_reviewed_anchor,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_anchor_name_constraints_policy,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_intermediate_name_constraints_policy,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_anchor_policy,
    validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_intermediate_name_constraints_policy,
    verify_x509_rsa_constructed_path_policy_signature,
};
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_signature_routing::{X509RsaSignatureRoute, X509RsaSignatureVerifier};
use x509_server_certificate_policy::{
    validate_x509_server_certificate_policy, ValidatedX509ServerCertificatePolicy,
};
use x509_time::{decode_x509_time, X509Time};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const TARGET_HEX: &str = include_str!(
    "../../x509-rsa-constructed-path-signature-routing/tests/fixtures/sectigo_com_leaf.der.hex"
);
const TARGET_NEAREST_INTERMEDIATE_HEX: &str = include_str!(
    "../../x509-rsa-constructed-path-signature-routing/tests/fixtures/sectigo_public_server_authentication_ca_ov_r36.der.hex"
);
const ANCHOR_NEAREST_INTERMEDIATE_HEX: &str = include_str!(
    "../../x509-rsa-constructed-path-signature-routing/tests/fixtures/sectigo_public_server_authentication_root_r46.der.hex"
);
const ROOT_HEX: &str = include_str!(
    "../../x509-rsa-constructed-path-signature-routing/tests/fixtures/usertrust_rsa_certification_authority.der.hex"
);

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

fn time(encoded: &[u8]) -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(encoded).unwrap()).unwrap()
}

fn observed_time() -> X509Time {
    time(b"\x17\x0d260201000000Z")
}

fn intermediate_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    profiles: &[X509CertificateAlgorithmProfile<'static>],
) -> ValidatedX509IntermediateCertificatePolicy<'certificate, 'encoded> {
    let role_policy = role_policy(
        certificate,
        X509CertificateRole::IssuingCertificateAuthority,
    );
    let algorithm_policy = algorithm_policy(certificate, profiles);
    let self_issued = classify_certificate_self_issued(certificate, |_, _| false);

    validate_x509_intermediate_certificate_policy(
        role_policy,
        algorithm_policy,
        self_issued,
        &observed_time(),
    )
    .unwrap()
}

fn server_policy_for<'certificate, 'encoded>(
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
    let profiles = [profile()];
    let algorithm_policy = validate_certificate_algorithm_policy(certificate, &profiles).unwrap();
    let reference = ReferenceIdentity::parse("sectigo.com").unwrap();
    let purpose = X509CertificatePurposePolicy::new(SERVER_AUTH_OID).unwrap();

    validate_x509_server_certificate_policy(
        role_policy,
        algorithm_policy,
        &reference,
        &time(b"\x17\x0d261112000000Z"),
        purpose,
        Asn1Limits::default(),
    )
    .unwrap()
}

fn exercise_binding(detached_destination: bool) {
    let target_encoded = fixture(TARGET_HEX);
    let detached_target_encoded = target_encoded.clone();
    let intermediate_encoded = fixture(TARGET_NEAREST_INTERMEDIATE_HEX);
    let anchor_nearest_encoded = fixture(ANCHOR_NEAREST_INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let target = decode(&target_encoded);
    let detached_target = decode(&detached_target_encoded);
    let candidates = vec![
        decode(&intermediate_encoded),
        decode(&anchor_nearest_encoded),
        decode(&root_encoded),
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
        |certificate| core::ptr::eq(certificate, &candidates[2]),
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
    let policies = [
        intermediate_policy(&candidates[1], &profiles),
        intermediate_policy(&candidates[0], &profiles),
    ];
    let ordered_signatures = [next_signature];
    let accumulated =
        accumulate_name_policy_reviewed_x509_rsa_constructed_path_intermediate_path_lengths(
            &initialized,
            &reviewed_terminal,
            &ordered_signatures,
            &policies,
            2,
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
        &routes,
        limits(),
    )
    .unwrap();
    let closed = bind_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_signature(
        accumulated,
        target_signature,
        server_policy_for(&target),
    )
    .unwrap();
    let anchor_policy =
        validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_anchor_policy(
            closed,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
    let intermediate_name_policy =
        validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_target_intermediate_name_constraints_policy(
            anchor_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
    let subordinate_name_policy =
        validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_intermediate_name_constraints_policy(
            intermediate_name_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();
    let reviewed_server_path =
        validate_accumulated_name_policy_reviewed_x509_rsa_constructed_path_subordinate_anchor_name_constraints_policy(
            subordinate_name_policy,
            Asn1Limits::default(),
            Asn1Limits::default(),
        )
        .unwrap();

    let destination_certificate = if detached_destination {
        &detached_target
    } else {
        &target
    };
    let server_name = TlsServerName::new("sectigo.com").unwrap();
    let host = validate_http1_request_host_authority(
        b"SECTIGO.COM",
        Some(443),
        Http1RequestHostAuthorityLimits::new(253, 259).unwrap(),
    )
    .unwrap();
    let destination = bind_https_dns_destination(
        destination_certificate,
        server_name,
        host,
        Asn1Limits::default(),
    )
    .unwrap();

    if detached_destination {
        let error =
            bind_https_dns_destination_to_reviewed_server_path(destination, reviewed_server_path)
                .unwrap_err();
        assert_eq!(
            error,
            HttpsReviewedServerPathBindingError::TargetCertificateMismatch
        );
        assert_eq!(format!("{error:?}"), "TargetCertificateMismatch");
        assert!(!error.to_string().contains("sectigo.com"));
        return;
    }

    let bound =
        bind_https_dns_destination_to_reviewed_server_path(destination, reviewed_server_path)
            .unwrap();
    assert!(core::ptr::eq(bound.destination().certificate(), &target));
    assert!(core::ptr::eq(
        bound
            .reviewed_server_path()
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure()
            .server_policy()
            .certificate(),
        &target,
    ));
    assert_eq!(bound.destination().server_name().as_str(), "sectigo.com");
    assert_eq!(bound.destination().host_authority().host(), b"SECTIGO.COM");
    assert_eq!(bound.destination().host_authority().port(), Some(443));

    let debug = format!("{bound:?}");
    assert_eq!(debug.matches("[REDACTED]").count(), 2);
    assert!(!debug.contains("sectigo.com"));
    assert!(!debug.contains("SECTIGO.COM"));
}

#[test]
fn binds_the_complete_destination_to_the_same_reviewed_path_leaf() {
    exercise_binding(false);
}

#[test]
fn rejects_a_byte_equal_independently_decoded_destination_leaf() {
    exercise_binding(true);
}
