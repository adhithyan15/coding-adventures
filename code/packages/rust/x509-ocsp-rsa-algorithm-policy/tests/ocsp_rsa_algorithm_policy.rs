use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_ocsp_basic_response::{decode_x509_ocsp_basic_response, X509OcspBasicResponse};
use x509_ocsp_rsa_algorithm_policy::{
    validate_x509_ocsp_rsa_algorithm_policy, X509OcspRsaAlgorithmParametersPolicy,
    X509OcspRsaAlgorithmPolicyErrorKind, X509OcspRsaAlgorithmProfile,
    X509OcspRsaAlgorithmProfileDefinitionErrorKind,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const SHA256_WITH_RSA_ENCRYPTION: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const SHA512_WITH_RSA_ENCRYPTION: &[u64] = &[1, 2, 840, 113_549, 1, 1, 13];
const SHA512_WITH_RSA_ENCRYPTION_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0d];
const NULL_DER: &[u8] = &[0x05, 0x00];
const RESPONSE_DATA_HEX: &str = concat!(
    "307ca21604144444444444444444444444444444444444444444180f32303236",
    "313032343132303030305a3051304f303a300906052b0e03021a050004141111",
    "1111111111111111111111111111111111110414222222222222222222222222",
    "22222222222222220201018000180f32303236313032343132333435365a",
);
const PUBLIC_SPKI_HEX: &str = concat!(
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

fn decode_hex(hexadecimal: &str) -> Vec<u8> {
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

fn response(oid: &[u8], parameters: Option<&[u8]>, signature: &[u8]) -> Vec<u8> {
    let mut algorithm_fields = vec![element(0x06, oid)];
    if let Some(parameters) = parameters {
        algorithm_fields.push(parameters.to_vec());
    }
    sequence(&[
        decode_hex(RESPONSE_DATA_HEX),
        sequence(&algorithm_fields),
        element(0x03, signature),
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
    validate_rsa_public_key(
        spki,
        RsaPublicKeyPolicy::new(2_048, 2_048, &[65_537]).unwrap(),
        Asn1Limits::default(),
    )
    .unwrap()
}

fn profile(
    oid: &'static [u64],
    parameters: X509OcspRsaAlgorithmParametersPolicy<'static>,
    modulus: (usize, usize),
    signature: (usize, usize),
    exponents: &'static [u64],
) -> X509OcspRsaAlgorithmProfile<'static> {
    X509OcspRsaAlgorithmProfile::new(
        oid,
        parameters,
        modulus.0,
        modulus.1,
        signature.0,
        signature.1,
        exponents,
    )
    .unwrap()
}

#[test]
fn profile_definitions_reject_noncanonical_or_empty_policy() {
    let cases = [
        (
            X509OcspRsaAlgorithmProfile::new(
                &[],
                X509OcspRsaAlgorithmParametersPolicy::Absent,
                1,
                1,
                1,
                1,
                &[3],
            )
            .unwrap_err()
            .kind(),
            X509OcspRsaAlgorithmProfileDefinitionErrorKind::EmptySignatureAlgorithm,
        ),
        (
            X509OcspRsaAlgorithmProfile::new(
                SHA512_WITH_RSA_ENCRYPTION,
                X509OcspRsaAlgorithmParametersPolicy::Absent,
                0,
                1,
                1,
                1,
                &[3],
            )
            .unwrap_err()
            .kind(),
            X509OcspRsaAlgorithmProfileDefinitionErrorKind::InvalidModulusBitLengthRange,
        ),
        (
            X509OcspRsaAlgorithmProfile::new(
                SHA512_WITH_RSA_ENCRYPTION,
                X509OcspRsaAlgorithmParametersPolicy::Absent,
                1,
                1,
                2,
                1,
                &[3],
            )
            .unwrap_err()
            .kind(),
            X509OcspRsaAlgorithmProfileDefinitionErrorKind::InvalidSignatureBitLengthRange,
        ),
        (
            X509OcspRsaAlgorithmProfile::new(
                SHA512_WITH_RSA_ENCRYPTION,
                X509OcspRsaAlgorithmParametersPolicy::Absent,
                1,
                1,
                1,
                1,
                &[],
            )
            .unwrap_err()
            .kind(),
            X509OcspRsaAlgorithmProfileDefinitionErrorKind::EmptyPublicExponentPolicy,
        ),
    ];
    for (actual, expected) in cases {
        assert_eq!(actual, expected);
    }

    let redacted = X509OcspRsaAlgorithmProfile::new(
        &[],
        X509OcspRsaAlgorithmParametersPolicy::Absent,
        1,
        1,
        1,
        1,
        &[3],
    )
    .unwrap_err();
    assert_eq!(
        redacted.to_string(),
        "X.509 OCSP RSA algorithm profile definition error EmptySignatureAlgorithm"
    );

    for exponents in [&[1][..], &[4][..], &[65_537, 3][..], &[3, 3][..]] {
        let error = X509OcspRsaAlgorithmProfile::new(
            SHA512_WITH_RSA_ENCRYPTION,
            X509OcspRsaAlgorithmParametersPolicy::Absent,
            1,
            1,
            1,
            1,
            exponents,
        )
        .unwrap_err();
        assert_eq!(
            error.kind(),
            X509OcspRsaAlgorithmProfileDefinitionErrorKind::InvalidPublicExponentPolicy
        );
    }
}

#[test]
fn selects_one_profile_and_binds_the_exact_response_and_key() {
    let encoded = response(SHA512_WITH_RSA_ENCRYPTION_DER, Some(NULL_DER), &[0, 0xaa]);
    let response = decode_response(&encoded);
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let profiles = [
        profile(
            SHA256_WITH_RSA_ENCRYPTION,
            X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER),
            (2_048, 4_096),
            (8, 4_096),
            &[65_537],
        ),
        profile(
            SHA512_WITH_RSA_ENCRYPTION,
            X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER),
            (2_048, 4_096),
            (8, 4_096),
            &[3, 65_537],
        ),
    ];

    let evidence = validate_x509_ocsp_rsa_algorithm_policy(&response, key, &profiles).unwrap();
    assert_eq!(evidence.profile_index(), 1);
    assert!(std::ptr::eq(evidence.response(), &response));
    assert_eq!(evidence.responder_public_key(), key);
    assert_eq!(
        format!("{evidence:?}"),
        "ValidatedX509OcspRsaAlgorithmPolicy { profile_index: 1, .. }"
    );
}

#[test]
fn rejects_unsupported_and_ambiguous_algorithm_profiles() {
    let encoded = response(SHA512_WITH_RSA_ENCRYPTION_DER, Some(NULL_DER), &[0, 0xaa]);
    let response = decode_response(&encoded);
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let absent = profile(
        SHA512_WITH_RSA_ENCRYPTION,
        X509OcspRsaAlgorithmParametersPolicy::Absent,
        (2_048, 2_048),
        (8, 8),
        &[65_537],
    );
    assert_eq!(
        validate_x509_ocsp_rsa_algorithm_policy(&response, key, &[absent])
            .unwrap_err()
            .kind(),
        X509OcspRsaAlgorithmPolicyErrorKind::UnsupportedAlgorithmProfile
    );

    let exact = profile(
        SHA512_WITH_RSA_ENCRYPTION,
        X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER),
        (2_048, 2_048),
        (8, 8),
        &[65_537],
    );
    assert_eq!(
        validate_x509_ocsp_rsa_algorithm_policy(&response, key, &[exact, exact])
            .unwrap_err()
            .kind(),
        X509OcspRsaAlgorithmPolicyErrorKind::AmbiguousAlgorithmProfile
    );
}

#[test]
fn rejects_selected_profiles_with_wrong_key_or_signature_bounds() {
    let encoded = response(SHA512_WITH_RSA_ENCRYPTION_DER, Some(NULL_DER), &[0, 0xaa]);
    let response = decode_response(&encoded);
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);

    let cases = [
        (
            profile(
                SHA512_WITH_RSA_ENCRYPTION,
                X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER),
                (2_049, 4_096),
                (8, 8),
                &[65_537],
            ),
            X509OcspRsaAlgorithmPolicyErrorKind::ModulusBitLengthOutOfPolicy,
        ),
        (
            profile(
                SHA512_WITH_RSA_ENCRYPTION,
                X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER),
                (2_048, 2_048),
                (8, 8),
                &[3],
            ),
            X509OcspRsaAlgorithmPolicyErrorKind::PublicExponentOutOfPolicy,
        ),
        (
            profile(
                SHA512_WITH_RSA_ENCRYPTION,
                X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER),
                (2_048, 2_048),
                (9, 16),
                &[65_537],
            ),
            X509OcspRsaAlgorithmPolicyErrorKind::SignatureBitLengthOutOfPolicy,
        ),
    ];

    for (profile, expected) in cases {
        let error =
            validate_x509_ocsp_rsa_algorithm_policy(&response, key, &[profile]).unwrap_err();
        assert_eq!(error.kind(), expected);
        assert!(!error.to_string().contains("65537"));
        assert!(!format!("{error:?}").contains("44444444"));
    }
}

#[test]
fn profile_accessors_expose_only_caller_policy_data() {
    let profile = profile(
        SHA512_WITH_RSA_ENCRYPTION,
        X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER),
        (2_048, 4_096),
        (2_048, 4_096),
        &[3, 65_537],
    );
    assert_eq!(profile.signature_algorithm(), SHA512_WITH_RSA_ENCRYPTION);
    assert_eq!(
        profile.signature_parameters(),
        X509OcspRsaAlgorithmParametersPolicy::ExactDer(NULL_DER)
    );
    assert_eq!(profile.minimum_modulus_bits(), 2_048);
    assert_eq!(profile.maximum_modulus_bits(), 4_096);
    assert_eq!(profile.minimum_signature_bits(), 2_048);
    assert_eq!(profile.maximum_signature_bits(), 4_096);
    assert_eq!(profile.permitted_public_exponents(), &[3, 65_537]);
}

#[test]
fn absent_parameter_policy_accepts_only_absence() {
    let encoded = response(SHA512_WITH_RSA_ENCRYPTION_DER, None, &[0, 0xaa]);
    let response = decode_response(&encoded);
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let profile = profile(
        SHA512_WITH_RSA_ENCRYPTION,
        X509OcspRsaAlgorithmParametersPolicy::Absent,
        (2_048, 2_048),
        (8, 8),
        &[65_537],
    );
    validate_x509_ocsp_rsa_algorithm_policy(&response, key, &[profile]).unwrap();
}
