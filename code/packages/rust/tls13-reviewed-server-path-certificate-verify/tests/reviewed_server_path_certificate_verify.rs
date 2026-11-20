use core::ptr;

use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use der_asn1::{Asn1Decoder, Asn1Limits};
use http1_request_host_authority::{
    validate_http1_request_host_authority, Http1RequestHostAuthorityLimits,
};
use https_destination_name_binding::bind_https_dns_destination;
use https_reviewed_server_path_binding::{
    bind_https_dns_destination_to_reviewed_server_path,
    bind_https_dns_destination_to_reviewed_server_path_anchor_self_signature,
};
use rsa_pss_sha256_signature::RsaPssSha256SignatureLimits;
use rsa_public_operation::RsaPublicOperationLimits;
use tls13_certificate_verify_input::{CertificateVerifyRole, MAXIMUM_INPUT_LENGTH};
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_certificate_verify_transcript_binding::{
    bind_certificate_verify_transcript, hash_transcript_bound_certificate_verify_input,
    CertificateVerifyFinishedContinuityErrorKind,
};
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, SelectedCipherSuite,
};
use tls13_finished::{derive_finished_key, FinishedKeyStorage, FinishedRole};
use tls13_handshake::{encode_tls13_handshake, Tls13HandshakePolicy, Tls13HandshakeType};
use tls13_reviewed_server_path_certificate_verify::{
    verify_https_dns_destination_reviewed_server_path_anchor_self_signature_certificate_verify,
    verify_https_dns_destination_reviewed_server_path_certificate_verify,
    verify_reviewed_server_path_certificate_verify,
    verify_reviewed_server_path_certificate_verify_finished_key_transcript_continuity,
    ReviewedServerPathCertificateVerifyErrorKind,
    ReviewedServerPathFinishedKeyTranscriptContinuityErrorKind,
};
use tls13_rsa_pss_signature_routing::Tls13RsaPssSignatureLimits;
use tls13_server_name::TlsServerName;
use tls13_signature_scheme_policy::{
    AuthorizedSignatureScheme, SignatureScheme, SignatureSchemePolicy,
};
use tls13_transcript::{PendingTls13Transcript, SelectedTls13Transcript, TranscriptPolicy};
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
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile, X509CertificatePurposePolicy,
    X509CertificateRole,
};
use x509_intermediate_certificate_policy::validate_x509_intermediate_certificate_policy;
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
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy};
use x509_rsa_reviewed_anchor_self_signature_binding::bind_x509_rsa_reviewed_anchor_self_signature;
use x509_rsa_reviewed_server_path_anchor_self_signature_binding::bind_x509_rsa_reviewed_server_path_anchor_self_signature;
use x509_rsa_signature_routing::{
    verify_x509_rsa_policy_signature, X509RsaSignatureRoute, X509RsaSignatureVerifier,
};
use x509_server_certificate_policy::validate_x509_server_certificate_policy;
use x509_time::{decode_x509_time, X509Time};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const LEAF_HEX: &str = include_str!("fixtures/apple_www_2026_11_11_leaf.der.hex");
const INTERMEDIATE_HEX: &str = include_str!(
    "../../x509-ocsp-reviewed-server-revocation-binding/tests/fixtures/apple-ev-intermediate.der.hex"
);
const ROOT_HEX: &str = include_str!(
    "../../x509-ocsp-reviewed-server-revocation-binding/tests/fixtures/digicert-global-root-g2.der.hex"
);
const TRANSCRIPT_HEX: &str =
    include_str!("fixtures/apple_www_2026_11_18_transcript_through_certificate.hex");
const SIGNATURE_HEX: &str =
    include_str!("fixtures/apple_www_2026_11_18_certificate_verify_signature.hex");

const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const SERVER_AUTH_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 3, 1];
const COUNTRY_NAME_OID: &[u64] = &[2, 5, 4, 6];
const ORGANIZATION_NAME_OID: &[u64] = &[2, 5, 4, 10];
const ORGANIZATIONAL_UNIT_NAME_OID: &[u64] = &[2, 5, 4, 11];
const COMMON_NAME_OID: &[u64] = &[2, 5, 4, 3];
const NULL_DER: &[u8] = &[0x05, 0x00];
const ALLOWED_EXPONENTS: &[u64] = &[65_537];

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
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn decode(input: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(input, Asn1Limits::default()).unwrap()
}

fn observed_time() -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(b"\x17\x0d261118000000Z").unwrap()).unwrap()
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

fn selected_suite() -> SelectedCipherSuite {
    let entry = CipherSuitePolicyEntry::new(CipherSuite::TlsAes256GcmSha384, 8_192).unwrap();
    CipherSuitePolicy::new(&[entry])
        .unwrap()
        .select(CipherSuite::TlsAes256GcmSha384.wire_id())
        .unwrap()
}

fn selected_transcript(encoded: &[u8]) -> SelectedTls13Transcript {
    let mut pending = PendingTls13Transcript::new(TranscriptPolicy::new(8_192).unwrap());
    pending.update(encoded).unwrap();
    pending.select(selected_suite())
}

fn authorized_scheme() -> AuthorizedSignatureScheme {
    SignatureSchemePolicy::new(&[SignatureScheme::RsaPssRsaeSha256])
        .unwrap()
        .select(SignatureScheme::RsaPssRsaeSha256.wire_id())
        .unwrap()
}

macro_rules! reviewed_server_path {
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
            |certificate| ptr::eq(certificate, &$candidates[1]),
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

fn exercise_public_composition(role: CertificateVerifyRole, expect_success: bool) {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    reviewed_server_path!(reviewed_path, &leaf, &candidates);

    let transcript_encoded = fixture(TRANSCRIPT_HEX);
    let transcript = selected_transcript(&transcript_encoded);
    let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
    let signature = fixture(SIGNATURE_HEX);
    let mut scratch = [0xa5; 223];
    let before_scratch = scratch;
    let expected_input;
    {
        let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
        let hashed = hash_transcript_bound_certificate_verify_input(authorized_scheme(), &bound);
        let message =
            CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), &signature)
                .unwrap();
        expected_input = bound.input().as_bytes().to_vec();
        let result = verify_reviewed_server_path_certificate_verify(
            &reviewed_path,
            &hashed,
            &message,
            RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            &mut scratch,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        );

        if expect_success {
            let verified =
                result.expect("captured server signature must verify with reviewed leaf");
            let expected_key = validate_rsa_public_key(
                leaf.tbs_certificate().subject_public_key_info(),
                RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                Asn1Limits::default(),
            )
            .unwrap();
            assert!(ptr::eq(verified.reviewed_server_path(), &reviewed_path));
            assert!(ptr::eq(verified.verification().hashed_input(), &hashed));
            assert!(ptr::eq(
                verified.verification().verification().signature().message(),
                &message,
            ));
            assert_eq!(
                verified
                    .verification()
                    .verification()
                    .signature()
                    .public_key()
                    .modulus(),
                expected_key.modulus(),
            );
        } else {
            assert_eq!(
                result.unwrap_err().kind(),
                ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
            );
        }
    }
    assert_eq!(&output[..expected_input.len()], expected_input);
    assert!(output[expected_input.len()..]
        .iter()
        .all(|byte| *byte == 0xa5));
    if !expect_success {
        assert_eq!(scratch, before_scratch);
    }
}

#[test]
fn public_composition_verifies_the_captured_signature_with_the_exact_reviewed_leaf() {
    exercise_public_composition(CertificateVerifyRole::Server, true);
}

#[test]
fn public_composition_rejects_client_role_before_output_or_scratch_mutation() {
    exercise_public_composition(CertificateVerifyRole::Client, false);
}

fn certificate_verify_body(signature: &[u8]) -> Vec<u8> {
    let message =
        CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), signature)
            .unwrap();
    let mut body = vec![0; message.encoded_length()];
    message.encode(&mut body).unwrap();
    body
}

fn server_finished_verify_data(
    transcript_encoded: &[u8],
    certificate_verify_body: &[u8],
    key: &tls13_finished::FinishedKey<'_>,
) -> Vec<u8> {
    let mut transcript = selected_transcript(transcript_encoded);
    let policy = Tls13HandshakePolicy::new(
        &[Tls13HandshakeType::CertificateVerify],
        certificate_verify_body.len(),
    )
    .unwrap();
    let mut encoding = vec![0; certificate_verify_body.len() + 4];
    let encoded_length = encode_tls13_handshake(
        &policy,
        Tls13HandshakeType::CertificateVerify,
        certificate_verify_body,
        &mut encoding,
    )
    .unwrap();
    transcript.update(&encoding[..encoded_length]).unwrap();
    let snapshot = transcript.snapshot();
    let mut verify_data = vec![0; snapshot.as_bytes().len()];
    key.write_verify_data(snapshot.as_bytes(), &mut verify_data)
        .unwrap();
    verify_data
}

#[test]
fn reviewed_path_finished_key_continuity_retains_path_key_and_exact_snapshot() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    reviewed_server_path!(reviewed_path, &leaf, &candidates);

    let transcript_encoded = fixture(TRANSCRIPT_HEX);
    let signature = fixture(SIGNATURE_HEX);
    let certificate_verify_body = certificate_verify_body(&signature);
    let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &[0x11; 48]).unwrap();
    let mut finished_storage = FinishedKeyStorage::new(selected_suite(), FinishedRole::Server);
    let finished_key = derive_finished_key(&prk, &mut finished_storage).unwrap();
    let verify_data =
        server_finished_verify_data(&transcript_encoded, &certificate_verify_body, &finished_key);
    let mut signed_input = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
    let mut handshake_output = vec![0xa5; certificate_verify_body.len() + 8];
    let mut scratch = [0xa5; 223];

    let verified =
        verify_reviewed_server_path_certificate_verify_finished_key_transcript_continuity(
            &reviewed_path,
            selected_transcript(&transcript_encoded),
            authorized_scheme(),
            &certificate_verify_body,
            RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            &mut signed_input,
            &mut handshake_output,
            &mut scratch,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
            &finished_key,
            &verify_data,
        )
        .expect("reviewed leaf signature and exact following Finished must verify");

    let expected_key = validate_rsa_public_key(
        leaf.tbs_certificate().subject_public_key_info(),
        RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
        Asn1Limits::default(),
    )
    .unwrap();
    assert!(ptr::eq(verified.reviewed_server_path(), &reviewed_path));
    assert_eq!(
        verified.continuity().certificate_verify_body(),
        certificate_verify_body,
    );
    assert_eq!(verified.continuity().role(), CertificateVerifyRole::Server);
    assert_eq!(
        verified.continuity().public_key().modulus(),
        expected_key.modulus(),
    );
    assert_eq!(
        verified.continuity().after_certificate_verify_snapshot(),
        verified.continuity().finished().snapshot(),
    );
    assert_eq!(
        verified.continuity().finished().transcript().snapshot(),
        verified.continuity().finished().snapshot(),
    );
    assert_eq!(
        format!("{verified:?}"),
        "VerifiedReviewedServerPathCertificateVerifyFinishedKeyTranscriptContinuity",
    );
}

#[test]
fn reviewed_path_finished_key_continuity_rejects_invalid_verify_data_without_state() {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    reviewed_server_path!(reviewed_path, &leaf, &candidates);

    let transcript_encoded = fixture(TRANSCRIPT_HEX);
    let signature = fixture(SIGNATURE_HEX);
    let certificate_verify_body = certificate_verify_body(&signature);
    let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &[0x11; 48]).unwrap();
    let mut finished_storage = FinishedKeyStorage::new(selected_suite(), FinishedRole::Server);
    let finished_key = derive_finished_key(&prk, &mut finished_storage).unwrap();
    let mut verify_data =
        server_finished_verify_data(&transcript_encoded, &certificate_verify_body, &finished_key);
    verify_data[0] ^= 0x80;
    let mut signed_input = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
    let mut handshake_output = vec![0xa5; certificate_verify_body.len() + 8];
    let mut scratch = [0xa5; 223];

    let error = verify_reviewed_server_path_certificate_verify_finished_key_transcript_continuity(
        &reviewed_path,
        selected_transcript(&transcript_encoded),
        authorized_scheme(),
        &certificate_verify_body,
        RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
        Asn1Limits::default(),
        &mut signed_input,
        &mut handshake_output,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        &finished_key,
        &verify_data,
    )
    .expect_err("invalid Finished must consume and withhold transcript state");

    assert!(matches!(
        error.kind(),
        ReviewedServerPathFinishedKeyTranscriptContinuityErrorKind::Finished(
            CertificateVerifyFinishedContinuityErrorKind::Finished(_)
        )
    ));
    let display = error.to_string();
    let debug = format!("{error:?}");
    for secret in ["www.apple.com", "signature", "modulus"] {
        assert!(!display.contains(secret));
        assert!(!debug.contains(secret));
    }
}

fn exercise_https_destination_composition(role: CertificateVerifyRole, expect_success: bool) {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    reviewed_server_path!(reviewed_path, &leaf, &candidates);

    let server_name = TlsServerName::new("www.apple.com").unwrap();
    let host = validate_http1_request_host_authority(
        b"WWW.APPLE.COM",
        Some(443),
        Http1RequestHostAuthorityLimits::new(253, 259).unwrap(),
    )
    .unwrap();
    let destination =
        bind_https_dns_destination(&leaf, server_name, host, Asn1Limits::default()).unwrap();
    let destination_path =
        bind_https_dns_destination_to_reviewed_server_path(destination, reviewed_path).unwrap();

    let transcript_encoded = fixture(TRANSCRIPT_HEX);
    let transcript = selected_transcript(&transcript_encoded);
    let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
    let signature = fixture(SIGNATURE_HEX);
    let mut scratch = [0xa5; 223];
    let before_scratch = scratch;
    let expected_input;
    {
        let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
        let hashed = hash_transcript_bound_certificate_verify_input(authorized_scheme(), &bound);
        let message =
            CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), &signature)
                .unwrap();
        expected_input = bound.input().as_bytes().to_vec();
        let result = verify_https_dns_destination_reviewed_server_path_certificate_verify(
            &destination_path,
            &hashed,
            &message,
            RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            &mut scratch,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        );

        if expect_success {
            let verified = result
                .expect("captured server signature must verify through exact HTTPS destination");
            assert!(ptr::eq(
                verified.https_destination_reviewed_server_path(),
                &destination_path,
            ));
            assert!(ptr::eq(
                verified.verification().reviewed_server_path(),
                destination_path.reviewed_server_path(),
            ));
            assert!(ptr::eq(
                verified.verification().verification().hashed_input(),
                &hashed,
            ));
            assert!(ptr::eq(
                verified
                    .verification()
                    .verification()
                    .verification()
                    .signature()
                    .message(),
                &message,
            ));
            assert!(ptr::eq(destination_path.destination().certificate(), &leaf,));
            assert_eq!(
                destination_path.destination().server_name().as_str(),
                "www.apple.com",
            );
            assert_eq!(
                destination_path.destination().host_authority().host(),
                b"WWW.APPLE.COM",
            );
            assert_eq!(
                destination_path.destination().host_authority().port(),
                Some(443),
            );
            assert_eq!(
                format!("{verified:?}"),
                "VerifiedHttpsDnsDestinationReviewedServerPathCertificateVerify",
            );
        } else {
            assert_eq!(
                result.unwrap_err().kind(),
                ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
            );
        }
    }
    assert_eq!(&output[..expected_input.len()], expected_input);
    assert!(output[expected_input.len()..]
        .iter()
        .all(|byte| *byte == 0xa5));
    if !expect_success {
        assert_eq!(scratch, before_scratch);
    }
}

#[test]
fn https_destination_composition_retains_the_exact_destination_path_and_signature() {
    exercise_https_destination_composition(CertificateVerifyRole::Server, true);
}

#[test]
fn https_destination_composition_rejects_client_role_before_buffer_mutation() {
    exercise_https_destination_composition(CertificateVerifyRole::Client, false);
}

fn exercise_https_destination_anchor_composition(
    role: CertificateVerifyRole,
    expect_success: bool,
) {
    let leaf_encoded = fixture(LEAF_HEX);
    let intermediate_encoded = fixture(INTERMEDIATE_HEX);
    let root_encoded = fixture(ROOT_HEX);
    let leaf = decode(&leaf_encoded);
    let candidates = vec![decode(&intermediate_encoded), decode(&root_encoded)];
    reviewed_server_path!(reviewed_path, &leaf, &candidates);

    let profiles = algorithm_profiles();
    let verified_self_signature = verify_x509_rsa_policy_signature(
        algorithm_policy(&candidates[1], &profiles),
        rsa_policy(&candidates[1], &profiles),
        &[route()],
        operation_limits(),
    )
    .unwrap();
    let anchor_self_signature = bind_x509_rsa_reviewed_anchor_self_signature(
        verified_self_signature,
        select_reviewed_webpki_root_anchor(&candidates[1]).unwrap(),
    )
    .unwrap();
    let reviewed_path_anchor = bind_x509_rsa_reviewed_server_path_anchor_self_signature(
        reviewed_path,
        anchor_self_signature,
    )
    .unwrap();

    let server_name = TlsServerName::new("www.apple.com").unwrap();
    let host = validate_http1_request_host_authority(
        b"WWW.APPLE.COM",
        Some(443),
        Http1RequestHostAuthorityLimits::new(253, 259).unwrap(),
    )
    .unwrap();
    let destination =
        bind_https_dns_destination(&leaf, server_name, host, Asn1Limits::default()).unwrap();
    let destination_path_anchor =
        bind_https_dns_destination_to_reviewed_server_path_anchor_self_signature(
            destination,
            reviewed_path_anchor,
        )
        .unwrap();

    let transcript_encoded = fixture(TRANSCRIPT_HEX);
    let transcript = selected_transcript(&transcript_encoded);
    let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
    let signature = fixture(SIGNATURE_HEX);
    let mut scratch = [0xa5; 223];
    let before_scratch = scratch;
    let expected_input;
    {
        let bound = bind_certificate_verify_transcript(&transcript, role, &mut output).unwrap();
        let hashed = hash_transcript_bound_certificate_verify_input(authorized_scheme(), &bound);
        let message =
            CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), &signature)
                .unwrap();
        expected_input = bound.input().as_bytes().to_vec();
        let result =
            verify_https_dns_destination_reviewed_server_path_anchor_self_signature_certificate_verify(
                &destination_path_anchor,
                &hashed,
                &message,
                RsaPublicKeyPolicy::new(2_048, 2_048, ALLOWED_EXPONENTS).unwrap(),
                Asn1Limits::default(),
                &mut scratch,
                Tls13RsaPssSignatureLimits::Sha256(
                    RsaPssSha256SignatureLimits::new(300, 300).unwrap(),
                ),
            );

        if expect_success {
            let verified = result.expect(
                "captured server signature must verify through destination and anchor proof",
            );
            assert!(ptr::eq(
                verified.https_destination_reviewed_server_path_anchor_self_signature(),
                &destination_path_anchor,
            ));
            assert!(ptr::eq(
                verified.verification().reviewed_server_path(),
                destination_path_anchor
                    .reviewed_server_path_anchor_self_signature()
                    .reviewed_server_path(),
            ));
            assert!(ptr::eq(
                verified.verification().verification().hashed_input(),
                &hashed,
            ));
            assert!(ptr::eq(
                verified
                    .verification()
                    .verification()
                    .verification()
                    .signature()
                    .message(),
                &message,
            ));
            assert!(ptr::eq(
                destination_path_anchor.destination().certificate(),
                &leaf,
            ));
            assert!(ptr::eq(
                destination_path_anchor
                    .reviewed_server_path_anchor_self_signature()
                    .anchor_self_signature()
                    .reviewed_anchor()
                    .candidate_certificate(),
                &candidates[1],
            ));
            assert_eq!(
                destination_path_anchor.destination().server_name().as_str(),
                "www.apple.com",
            );
            assert_eq!(
                destination_path_anchor
                    .destination()
                    .host_authority()
                    .host(),
                b"WWW.APPLE.COM",
            );
            assert_eq!(
                destination_path_anchor
                    .destination()
                    .host_authority()
                    .port(),
                Some(443),
            );
            assert_eq!(
                format!("{verified:?}"),
                "VerifiedHttpsDnsDestinationReviewedServerPathAnchorSelfSignatureCertificateVerify",
            );
        } else {
            assert_eq!(
                result.unwrap_err().kind(),
                ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
            );
        }
    }
    assert_eq!(&output[..expected_input.len()], expected_input);
    assert!(output[expected_input.len()..]
        .iter()
        .all(|byte| *byte == 0xa5));
    if !expect_success {
        assert_eq!(scratch, before_scratch);
    }
}

#[test]
fn anchor_composition_retains_destination_path_anchor_and_signature() {
    exercise_https_destination_anchor_composition(CertificateVerifyRole::Server, true);
}

#[test]
fn anchor_composition_rejects_client_role_before_buffer_mutation() {
    exercise_https_destination_anchor_composition(CertificateVerifyRole::Client, false);
}
