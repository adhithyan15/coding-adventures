use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_policy::{
    validate_certificate_algorithm_policy, ValidatedCertificateAlgorithmPolicy,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile,
};
use x509_rsa_algorithm_policy::{
    validate_x509_rsa_algorithm_policy, ValidatedX509RsaAlgorithmPolicy,
};
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_reviewed_anchor_self_signature_binding::{
    bind_x509_rsa_reviewed_anchor_self_signature, X509RsaReviewedAnchorSelfSignatureBindingError,
};
use x509_rsa_signature_routing::{
    verify_x509_rsa_policy_signature, VerifiedX509RsaPolicySignature, X509RsaSignatureRoute,
    X509RsaSignatureVerifier,
};
use x509_webpki_root_anchor_selection::select_reviewed_webpki_root_anchor;

const SHA256_ROOT_HEX: &str =
    include_str!("../../x509-rsa-sha256-reviewed-anchor-link/tests/fixtures/isrg_root_x1.hex");
const SHA384_ROOT_HEX: &str =
    include_str!("../../x509-rsa-signature-routing/tests/fixtures/microsoft_rsa_root_2017.der.hex");
const SHA512_ROOT_HEX: &str = include_str!(
    "../../x509-rsa-sha512-reviewed-anchor-link/tests/fixtures/d_trust_br_root_ca_2_2023.der.hex"
);

// Public fixtures and SHA-256 fingerprints:
// - ISRG Root X1: 96bcec06264976f37460779acf28c5a7cfe8a3c0aae11a8ffcee05c0bddf08c6
// - Microsoft RSA Root Certificate Authority 2017:
//   c741f70f4b2a8d88bf2e71c14122ef53ef10eba0cfa5e64cfa20f418853073e0
// - D-TRUST BR Root CA 2 2023:
//   0552e6f83fdf65e8fa9670e666df28a4e21340b510cbe52566f97c4fb94b2bd1

const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const SHA384_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 12];
const SHA512_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 13];
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

fn profile(signature_oid: &'static [u64]) -> X509CertificateAlgorithmProfile<'static> {
    X509CertificateAlgorithmProfile::new(
        signature_oid,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        4_096,
        8_192,
        4_096,
        4_096,
    )
    .unwrap()
}

fn profiles() -> [X509CertificateAlgorithmProfile<'static>; 3] {
    [
        profile(SHA512_WITH_RSA_ENCRYPTION_OID),
        profile(SHA256_WITH_RSA_ENCRYPTION_OID),
        profile(SHA384_WITH_RSA_ENCRYPTION_OID),
    ]
}

fn child_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    profiles: &[X509CertificateAlgorithmProfile<'static>],
) -> ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded> {
    validate_certificate_algorithm_policy(certificate, profiles).unwrap()
}

fn issuer_policy<'certificate, 'encoded>(
    certificate: &'certificate X509Certificate<'encoded>,
    profiles: &[X509CertificateAlgorithmProfile<'static>],
) -> ValidatedX509RsaAlgorithmPolicy<'certificate, 'encoded> {
    let algorithm_policy = validate_certificate_algorithm_policy(certificate, profiles).unwrap();
    validate_x509_rsa_algorithm_policy(
        algorithm_policy,
        RsaPublicKeyPolicy::new(4_096, 4_096, &[65_537]).unwrap(),
        Asn1Limits::default(),
    )
    .unwrap()
}

fn verify<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>(
    child: &'child_certificate X509Certificate<'child_encoded>,
    issuer: &'issuer_certificate X509Certificate<'issuer_encoded>,
    profile_index: usize,
    verifier: X509RsaSignatureVerifier,
) -> VerifiedX509RsaPolicySignature<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    let profiles = profiles();
    verify_x509_rsa_policy_signature(
        child_policy(child, &profiles),
        issuer_policy(issuer, &profiles),
        &[X509RsaSignatureRoute::new(profile_index, verifier)],
        RsaPublicOperationLimits::new(512).unwrap(),
    )
    .unwrap()
}

#[test]
fn binds_all_routed_rsa_verifiers_to_one_exact_reviewed_anchor() {
    let cases = [
        (
            SHA256_ROOT_HEX,
            1,
            X509RsaSignatureVerifier::RsaPkcs1V15Sha256,
        ),
        (
            SHA384_ROOT_HEX,
            2,
            X509RsaSignatureVerifier::RsaPkcs1V15Sha384,
        ),
        (
            SHA512_ROOT_HEX,
            0,
            X509RsaSignatureVerifier::RsaPkcs1V15Sha512,
        ),
    ];

    for (fixture_hex, profile_index, verifier) in cases {
        let encoded = fixture(fixture_hex);
        let certificate = decode(&encoded);
        let verified = verify(&certificate, &certificate, profile_index, verifier);
        let reviewed_anchor = select_reviewed_webpki_root_anchor(&certificate).unwrap();

        let bound =
            bind_x509_rsa_reviewed_anchor_self_signature(verified, reviewed_anchor).unwrap();

        assert_eq!(bound.verified_self_signature().verifier(), verifier);
        assert!(std::ptr::eq(
            bound
                .verified_self_signature()
                .child_algorithm_policy()
                .certificate(),
            &certificate
        ));
        assert!(std::ptr::eq(
            bound
                .verified_self_signature()
                .issuer_rsa_algorithm_policy()
                .certificate(),
            &certificate
        ));
        assert!(std::ptr::eq(
            bound.reviewed_anchor().candidate_certificate(),
            &certificate
        ));
        assert_eq!(
            format!("{bound:?}"),
            "BoundX509RsaReviewedAnchorSelfSignature"
        );
    }
}

#[test]
fn rejects_a_byte_equal_distinct_signed_child() {
    let encoded = fixture(SHA256_ROOT_HEX);
    let reviewed_certificate = decode(&encoded);
    let distinct_child = decode(&encoded);
    let verified = verify(
        &distinct_child,
        &reviewed_certificate,
        1,
        X509RsaSignatureVerifier::RsaPkcs1V15Sha256,
    );
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&reviewed_certificate).unwrap();

    let error = bind_x509_rsa_reviewed_anchor_self_signature(verified, reviewed_anchor)
        .expect_err("byte-equal child substitution must fail");

    assert_eq!(
        error,
        X509RsaReviewedAnchorSelfSignatureBindingError::ChildCertificateMismatch
    );
}

#[test]
fn rejects_a_byte_equal_distinct_issuer_key_source() {
    let encoded = fixture(SHA256_ROOT_HEX);
    let reviewed_certificate = decode(&encoded);
    let distinct_issuer = decode(&encoded);
    let verified = verify(
        &reviewed_certificate,
        &distinct_issuer,
        1,
        X509RsaSignatureVerifier::RsaPkcs1V15Sha256,
    );
    let reviewed_anchor = select_reviewed_webpki_root_anchor(&reviewed_certificate).unwrap();

    let error = bind_x509_rsa_reviewed_anchor_self_signature(verified, reviewed_anchor)
        .expect_err("byte-equal issuer substitution must fail");

    assert_eq!(
        error,
        X509RsaReviewedAnchorSelfSignatureBindingError::IssuerCertificateMismatch
    );
}

#[test]
fn failures_are_stable_and_payload_free() {
    let cases = [
        (
            X509RsaReviewedAnchorSelfSignatureBindingError::ChildCertificateMismatch,
            "X.509 reviewed-anchor RSA self-signature binding failed: child mismatch",
        ),
        (
            X509RsaReviewedAnchorSelfSignatureBindingError::IssuerCertificateMismatch,
            "X.509 reviewed-anchor RSA self-signature binding failed: issuer mismatch",
        ),
    ];

    for (error, expected) in cases {
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert_eq!(display, expected);
        for payload in ["ISRG", "Microsoft", "D-TRUST", "4096", "65537"] {
            assert!(!display.contains(payload));
            assert!(!debug.contains(payload));
        }
    }
}
