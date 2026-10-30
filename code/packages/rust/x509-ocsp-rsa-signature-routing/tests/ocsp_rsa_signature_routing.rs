use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use x509_ocsp_basic_response::{decode_x509_ocsp_basic_response, X509OcspBasicResponse};
use x509_ocsp_rsa_algorithm_policy::{
    validate_x509_ocsp_rsa_algorithm_policy, ValidatedX509OcspRsaAlgorithmPolicy,
    X509OcspRsaAlgorithmParametersPolicy, X509OcspRsaAlgorithmProfile,
};
use x509_ocsp_rsa_pkcs1_v1_5_sha256_signature::X509OcspRsaSha256SignatureErrorKind;
use x509_ocsp_rsa_pkcs1_v1_5_sha512_signature::X509OcspRsaSha512SignatureErrorKind;
use x509_ocsp_rsa_signature_routing::{
    verify_x509_ocsp_rsa_policy_signature, VerifiedX509OcspRsaPolicySignature,
    X509OcspRsaSignatureRoute, X509OcspRsaSignatureRoutingErrorKind, X509OcspRsaSignatureVerifier,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const SHA512_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 13];
const SHA256_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const SHA512_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0d];
const NULL_DER: &[u8] = &[0x05, 0x00];

const RESPONSE_DATA_HEX: &str = concat!(
    "307ca21604144444444444444444444444444444444444444444180f32303236",
    "313032343132303030305a3051304f303a300906052b0e03021a050004141111",
    "1111111111111111111111111111111111110414222222222222222222222222",
    "22222222222222220201018000180f32303236313032343132333435365a",
);
const SHA256_PUBLIC_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a02820101",
    "00e550386fb8134a3a2e3e850d7a3d8ea5d6e601391b7c5c3327684f3d00b8",
    "6e517e4c210888b64d1f84699b74c18546a7dc992e9a4dcbaac6a6d688e653",
    "0d7080645513562c24c1cdb016f5bed4ce6b4b2ec2e07e63fceeec14f62805",
    "9ff62588e6a4f931714b85e1658de0b6ea4b2def535776efe7c0a816e99c3a",
    "a2202250fa12ea46de1a13d23722a0b38d9690a75823c851700ad5f274a9331",
    "c0ce16a78eb36af10291320c43a5685d4aaab91ae8dc8c7450977614c7200c",
    "ddaaff26a05e3435b885254f8477da784a026fed15773341329088b6b8fd729",
    "8a8fe8978ed1c06c37ce01dfe6b181e9378b860018ffc9ac5954448e4bf3f9",
    "bcb688498d282f210203010001",
);
const SHA256_SIGNATURE_HEX: &str = concat!(
    "9c6c6cceada1c781da228be19c80408d63e311076efa3f88932bedfa7624cdaa",
    "98af8f6e9948483b26e75efab65bfaff0d5b60b5538e3301f8f1f4784460141",
    "63a3a5c6ee19a6b19691945158ab2a9b6258cd9183dd60b330cce640d7258d7",
    "dbf0ab26a4548a9ac7a9a67edd9f54ae50602a15e7cb839601a6a1b840c217",
    "662d8b835ffdbdbd38f55c02d031eb534efc257643439215f33703257219fd9",
    "6c0fbf3fff597f50b9954346195e8643b202ae86d2ca16d5e56f0c27f8dcfa",
    "b80a4df8a6ad10abf07a5b7aa879511d2b25f56663405cd6bf77abedd5a5fa",
    "f906fb5b073c78134731d0d92c8201065a3c2bcebb5464a1d65cbdad4f21fb",
    "850c0bf8a1f",
);
const SHA512_PUBLIC_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a02820101",
    "00bf3b7ec4399b92c6cc41bb49a5962a6a439823ccf6d95066624b5e674b5e",
    "324d48407ecfc8cbebf714c0267370569c1a4f4b3c74e454519be2244e4626",
    "895bae3dc2d3c8a8f5b0924717a7de76727750cc2f9437244e76562a385e0c",
    "36bbaf772d506b9b89c86c05f472c0dca7165acc9ba2a6d913618ce62a9711",
    "0b2277db926d385892820f7d852643b583851a93a153d9fbc3fd892d13f5cd",
    "6a1e80811afadff990f7f929b6deef83b37ea46784446165cc8f7e30b530c4",
    "77e986cffb93a7cfcbe0753f7a7bb5ae8470c91336f8e314a10a5348a0a496",
    "acd8ce97404feee5461f5c6d95ec4f779070da2576fce22daffdd90576d152",
    "59308a829d97179fc30203010001",
);
const SHA512_SIGNATURE_HEX: &str = concat!(
    "4f45613e4185e22339af206eaf888d233ef899669f355d34918ba3b33e99dba8",
    "ff86f4a0a90101ae6586d18ed1c67f10245e7941b939d51c876b9341b1c646a5",
    "d3dd02bae3742f5370e2bb006836eb5541513451f97454e9fff4c73b756defba",
    "a16b5262a03c2f3c850623b35d6c6a6ccfdd0fe358634c376c84ff51445252a0",
    "15b6cd8fa18742c4d6e148a49f697f98fd34f1767b1b2b6e0ba9d16fdd9a7a43",
    "6511c2ae5f9a9770b724ca1b952191413de8bc9b484581c4ab2ac163e4335f8d",
    "69f758773a98a433d45389a4de1a694eda2ad1569604035e35c0961b33bf40e2",
    "b2f012a1c12797ef549b048317bacf91b69aca6244c1da3eda0b26ee149a4d38",
);

fn decode_hex(hexadecimal: &str) -> Vec<u8> {
    assert!(hexadecimal.len().is_multiple_of(2));
    hexadecimal
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
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

fn fixture_response(oid: &[u8], signature_hex: &str) -> Vec<u8> {
    let algorithm = sequence(&[element(0x06, oid), NULL_DER.to_vec()]);
    let mut signature = vec![0];
    signature.extend(decode_hex(signature_hex));
    sequence(&[
        decode_hex(RESPONSE_DATA_HEX),
        algorithm,
        element(0x03, &signature),
    ])
}

fn decode_response(encoded: &[u8]) -> X509OcspBasicResponse<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    decode_x509_ocsp_basic_response(&mut decoder, root).unwrap()
}

fn validated_key(encoded: &[u8]) -> ValidatedRsaPublicKey<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    let spki = decode_x509_subject_public_key_info(&mut decoder, root).unwrap();
    let policy = RsaPublicKeyPolicy::new(2_048, 2_048, &[65_537]).unwrap();
    validate_rsa_public_key(spki, policy, Asn1Limits::default()).unwrap()
}

fn algorithm_policy<'response, 'encoded, 'key>(
    response: &'response X509OcspBasicResponse<'encoded>,
    key: ValidatedRsaPublicKey<'key>,
    profiles: &[X509OcspRsaAlgorithmProfile<'static>],
) -> ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key> {
    validate_x509_ocsp_rsa_algorithm_policy(response, key, profiles).unwrap()
}

fn profile(oid: &'static [u64]) -> X509OcspRsaAlgorithmProfile<'static> {
    X509OcspRsaAlgorithmProfile::new(
        oid,
        X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        2_048,
        2_048,
        2_048,
        &[65_537],
    )
    .unwrap()
}

#[test]
fn routes_sha256_by_selected_profile_index_and_binds_evidence() {
    let encoded = fixture_response(SHA256_WITH_RSA_ENCRYPTION_OID_DER, SHA256_SIGNATURE_HEX);
    let response = decode_response(&encoded);
    let spki = decode_hex(SHA256_PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let profiles = [
        profile(SHA512_WITH_RSA_ENCRYPTION_OID),
        profile(SHA256_WITH_RSA_ENCRYPTION_OID),
    ];
    let policy = algorithm_policy(&response, key, &profiles);
    assert_eq!(policy.profile_index(), 1);
    let route = X509OcspRsaSignatureRoute::new(1, X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha256);
    assert_eq!(route.profile_index(), 1);
    assert_eq!(
        route.verifier(),
        X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha256
    );

    let verified = verify_x509_ocsp_rsa_policy_signature(
        policy,
        &[route],
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap();
    assert_eq!(
        verified.verifier(),
        X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha256
    );
    assert_eq!(
        format!("{verified:?}"),
        "VerifiedX509OcspRsaPolicySignature::Sha256"
    );
    let VerifiedX509OcspRsaPolicySignature::Sha256(verified) = verified else {
        panic!("selected the wrong verifier")
    };
    assert_eq!(
        format!("{verified:?}"),
        "VerifiedX509OcspRsaSha256PolicySignature"
    );
    assert_eq!(verified.algorithm_policy().profile_index(), 1);
    assert!(std::ptr::eq(
        verified.algorithm_policy().response(),
        verified.signature().response()
    ));
    assert_eq!(
        verified.algorithm_policy().responder_public_key(),
        verified.signature().responder_public_key()
    );
}

#[test]
fn routes_sha512_and_binds_evidence() {
    let encoded = fixture_response(SHA512_WITH_RSA_ENCRYPTION_OID_DER, SHA512_SIGNATURE_HEX);
    let response = decode_response(&encoded);
    let spki = decode_hex(SHA512_PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let profiles = [profile(SHA512_WITH_RSA_ENCRYPTION_OID)];
    let policy = algorithm_policy(&response, key, &profiles);
    let routes = [X509OcspRsaSignatureRoute::new(
        0,
        X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha512,
    )];

    let verified = verify_x509_ocsp_rsa_policy_signature(
        policy,
        &routes,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap();
    assert_eq!(
        verified.verifier(),
        X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha512
    );
    assert_eq!(
        format!("{verified:?}"),
        "VerifiedX509OcspRsaPolicySignature::Sha512"
    );
    let VerifiedX509OcspRsaPolicySignature::Sha512(verified) = verified else {
        panic!("selected the wrong verifier")
    };
    assert_eq!(
        format!("{verified:?}"),
        "VerifiedX509OcspRsaSha512PolicySignature"
    );
    assert!(std::ptr::eq(
        verified.algorithm_policy().response(),
        verified.signature().response()
    ));
    assert_eq!(
        verified.algorithm_policy().responder_public_key(),
        verified.signature().responder_public_key()
    );
}

#[test]
fn rejects_missing_duplicate_and_mismatched_routes_with_redacted_errors() {
    let encoded = fixture_response(SHA256_WITH_RSA_ENCRYPTION_OID_DER, SHA256_SIGNATURE_HEX);
    let response = decode_response(&encoded);
    let spki = decode_hex(SHA256_PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let profiles = [profile(SHA256_WITH_RSA_ENCRYPTION_OID)];
    let limits = RsaPublicOperationLimits::new(256).unwrap();

    let missing = verify_x509_ocsp_rsa_policy_signature(
        algorithm_policy(&response, key, &profiles),
        &[],
        limits,
    )
    .unwrap_err();
    assert_eq!(
        missing.kind(),
        X509OcspRsaSignatureRoutingErrorKind::UnsupportedProfileRoute
    );

    let route = X509OcspRsaSignatureRoute::new(0, X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha256);
    let duplicate = verify_x509_ocsp_rsa_policy_signature(
        algorithm_policy(&response, key, &profiles),
        &[route, route],
        limits,
    )
    .unwrap_err();
    assert_eq!(
        duplicate.kind(),
        X509OcspRsaSignatureRoutingErrorKind::AmbiguousProfileRoute
    );

    let mismatch = verify_x509_ocsp_rsa_policy_signature(
        algorithm_policy(&response, key, &profiles),
        &[X509OcspRsaSignatureRoute::new(
            0,
            X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha512,
        )],
        limits,
    )
    .unwrap_err();
    assert_eq!(
        mismatch.kind(),
        X509OcspRsaSignatureRoutingErrorKind::Sha512(
            X509OcspRsaSha512SignatureErrorKind::UnsupportedSignatureAlgorithm
        )
    );

    let sha512_encoded = fixture_response(SHA512_WITH_RSA_ENCRYPTION_OID_DER, SHA512_SIGNATURE_HEX);
    let sha512_response = decode_response(&sha512_encoded);
    let sha512_spki = decode_hex(SHA512_PUBLIC_SPKI_HEX);
    let sha512_key = validated_key(&sha512_spki);
    let sha512_profiles = [profile(SHA512_WITH_RSA_ENCRYPTION_OID)];
    let sha256_mismatch = verify_x509_ocsp_rsa_policy_signature(
        algorithm_policy(&sha512_response, sha512_key, &sha512_profiles),
        &[X509OcspRsaSignatureRoute::new(
            0,
            X509OcspRsaSignatureVerifier::RsaPkcs1V15Sha256,
        )],
        limits,
    )
    .unwrap_err();
    assert_eq!(
        sha256_mismatch.kind(),
        X509OcspRsaSignatureRoutingErrorKind::Sha256(
            X509OcspRsaSha256SignatureErrorKind::UnsupportedSignatureAlgorithm
        )
    );
    for error in [missing, duplicate, mismatch, sha256_mismatch] {
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains(RESPONSE_DATA_HEX));
        assert!(!rendered.contains(SHA256_PUBLIC_SPKI_HEX));
    }
}
