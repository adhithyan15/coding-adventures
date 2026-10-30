use der_asn1::{Asn1ErrorKind, Asn1Limits};
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    validate_certificate_algorithm_policy, X509AlgorithmParametersPolicy,
    X509CertificateAlgorithmProfile,
};
use x509_rsa_algorithm_policy::{
    validate_x509_rsa_algorithm_policy, X509RsaAlgorithmPolicyErrorKind,
};
use x509_rsa_public_key::{RsaPublicKeyErrorKind, RsaPublicKeyPolicy};

const SHA256_WITH_RSA_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const SHA256_WITH_RSA_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const OTHER_KEY_OID_DER: &[u8] = &[0x2a, 0x03];
const OTHER_KEY_OID: &[u64] = &[1, 2, 3];
const NULL_DER: &[u8] = &[0x05, 0x00];

fn length(length: usize) -> Vec<u8> {
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

fn der(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = vec![tag];
    encoded.extend(length(value.len()));
    encoded.extend_from_slice(value);
    encoded
}

fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    der(0x30, &fields.concat())
}

fn positive_integer(magnitude: &[u8]) -> Vec<u8> {
    let mut contents = Vec::with_capacity(magnitude.len() + 1);
    if magnitude.first().is_some_and(|byte| byte & 0x80 != 0) {
        contents.push(0);
    }
    contents.extend_from_slice(magnitude);
    der(0x02, &contents)
}

fn modulus(bits: usize) -> Vec<u8> {
    let mut magnitude = vec![0; bits.div_ceil(8)];
    magnitude[0] = 1 << ((bits - 1) % 8);
    let last = magnitude.len() - 1;
    magnitude[last] |= 1;
    magnitude
}

fn exponent(value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len() - 1);
    positive_integer(&bytes[first..])
}

fn rsa_key(bits: usize, public_exponent: u64) -> Vec<u8> {
    sequence(&[positive_integer(&modulus(bits)), exponent(public_exponent)])
}

fn algorithm(oid: &[u8], parameters: Option<&[u8]>) -> Vec<u8> {
    let mut fields = vec![der(0x06, oid)];
    if let Some(parameters) = parameters {
        fields.push(parameters.to_vec());
    }
    sequence(&fields)
}

fn name() -> Vec<u8> {
    let attribute = sequence(&[der(0x06, &[0x55, 0x04, 0x03]), der(0x0c, b"example")]);
    sequence(&[der(0x31, &attribute)])
}

fn certificate(key_oid: &[u8], key_parameters: Option<&[u8]>, key: &[u8]) -> Vec<u8> {
    let validity = sequence(&[der(0x17, b"260101000000Z"), der(0x17, b"270101000000Z")]);
    let subject_public_key_info = sequence(&[algorithm(key_oid, key_parameters), {
        let mut bits = vec![0];
        bits.extend_from_slice(key);
        der(0x03, &bits)
    }]);
    let tbs = sequence(&[
        der(0xa0, &der(0x02, &[0x02])),
        der(0x02, &[0x01]),
        algorithm(SHA256_WITH_RSA_OID_DER, Some(NULL_DER)),
        name(),
        validity,
        name(),
        subject_public_key_info,
    ]);
    sequence(&[
        tbs,
        algorithm(SHA256_WITH_RSA_OID_DER, Some(NULL_DER)),
        der(0x03, &[0, 0xaa]),
    ])
}

fn algorithm_policy<'certificate, 'encoded>(
    certificate: &'certificate x509_certificate::X509Certificate<'encoded>,
    key_oid: &'static [u64],
    key_bits: usize,
) -> x509_certificate_policy::ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded> {
    let profiles = [X509CertificateAlgorithmProfile::new(
        SHA256_WITH_RSA_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        key_oid,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        key_bits,
        key_bits,
        8,
        8,
    )
    .unwrap()];
    validate_certificate_algorithm_policy(certificate, &profiles).unwrap()
}

fn rsa_policy<'a>(
    minimum_bits: usize,
    maximum_bits: usize,
    exponents: &'a [u64],
) -> RsaPublicKeyPolicy<'a> {
    RsaPublicKeyPolicy::new(minimum_bits, maximum_bits, exponents).unwrap()
}

#[test]
fn binds_selected_profile_certificate_and_exact_rsa_key() {
    let key = rsa_key(2_048, 65_537);
    let encoded = certificate(RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &key);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let policy = algorithm_policy(&certificate, RSA_ENCRYPTION_OID, key.len() * 8);

    let evidence = validate_x509_rsa_algorithm_policy(
        policy,
        rsa_policy(2_048, 4_096, &[3, 65_537]),
        Asn1Limits::default(),
    )
    .unwrap();

    assert!(std::ptr::eq(evidence.certificate(), &certificate));
    assert_eq!(
        evidence
            .algorithm_policy()
            .algorithm_profile()
            .profile_index(),
        0
    );
    assert_eq!(evidence.rsa_public_key().modulus_bits(), 2_048);
    assert_eq!(evidence.rsa_public_key().public_exponent(), 65_537);
    assert_eq!(format!("{evidence:?}"), "ValidatedX509RsaAlgorithmPolicy");
}

#[test]
fn applies_modulus_and_exponent_policy_to_the_retained_certificate() {
    let weak_key = rsa_key(1_024, 65_537);
    let weak_encoded = certificate(RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &weak_key);
    let weak_certificate = decode_x509_certificate(&weak_encoded, Asn1Limits::default()).unwrap();
    let weak_algorithm_policy =
        algorithm_policy(&weak_certificate, RSA_ENCRYPTION_OID, weak_key.len() * 8);
    assert_eq!(
        validate_x509_rsa_algorithm_policy(
            weak_algorithm_policy,
            rsa_policy(2_048, 4_096, &[65_537]),
            Asn1Limits::default(),
        )
        .unwrap_err()
        .kind(),
        X509RsaAlgorithmPolicyErrorKind::RsaPublicKey(
            RsaPublicKeyErrorKind::ModulusBitLengthOutOfRange
        )
    );

    let exponent_key = rsa_key(2_048, 3);
    let exponent_encoded = certificate(RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &exponent_key);
    let exponent_certificate =
        decode_x509_certificate(&exponent_encoded, Asn1Limits::default()).unwrap();
    let exponent_algorithm_policy = algorithm_policy(
        &exponent_certificate,
        RSA_ENCRYPTION_OID,
        exponent_key.len() * 8,
    );
    assert_eq!(
        validate_x509_rsa_algorithm_policy(
            exponent_algorithm_policy,
            rsa_policy(2_048, 4_096, &[65_537]),
            Asn1Limits::default(),
        )
        .unwrap_err()
        .kind(),
        X509RsaAlgorithmPolicyErrorKind::RsaPublicKey(
            RsaPublicKeyErrorKind::PublicExponentNotAllowed
        )
    );
}

#[test]
fn refuses_non_rsa_profiles_instead_of_treating_length_as_key_validation() {
    let opaque_key = [0x30, 0x00];
    let encoded = certificate(OTHER_KEY_OID_DER, Some(NULL_DER), &opaque_key);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let policy = algorithm_policy(&certificate, OTHER_KEY_OID, opaque_key.len() * 8);

    assert_eq!(
        validate_x509_rsa_algorithm_policy(
            policy,
            rsa_policy(8, 4_096, &[3]),
            Asn1Limits::default(),
        )
        .unwrap_err()
        .kind(),
        X509RsaAlgorithmPolicyErrorKind::RsaPublicKey(RsaPublicKeyErrorKind::WrongAlgorithm)
    );
}

#[test]
fn maps_malformed_nested_key_and_decoder_budget_without_payloads() {
    let malformed = [0x31, 0x00];
    let malformed_encoded = certificate(RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &malformed);
    let malformed_certificate =
        decode_x509_certificate(&malformed_encoded, Asn1Limits::default()).unwrap();
    let malformed_policy = algorithm_policy(
        &malformed_certificate,
        RSA_ENCRYPTION_OID,
        malformed.len() * 8,
    );
    let malformed_error = validate_x509_rsa_algorithm_policy(
        malformed_policy,
        rsa_policy(8, 4_096, &[3]),
        Asn1Limits::default(),
    )
    .unwrap_err();
    assert!(matches!(
        malformed_error.kind(),
        X509RsaAlgorithmPolicyErrorKind::RsaPublicKey(RsaPublicKeyErrorKind::Structure(
            Asn1ErrorKind::UnexpectedTag
        ))
    ));

    let key = rsa_key(2_048, 65_537);
    let encoded = certificate(RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &key);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let policy = algorithm_policy(&certificate, RSA_ENCRYPTION_OID, key.len() * 8);
    let limit_error = validate_x509_rsa_algorithm_policy(
        policy,
        rsa_policy(2_048, 4_096, &[65_537]),
        Asn1Limits {
            max_total_elements: 1,
            ..Asn1Limits::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        limit_error.kind(),
        X509RsaAlgorithmPolicyErrorKind::RsaPublicKey(RsaPublicKeyErrorKind::Structure(_))
    ));
    assert!(!limit_error.to_string().contains("65537"));
    assert!(!format!("{limit_error:?}").contains("65537"));
}

#[test]
fn requires_canonical_rsa_algorithm_parameters_from_the_exact_certificate() {
    let key = rsa_key(2_048, 65_537);
    let encoded = certificate(RSA_ENCRYPTION_OID_DER, None, &key);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let profiles = [X509CertificateAlgorithmProfile::new(
        SHA256_WITH_RSA_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::Absent,
        key.len() * 8,
        key.len() * 8,
        8,
        8,
    )
    .unwrap()];
    let policy = validate_certificate_algorithm_policy(&certificate, &profiles).unwrap();

    assert_eq!(
        validate_x509_rsa_algorithm_policy(
            policy,
            rsa_policy(2_048, 4_096, &[65_537]),
            Asn1Limits::default(),
        )
        .unwrap_err()
        .kind(),
        X509RsaAlgorithmPolicyErrorKind::RsaPublicKey(
            RsaPublicKeyErrorKind::MissingAlgorithmParameters
        )
    );
}
