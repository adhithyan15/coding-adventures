use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use sha256::sha256;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_policy::{
    validate_certificate_algorithm_policy, validate_certificate_role_policy,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile, X509CertificateRole,
    X509CertificateValidityErrorKind,
};
use x509_ocsp_authenticated_responder::{
    authenticate_x509_ocsp_rsa_sha256_responder, AuthenticatedX509OcspRsaSha256Responder,
    X509OcspRsaSha256ResponderPolicy,
};
use x509_ocsp_authenticated_target_revocation::{
    authenticate_x509_ocsp_rsa_sha256_target_revocation,
    X509OcspAuthenticatedTargetRevocationErrorKind,
};
use x509_ocsp_basic_response::{decode_x509_ocsp_basic_response, X509OcspBasicResponse};
use x509_ocsp_cert_id_binding::{bind_x509_ocsp_cert_id_to_certificates, X509OcspCertIdHashInput};
use x509_ocsp_cert_status::X509OcspCertStatus;
use x509_ocsp_issuer_bound_responder::bind_x509_ocsp_authenticated_responder_to_issuer;
use x509_ocsp_responder_certificate_policy::validate_x509_ocsp_responder_certificate_policy;
use x509_ocsp_responder_id_binding::bind_x509_ocsp_responder_id_to_certificate;
use x509_ocsp_revocation_policy::validate_bound_x509_ocsp_revocation;
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_sha256_issuer_link::{
    verify_x509_rsa_sha256_issuer_link, VerifiedX509RsaSha256IssuerLink,
    X509RsaSha256IssuerLinkPolicy,
};
use x509_time::{decode_x509_time, X509Time};

const SHA256_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const NULL_DER: &[u8] = &[0x05, 0x00];
const ALLOWED_EXPONENTS: &[u64] = &[65_537];
const NAME_HASH: [u8; 20] = [0x11; 20];
const KEY_HASH: [u8; 20] = [0x22; 20];

const ISSUER_CERTIFICATE_SHA256: &str =
    "e753e3a04ff526e93dba801c21d224f2fdae2966b5883f0b67aad79b063c7341";
const RESPONDER_CERTIFICATE_SHA256: &str =
    "623765352cc5bd58e04a113a31a16af13f468d5871eee7eb96a71adaab58aa77";
const TARGET_CERTIFICATE_SHA256: &str =
    "fef3439bddea088a83e89e6c28c8a44c103a003e972bb331c951b6d01e43e6f4";
const RESPONSE_DATA_SHA256: &str =
    "ab87cbe247226d66a02f9983ffa1949e7439cac16416862ca383ed8a191c7411";
const RESPONSE_SIGNATURE_SHA256: &str =
    "5499394a8e612423cd9b94b98b3f053f7488d3ff860b41bc25d7b73d4417717d";
const EXTENDED_RESPONSE_DATA_SHA256: &str =
    "2f3f969bfb17561460c6f595367300729ee54ad93508a6f8b8d9bf98c5fbf45a";
const EXTENDED_RESPONSE_SIGNATURE_SHA256: &str =
    "e4c137e5bc1d8e14cc681c7e220573c3c2e4665a5c4639f6ec3f0e0afc0717fa";
const CONFLICTING_RESPONSE_DATA_SHA256: &str =
    "73ccb85d8dc14d705aa5941d68f6921d8aa1561117d2567290932c696b987187";
const CONFLICTING_RESPONSE_SIGNATURE_SHA256: &str =
    "91afb47c4402eda1242793733fceb132c5d6507c81aa780c7f915ce3b4aa3243";
const EXPIRED_TIME_RESPONSE_DATA_SHA256: &str =
    "1fc2b7bb4699113706f2cd3df4450255ee92b41c2ad5f3f2d7de41a7ec80e6ef";
const EXPIRED_TIME_RESPONSE_SIGNATURE_SHA256: &str =
    "432e046c88355db731a6b05ed18d861eaa5ac2b0c5db97a323caff56896eb5d1";

fn decode_hex(hexadecimal: &str) -> Vec<u8> {
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

fn der_length(length: usize) -> Vec<u8> {
    if length < 128 {
        return vec![length as u8];
    }
    let bytes = length.to_be_bytes();
    let first = bytes.iter().position(|byte| *byte != 0).unwrap();
    let significant = &bytes[first..];
    let mut encoded = vec![0x80 | significant.len() as u8];
    encoded.extend_from_slice(significant);
    encoded
}

fn element(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = vec![tag];
    encoded.extend_from_slice(&der_length(value.len()));
    encoded.extend_from_slice(value);
    encoded
}

fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    element(0x30, &fields.iter().flatten().copied().collect::<Vec<_>>())
}

fn algorithm(oid: &[u8]) -> Vec<u8> {
    sequence(&[element(0x06, oid), NULL_DER.to_vec()])
}

fn response_with_signature(response_data: &str, signature: &str) -> Vec<u8> {
    let mut signature_bits = vec![0];
    signature_bits.extend_from_slice(&decode_hex(signature));
    sequence(&[
        decode_hex(response_data),
        algorithm(SHA256_WITH_RSA_ENCRYPTION_OID_DER),
        element(0x03, &signature_bits),
    ])
}

fn decode_response(encoded: &[u8]) -> X509OcspBasicResponse<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    decode_x509_ocsp_basic_response(&mut decoder, root).unwrap()
}

fn observed_time() -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(b"\x17\x0d261026000000Z").unwrap()).unwrap()
}

fn observation_after_responder_expiry() -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(b"\x17\x0d371026000000Z").unwrap()).unwrap()
}

fn algorithm_profiles() -> [X509CertificateAlgorithmProfile<'static>; 1] {
    [X509CertificateAlgorithmProfile::new(
        SHA256_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        4_096,
        2_048,
        4_096,
    )
    .unwrap()]
}

fn authenticate<'response, 'response_encoded, 'responder, 'responder_encoded>(
    response: &'response X509OcspBasicResponse<'response_encoded>,
    responder: &'responder X509Certificate<'responder_encoded>,
) -> AuthenticatedX509OcspRsaSha256Responder<
    'response,
    'response_encoded,
    'responder,
    'responder_encoded,
> {
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
            oid.equals(&[2, 5, 29, 19])
                || oid.equals(&[2, 5, 29, 15])
                || oid.equals(&[2, 5, 29, 37])
        },
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(responder, &algorithm_profiles()).unwrap();
    let certificate_policy = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &observed_time(),
    )
    .unwrap();
    authenticate_x509_ocsp_rsa_sha256_responder(
        responder_id,
        certificate_policy,
        X509OcspRsaSha256ResponderPolicy::new(
            RsaPublicKeyPolicy::new(2_048, 4_096, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            RsaPublicOperationLimits::new(512).unwrap(),
        ),
    )
    .unwrap()
}

fn issuer_link<'child, 'child_encoded, 'issuer, 'issuer_encoded>(
    child: &'child X509Certificate<'child_encoded>,
    issuer: &'issuer X509Certificate<'issuer_encoded>,
) -> VerifiedX509RsaSha256IssuerLink<'child, 'child_encoded, 'issuer, 'issuer_encoded> {
    verify_x509_rsa_sha256_issuer_link(
        child,
        issuer,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        X509RsaSha256IssuerLinkPolicy::new(
            Asn1Limits::default(),
            RsaPublicKeyPolicy::new(2_048, 4_096, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            RsaPublicOperationLimits::new(512).unwrap(),
        ),
    )
    .unwrap()
}

#[test]
fn checked_in_public_fixtures_match_the_generation_trace() {
    for (fixture, expected) in [
        (
            include_str!("fixtures/issuer.der.hex"),
            ISSUER_CERTIFICATE_SHA256,
        ),
        (
            include_str!("fixtures/responder.der.hex"),
            RESPONDER_CERTIFICATE_SHA256,
        ),
        (
            include_str!("fixtures/target.der.hex"),
            TARGET_CERTIFICATE_SHA256,
        ),
        (
            include_str!("fixtures/response-data.der.hex"),
            RESPONSE_DATA_SHA256,
        ),
        (
            include_str!("fixtures/response-signature.hex"),
            RESPONSE_SIGNATURE_SHA256,
        ),
        (
            include_str!("fixtures/response-data-with-extensions.der.hex"),
            EXTENDED_RESPONSE_DATA_SHA256,
        ),
        (
            include_str!("fixtures/response-signature-with-extensions.hex"),
            EXTENDED_RESPONSE_SIGNATURE_SHA256,
        ),
        (
            include_str!("fixtures/response-data-with-conflict.der.hex"),
            CONFLICTING_RESPONSE_DATA_SHA256,
        ),
        (
            include_str!("fixtures/response-signature-with-conflict.hex"),
            CONFLICTING_RESPONSE_SIGNATURE_SHA256,
        ),
        (
            include_str!("fixtures/response-data-after-responder-expiry.der.hex"),
            EXPIRED_TIME_RESPONSE_DATA_SHA256,
        ),
        (
            include_str!("fixtures/response-signature-after-responder-expiry.hex"),
            EXPIRED_TIME_RESPONSE_SIGNATURE_SHA256,
        ),
    ] {
        assert_eq!(
            sha256(&decode_hex(fixture)).as_slice(),
            decode_hex(expected).as_slice()
        );
    }
}

#[test]
fn authenticates_one_exact_target_revocation_result() {
    let issuer_encoded = decode_hex(include_str!("fixtures/issuer.der.hex"));
    let responder_encoded = decode_hex(include_str!("fixtures/responder.der.hex"));
    let target_encoded = decode_hex(include_str!("fixtures/target.der.hex"));
    let response_encoded = response_with_signature(
        include_str!("fixtures/response-data.der.hex"),
        include_str!("fixtures/response-signature.hex"),
    );
    let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
    let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
    let target = decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let response = decode_response(&response_encoded);
    let issuer_bound = bind_x509_ocsp_authenticated_responder_to_issuer(
        authenticate(&response, &responder),
        issuer_link(&responder, &issuer),
    )
    .unwrap();
    let binding = bind_x509_ocsp_cert_id_to_certificates(
        response.response_data().get(0).unwrap(),
        &target,
        &issuer,
        |_, input| match input {
            X509OcspCertIdHashInput::TargetIssuerNameDer(_) => Some(NAME_HASH),
            X509OcspCertIdHashInput::IssuerSubjectPublicKey(_) => Some(KEY_HASH),
        },
    )
    .unwrap();
    let revocation = validate_bound_x509_ocsp_revocation(&binding, observed_time()).unwrap();

    let authenticated = authenticate_x509_ocsp_rsa_sha256_target_revocation(
        issuer_bound,
        revocation,
        issuer_link(&target, &issuer),
    )
    .unwrap();

    assert!(std::ptr::eq(
        authenticated
            .issuer_bound_responder()
            .authenticated_responder()
            .responder_id_binding()
            .response(),
        &response
    ));
    assert!(std::ptr::eq(
        authenticated.revocation().cert_id_binding(),
        &binding
    ));
    assert!(std::ptr::eq(
        authenticated.target_issuer_link().child(),
        &target
    ));
    assert!(std::ptr::eq(
        authenticated.target_issuer_link().candidate_issuer(),
        &issuer
    ));
    let _ = authenticated.responder_validity_at_observation();
    assert_eq!(
        format!("{authenticated:?}"),
        "AuthenticatedX509OcspRsaSha256TargetRevocation"
    );
}

#[test]
fn rejects_a_revocation_result_from_a_byte_equal_distinct_response() {
    let issuer_encoded = decode_hex(include_str!("fixtures/issuer.der.hex"));
    let responder_encoded = decode_hex(include_str!("fixtures/responder.der.hex"));
    let target_encoded = decode_hex(include_str!("fixtures/target.der.hex"));
    let response_encoded = response_with_signature(
        include_str!("fixtures/response-data.der.hex"),
        include_str!("fixtures/response-signature.hex"),
    );
    let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
    let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
    let target = decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let response = decode_response(&response_encoded);
    let distinct_response = decode_response(&response_encoded);
    let issuer_bound = bind_x509_ocsp_authenticated_responder_to_issuer(
        authenticate(&response, &responder),
        issuer_link(&responder, &issuer),
    )
    .unwrap();
    let binding = bind_x509_ocsp_cert_id_to_certificates(
        distinct_response.response_data().get(0).unwrap(),
        &target,
        &issuer,
        |_, input| match input {
            X509OcspCertIdHashInput::TargetIssuerNameDer(_) => Some(NAME_HASH),
            X509OcspCertIdHashInput::IssuerSubjectPublicKey(_) => Some(KEY_HASH),
        },
    )
    .unwrap();
    let revocation = validate_bound_x509_ocsp_revocation(&binding, observed_time()).unwrap();

    let error = authenticate_x509_ocsp_rsa_sha256_target_revocation(
        issuer_bound,
        revocation,
        issuer_link(&target, &issuer),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspAuthenticatedTargetRevocationErrorKind::ResponseMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP authenticated target revocation error ResponseMismatch"
    );
}

#[test]
fn rejects_a_byte_equal_distinct_target() {
    let issuer_encoded = decode_hex(include_str!("fixtures/issuer.der.hex"));
    let responder_encoded = decode_hex(include_str!("fixtures/responder.der.hex"));
    let target_encoded = decode_hex(include_str!("fixtures/target.der.hex"));
    let response_encoded = response_with_signature(
        include_str!("fixtures/response-data.der.hex"),
        include_str!("fixtures/response-signature.hex"),
    );
    let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
    let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
    let target = decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let distinct_target = decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let response = decode_response(&response_encoded);
    let issuer_bound = bind_x509_ocsp_authenticated_responder_to_issuer(
        authenticate(&response, &responder),
        issuer_link(&responder, &issuer),
    )
    .unwrap();
    let binding = bind_x509_ocsp_cert_id_to_certificates(
        response.response_data().get(0).unwrap(),
        &distinct_target,
        &issuer,
        |_, input| match input {
            X509OcspCertIdHashInput::TargetIssuerNameDer(_) => Some(NAME_HASH),
            X509OcspCertIdHashInput::IssuerSubjectPublicKey(_) => Some(KEY_HASH),
        },
    )
    .unwrap();
    let revocation = validate_bound_x509_ocsp_revocation(&binding, observed_time()).unwrap();

    let error = authenticate_x509_ocsp_rsa_sha256_target_revocation(
        issuer_bound,
        revocation,
        issuer_link(&target, &issuer),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspAuthenticatedTargetRevocationErrorKind::TargetMismatch
    );
}

#[test]
fn rejects_a_byte_equal_distinct_issuer() {
    let issuer_encoded = decode_hex(include_str!("fixtures/issuer.der.hex"));
    let responder_encoded = decode_hex(include_str!("fixtures/responder.der.hex"));
    let target_encoded = decode_hex(include_str!("fixtures/target.der.hex"));
    let response_encoded = response_with_signature(
        include_str!("fixtures/response-data.der.hex"),
        include_str!("fixtures/response-signature.hex"),
    );
    let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
    let distinct_issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
    let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
    let target = decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let response = decode_response(&response_encoded);
    let issuer_bound = bind_x509_ocsp_authenticated_responder_to_issuer(
        authenticate(&response, &responder),
        issuer_link(&responder, &distinct_issuer),
    )
    .unwrap();
    let binding = bind_x509_ocsp_cert_id_to_certificates(
        response.response_data().get(0).unwrap(),
        &target,
        &issuer,
        |_, input| match input {
            X509OcspCertIdHashInput::TargetIssuerNameDer(_) => Some(NAME_HASH),
            X509OcspCertIdHashInput::IssuerSubjectPublicKey(_) => Some(KEY_HASH),
        },
    )
    .unwrap();
    let revocation = validate_bound_x509_ocsp_revocation(&binding, observed_time()).unwrap();

    let error = authenticate_x509_ocsp_rsa_sha256_target_revocation(
        issuer_bound,
        revocation,
        issuer_link(&target, &issuer),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspAuthenticatedTargetRevocationErrorKind::IssuerMismatch
    );
}

#[test]
fn rejects_outer_response_extensions_after_authenticating_the_response() {
    let issuer_encoded = decode_hex(include_str!("fixtures/issuer.der.hex"));
    let responder_encoded = decode_hex(include_str!("fixtures/responder.der.hex"));
    let target_encoded = decode_hex(include_str!("fixtures/target.der.hex"));
    let response_encoded = response_with_signature(
        include_str!("fixtures/response-data-with-extensions.der.hex"),
        include_str!("fixtures/response-signature-with-extensions.hex"),
    );
    let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
    let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
    let target = decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let response = decode_response(&response_encoded);
    let issuer_bound = bind_x509_ocsp_authenticated_responder_to_issuer(
        authenticate(&response, &responder),
        issuer_link(&responder, &issuer),
    )
    .unwrap();
    let binding = bind_x509_ocsp_cert_id_to_certificates(
        response.response_data().get(0).unwrap(),
        &target,
        &issuer,
        |_, input| match input {
            X509OcspCertIdHashInput::TargetIssuerNameDer(_) => Some(NAME_HASH),
            X509OcspCertIdHashInput::IssuerSubjectPublicKey(_) => Some(KEY_HASH),
        },
    )
    .unwrap();
    let revocation = validate_bound_x509_ocsp_revocation(&binding, observed_time()).unwrap();

    let error = authenticate_x509_ocsp_rsa_sha256_target_revocation(
        issuer_bound,
        revocation,
        issuer_link(&target, &issuer),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspAuthenticatedTargetRevocationErrorKind::UnsupportedResponseExtensions
    );
    assert!(!error.to_string().contains("1.2.3"));
}

#[test]
fn rejects_a_signed_response_with_conflicting_duplicate_target_statuses() {
    let issuer_encoded = decode_hex(include_str!("fixtures/issuer.der.hex"));
    let responder_encoded = decode_hex(include_str!("fixtures/responder.der.hex"));
    let target_encoded = decode_hex(include_str!("fixtures/target.der.hex"));
    let response_encoded = response_with_signature(
        include_str!("fixtures/response-data-with-conflict.der.hex"),
        include_str!("fixtures/response-signature-with-conflict.hex"),
    );
    let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
    let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
    let target = decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let response = decode_response(&response_encoded);
    assert_eq!(response.response_data().len(), 2);
    assert!(matches!(
        response.response_data().get(1).unwrap().cert_status(),
        X509OcspCertStatus::Revoked(_)
    ));
    let issuer_bound = bind_x509_ocsp_authenticated_responder_to_issuer(
        authenticate(&response, &responder),
        issuer_link(&responder, &issuer),
    )
    .unwrap();
    let binding = bind_x509_ocsp_cert_id_to_certificates(
        response.response_data().get(0).unwrap(),
        &target,
        &issuer,
        |_, input| match input {
            X509OcspCertIdHashInput::TargetIssuerNameDer(_) => Some(NAME_HASH),
            X509OcspCertIdHashInput::IssuerSubjectPublicKey(_) => Some(KEY_HASH),
        },
    )
    .unwrap();
    let revocation = validate_bound_x509_ocsp_revocation(&binding, observed_time()).unwrap();

    let error = authenticate_x509_ocsp_rsa_sha256_target_revocation(
        issuer_bound,
        revocation,
        issuer_link(&target, &issuer),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspAuthenticatedTargetRevocationErrorKind::AmbiguousResponseSet
    );
}

#[test]
fn rejects_responder_validity_evidence_from_a_different_time() {
    let issuer_encoded = decode_hex(include_str!("fixtures/issuer.der.hex"));
    let responder_encoded = decode_hex(include_str!("fixtures/responder.der.hex"));
    let target_encoded = decode_hex(include_str!("fixtures/target.der.hex"));
    let response_encoded = response_with_signature(
        include_str!("fixtures/response-data-after-responder-expiry.der.hex"),
        include_str!("fixtures/response-signature-after-responder-expiry.hex"),
    );
    let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
    let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
    let target = decode_x509_certificate(&target_encoded, Asn1Limits::default()).unwrap();
    let response = decode_response(&response_encoded);
    let issuer_bound = bind_x509_ocsp_authenticated_responder_to_issuer(
        // This authentication intentionally carries responder policy checked
        // in 2026, while the revocation observation below is after expiry.
        authenticate(&response, &responder),
        issuer_link(&responder, &issuer),
    )
    .unwrap();
    let binding = bind_x509_ocsp_cert_id_to_certificates(
        response.response_data().get(0).unwrap(),
        &target,
        &issuer,
        |_, input| match input {
            X509OcspCertIdHashInput::TargetIssuerNameDer(_) => Some(NAME_HASH),
            X509OcspCertIdHashInput::IssuerSubjectPublicKey(_) => Some(KEY_HASH),
        },
    )
    .unwrap();
    let revocation =
        validate_bound_x509_ocsp_revocation(&binding, observation_after_responder_expiry())
            .unwrap();

    let error = authenticate_x509_ocsp_rsa_sha256_target_revocation(
        issuer_bound,
        revocation,
        issuer_link(&target, &issuer),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspAuthenticatedTargetRevocationErrorKind::ResponderCertificateValidity(
            X509CertificateValidityErrorKind::Expired
        )
    );
}
