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
use x509_rsa_pkcs1_v1_5_sha512_signature::X509RsaSha512SignatureErrorKind;
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_signature_routing::{
    verify_x509_rsa_policy_signature, VerifiedX509RsaPolicySignature, X509RsaSignatureRoute,
    X509RsaSignatureRoutingErrorKind, X509RsaSignatureVerifier,
};

const SHA256_ROOT_HEX: &str =
    include_str!("../../x509-rsa-sha256-reviewed-anchor-link/tests/fixtures/isrg_root_x1.hex");
const SHA384_ROOT_HEX: &str = include_str!("fixtures/microsoft_rsa_root_2017.der.hex");
const SHA512_ROOT_HEX: &str = include_str!(
    "../../x509-rsa-sha512-reviewed-anchor-link/tests/fixtures/d_trust_br_root_ca_2_2023.der.hex"
);

// Public fixtures and SHA-256 fingerprints:
// - ISRG Root X1: https://letsencrypt.org/certs/isrgrootx1.der
//   96bcec06264976f37460779acf28c5a7cfe8a3c0aae11a8ffcee05c0bddf08c6
// - Microsoft RSA Root Certificate Authority 2017:
//   https://www.microsoft.com/pkiops/certs/Microsoft%20RSA%20Root%20Certificate%20Authority%202017.crt
//   c741f70f4b2a8d88bf2e71c14122ef53ef10eba0cfa5e64cfa20f418853073e0
// - D-TRUST BR Root CA 2 2023, reviewed webpki-roots v1.0.9 snapshot:
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

#[test]
fn routes_sha256_by_the_selected_child_profile_and_retains_exact_evidence() {
    let child_encoded = fixture(SHA256_ROOT_HEX);
    let issuer_encoded = child_encoded.clone();
    let child = decode(&child_encoded);
    let issuer = decode(&issuer_encoded);
    let profiles = profiles();
    let child_policy = child_policy(&child, &profiles);
    assert_eq!(child_policy.algorithm_profile().profile_index(), 1);
    let route = X509RsaSignatureRoute::new(1, X509RsaSignatureVerifier::RsaPkcs1V15Sha256);
    assert_eq!(route.profile_index(), 1);
    assert_eq!(
        route.verifier(),
        X509RsaSignatureVerifier::RsaPkcs1V15Sha256
    );

    let verified = verify_x509_rsa_policy_signature(
        child_policy,
        issuer_policy(&issuer, &profiles),
        &[route],
        RsaPublicOperationLimits::new(512).unwrap(),
    )
    .unwrap();

    assert_eq!(
        verified.verifier(),
        X509RsaSignatureVerifier::RsaPkcs1V15Sha256
    );
    assert_eq!(
        format!("{verified:?}"),
        "VerifiedX509RsaPolicySignature::Sha256"
    );
    let VerifiedX509RsaPolicySignature::Sha256(verified) = verified else {
        panic!("selected the wrong verifier")
    };
    assert!(std::ptr::eq(
        verified.child_algorithm_policy().certificate(),
        &child
    ));
    assert!(std::ptr::eq(
        verified.issuer_rsa_algorithm_policy().certificate(),
        &issuer
    ));
    assert!(std::ptr::eq(verified.signature().certificate(), &child));
    assert_eq!(
        verified.signature().issuer_public_key(),
        verified.issuer_rsa_algorithm_policy().rsa_public_key()
    );
}

#[test]
fn routes_sha384_and_sha512_through_their_fixed_verifiers() {
    let cases = [
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
        let child_encoded = fixture(fixture_hex);
        let issuer_encoded = child_encoded.clone();
        let child = decode(&child_encoded);
        let issuer = decode(&issuer_encoded);
        let profiles = profiles();
        let verified = verify_x509_rsa_policy_signature(
            child_policy(&child, &profiles),
            issuer_policy(&issuer, &profiles),
            &[X509RsaSignatureRoute::new(profile_index, verifier)],
            RsaPublicOperationLimits::new(512).unwrap(),
        )
        .unwrap();

        assert_eq!(verified.verifier(), verifier);
        match verified {
            VerifiedX509RsaPolicySignature::Sha384(verified) => {
                assert_eq!(profile_index, 2);
                assert!(std::ptr::eq(verified.signature().certificate(), &child));
                assert_eq!(
                    verified.signature().issuer_public_key(),
                    verified.issuer_rsa_algorithm_policy().rsa_public_key()
                );
            }
            VerifiedX509RsaPolicySignature::Sha512(verified) => {
                assert_eq!(profile_index, 0);
                assert!(std::ptr::eq(verified.signature().certificate(), &child));
                assert_eq!(
                    verified.signature().issuer_public_key(),
                    verified.issuer_rsa_algorithm_policy().rsa_public_key()
                );
            }
            VerifiedX509RsaPolicySignature::Sha256(_) => panic!("selected the wrong verifier"),
        }
    }
}

#[test]
fn rejects_missing_duplicate_and_mismatched_routes_with_redacted_errors() {
    let child_encoded = fixture(SHA256_ROOT_HEX);
    let issuer_encoded = child_encoded.clone();
    let child = decode(&child_encoded);
    let issuer = decode(&issuer_encoded);
    let profiles = profiles();
    let limits = RsaPublicOperationLimits::new(512).unwrap();

    let missing = verify_x509_rsa_policy_signature(
        child_policy(&child, &profiles),
        issuer_policy(&issuer, &profiles),
        &[],
        limits,
    )
    .unwrap_err();
    assert_eq!(
        missing.kind(),
        X509RsaSignatureRoutingErrorKind::UnsupportedProfileRoute
    );

    let route = X509RsaSignatureRoute::new(1, X509RsaSignatureVerifier::RsaPkcs1V15Sha256);
    let duplicate = verify_x509_rsa_policy_signature(
        child_policy(&child, &profiles),
        issuer_policy(&issuer, &profiles),
        &[route, route],
        limits,
    )
    .unwrap_err();
    assert_eq!(
        duplicate.kind(),
        X509RsaSignatureRoutingErrorKind::AmbiguousProfileRoute
    );

    let mismatch = verify_x509_rsa_policy_signature(
        child_policy(&child, &profiles),
        issuer_policy(&issuer, &profiles),
        &[X509RsaSignatureRoute::new(
            1,
            X509RsaSignatureVerifier::RsaPkcs1V15Sha512,
        )],
        limits,
    )
    .unwrap_err();
    assert_eq!(
        mismatch.kind(),
        X509RsaSignatureRoutingErrorKind::Sha512(
            X509RsaSha512SignatureErrorKind::UnsupportedSignatureAlgorithm
        )
    );

    for error in [missing, duplicate, mismatch] {
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains("ISRG"));
        assert!(!rendered.contains("4096"));
    }
}
