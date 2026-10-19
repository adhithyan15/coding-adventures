use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_rsa_public_key::{
    validate_rsa_public_key, RsaPublicKeyErrorKind, RsaPublicKeyPolicy,
    RsaPublicKeyPolicyDefinitionErrorKind, RSA_ENCRYPTION_OID,
};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const SHA256_WITH_RSA_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];

fn length(length: usize) -> Vec<u8> {
    if length < 128 {
        return vec![length as u8];
    }
    let bytes = length.to_be_bytes();
    let first = bytes.iter().position(|byte| *byte != 0).unwrap();
    let encoded = &bytes[first..];
    let mut result = vec![0x80 | encoded.len() as u8];
    result.extend_from_slice(encoded);
    result
}

fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = vec![tag];
    encoded.extend(length(value.len()));
    encoded.extend_from_slice(value);
    encoded
}

fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    let value = fields.concat();
    tlv(0x30, &value)
}

fn integer_signed(contents: &[u8]) -> Vec<u8> {
    tlv(0x02, contents)
}

fn positive_integer(magnitude: &[u8]) -> Vec<u8> {
    let mut contents = Vec::with_capacity(magnitude.len() + 1);
    if magnitude.first().is_some_and(|byte| byte & 0x80 != 0) {
        contents.push(0);
    }
    contents.extend_from_slice(magnitude);
    integer_signed(&contents)
}

fn exponent(value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len() - 1);
    positive_integer(&bytes[first..])
}

fn modulus(bits: usize) -> Vec<u8> {
    assert!(bits > 1);
    let mut magnitude = vec![0; bits.div_ceil(8)];
    let high_bit = (bits - 1) % 8;
    magnitude[0] = 1 << high_bit;
    let last = magnitude.len() - 1;
    magnitude[last] |= 1;
    magnitude
}

fn rsa_key(modulus_integer: Vec<u8>, exponent_integer: Vec<u8>) -> Vec<u8> {
    sequence(&[modulus_integer, exponent_integer])
}

fn algorithm(oid_contents: &[u8], parameters: Option<Vec<u8>>) -> Vec<u8> {
    let mut fields = vec![tlv(0x06, oid_contents)];
    if let Some(parameters) = parameters {
        fields.push(parameters);
    }
    sequence(&fields)
}

fn spki_with(
    oid_contents: &[u8],
    parameters: Option<Vec<u8>>,
    key: &[u8],
    unused_bits: u8,
) -> Vec<u8> {
    let mut bit_string = vec![unused_bits];
    bit_string.extend_from_slice(key);
    sequence(&[algorithm(oid_contents, parameters), tlv(0x03, &bit_string)])
}

fn valid_spki(bits: usize, public_exponent: u64) -> Vec<u8> {
    let key = rsa_key(positive_integer(&modulus(bits)), exponent(public_exponent));
    spki_with(RSA_ENCRYPTION_OID_DER, Some(vec![0x05, 0x00]), &key, 0)
}

fn validate_with<'a>(
    encoded: &'a [u8],
    policy: RsaPublicKeyPolicy<'_>,
    inner_limits: Asn1Limits,
) -> Result<x509_rsa_public_key::ValidatedRsaPublicKey<'a>, x509_rsa_public_key::RsaPublicKeyError>
{
    let mut outer = Asn1Decoder::new(Asn1Limits::default());
    let root = outer.decode_exact(encoded).expect("valid generic DER root");
    let spki = decode_x509_subject_public_key_info(&mut outer, root)
        .expect("valid generic SubjectPublicKeyInfo");
    validate_rsa_public_key(spki, policy, inner_limits)
}

fn policy<'a>(minimum: usize, maximum: usize, exponents: &'a [u64]) -> RsaPublicKeyPolicy<'a> {
    RsaPublicKeyPolicy::new(minimum, maximum, exponents).unwrap()
}

#[test]
fn validates_exact_rsa_spki_and_borrows_normalized_key() {
    assert_eq!(RSA_ENCRYPTION_OID, &[1, 2, 840, 113549, 1, 1, 1]);
    let encoded = valid_spki(2048, 65_537);
    let validated = validate_with(
        &encoded,
        policy(2048, 4096, &[3, 65_537]),
        Asn1Limits::default(),
    )
    .unwrap();

    assert_eq!(validated.modulus_bits(), 2048);
    assert_eq!(validated.modulus().len(), 256);
    assert_eq!(validated.modulus()[0], 0x80);
    assert_eq!(validated.modulus().last(), Some(&1));
    assert_eq!(validated.public_exponent(), 65_537);
}

#[test]
fn accepts_inclusive_modulus_boundaries_and_multiple_allowed_exponents() {
    let allowed_exponents = [3, 65_537];
    let accepted_policy = policy(2048, 4096, &allowed_exponents);
    assert_eq!(accepted_policy.minimum_modulus_bits(), 2048);
    assert_eq!(accepted_policy.maximum_modulus_bits(), 4096);
    assert_eq!(
        accepted_policy.allowed_public_exponents(),
        &allowed_exponents
    );

    for (bits, public_exponent) in [(2048, 3), (4096, 65_537)] {
        let encoded = valid_spki(bits, public_exponent);
        let validated = validate_with(&encoded, accepted_policy, Asn1Limits::default()).unwrap();
        assert_eq!(validated.modulus_bits(), bits);
        assert_eq!(validated.public_exponent(), public_exponent);
    }
}

#[test]
fn rejects_invalid_policy_definitions_without_retaining_values() {
    let cases = [
        (
            RsaPublicKeyPolicy::new(0, 2048, &[65_537]).unwrap_err(),
            RsaPublicKeyPolicyDefinitionErrorKind::ZeroMinimumModulusBits,
        ),
        (
            RsaPublicKeyPolicy::new(4096, 2048, &[65_537]).unwrap_err(),
            RsaPublicKeyPolicyDefinitionErrorKind::InvertedModulusBitRange,
        ),
        (
            RsaPublicKeyPolicy::new(2048, 4096, &[]).unwrap_err(),
            RsaPublicKeyPolicyDefinitionErrorKind::EmptyAllowedPublicExponents,
        ),
        (
            RsaPublicKeyPolicy::new(2048, 4096, &[1]).unwrap_err(),
            RsaPublicKeyPolicyDefinitionErrorKind::InvalidAllowedPublicExponent,
        ),
        (
            RsaPublicKeyPolicy::new(2048, 4096, &[4]).unwrap_err(),
            RsaPublicKeyPolicyDefinitionErrorKind::InvalidAllowedPublicExponent,
        ),
        (
            RsaPublicKeyPolicy::new(2048, 4096, &[65_537, 65_537]).unwrap_err(),
            RsaPublicKeyPolicyDefinitionErrorKind::DuplicateAllowedPublicExponent,
        ),
    ];
    for (error, kind) in cases {
        assert_eq!(error.kind(), kind);
        let rendered = error.to_string();
        assert!(!rendered.contains("65537"));
        assert!(!rendered.contains("4096"));
    }
}

#[test]
fn requires_exact_rsa_algorithm_and_present_null_parameters() {
    let key = rsa_key(positive_integer(&modulus(2048)), exponent(65_537));
    let wrong_oid = spki_with(SHA256_WITH_RSA_OID_DER, Some(vec![0x05, 0]), &key, 0);
    assert_eq!(
        validate_with(
            &wrong_oid,
            policy(2048, 4096, &[65_537]),
            Asn1Limits::default()
        )
        .unwrap_err()
        .kind(),
        RsaPublicKeyErrorKind::WrongAlgorithm
    );

    let absent = spki_with(RSA_ENCRYPTION_OID_DER, None, &key, 0);
    assert_eq!(
        validate_with(
            &absent,
            policy(2048, 4096, &[65_537]),
            Asn1Limits::default()
        )
        .unwrap_err()
        .kind(),
        RsaPublicKeyErrorKind::MissingAlgorithmParameters
    );

    for parameters in [vec![0x04, 0], vec![0x05, 1, 0]] {
        let encoded = spki_with(RSA_ENCRYPTION_OID_DER, Some(parameters), &key, 0);
        assert!(matches!(
            validate_with(
                &encoded,
                policy(2048, 4096, &[65_537]),
                Asn1Limits::default()
            )
            .unwrap_err()
            .kind(),
            RsaPublicKeyErrorKind::InvalidAlgorithmParameters(_)
        ));
    }
}

#[test]
fn rejects_empty_non_aligned_and_non_sequence_key_payloads() {
    let empty = spki_with(RSA_ENCRYPTION_OID_DER, Some(vec![0x05, 0]), &[], 0);
    assert_eq!(
        validate_with(&empty, policy(8, 4096, &[3]), Asn1Limits::default())
            .unwrap_err()
            .kind(),
        RsaPublicKeyErrorKind::EmptySubjectPublicKey
    );

    let non_aligned = spki_with(RSA_ENCRYPTION_OID_DER, Some(vec![0x05, 0]), &[0], 1);
    assert_eq!(
        validate_with(&non_aligned, policy(8, 4096, &[3]), Asn1Limits::default())
            .unwrap_err()
            .kind(),
        RsaPublicKeyErrorKind::NonByteAlignedSubjectPublicKey
    );

    for payload in [vec![0x31, 0], vec![0x30, 0, 0x05, 0]] {
        let encoded = spki_with(RSA_ENCRYPTION_OID_DER, Some(vec![0x05, 0]), &payload, 0);
        assert!(matches!(
            validate_with(&encoded, policy(8, 4096, &[3]), Asn1Limits::default())
                .unwrap_err()
                .kind(),
            RsaPublicKeyErrorKind::Structure(_)
        ));
    }
}

#[test]
fn rejects_missing_malformed_and_trailing_fields() {
    for (payload, kind) in [
        (sequence(&[]), RsaPublicKeyErrorKind::MissingModulus),
        (
            sequence(&[positive_integer(&modulus(16))]),
            RsaPublicKeyErrorKind::MissingPublicExponent,
        ),
        (
            sequence(&[positive_integer(&modulus(16)), exponent(3), vec![0x05, 0]]),
            RsaPublicKeyErrorKind::TrailingElement,
        ),
    ] {
        let encoded = spki_with(RSA_ENCRYPTION_OID_DER, Some(vec![0x05, 0]), &payload, 0);
        assert_eq!(
            validate_with(&encoded, policy(8, 4096, &[3]), Asn1Limits::default())
                .unwrap_err()
                .kind(),
            kind
        );
    }

    for payload in [
        sequence(&[vec![0x02, 2, 1]]),
        sequence(&[positive_integer(&modulus(16)), vec![0x02, 2, 1]]),
        sequence(&[
            positive_integer(&modulus(16)),
            exponent(3),
            vec![0x02, 2, 1],
        ]),
    ] {
        let encoded = spki_with(RSA_ENCRYPTION_OID_DER, Some(vec![0x05, 0]), &payload, 0);
        assert!(matches!(
            validate_with(&encoded, policy(8, 4096, &[3]), Asn1Limits::default())
                .unwrap_err()
                .kind(),
            RsaPublicKeyErrorKind::Structure(Asn1ErrorKind::Framing(_))
        ));
    }

    for (payload, kind) in [
        (
            sequence(&[vec![0x05, 0], exponent(3)]),
            RsaPublicKeyErrorKind::InvalidModulus(Asn1ErrorKind::UnexpectedTag),
        ),
        (
            sequence(&[positive_integer(&modulus(16)), vec![0x05, 0]]),
            RsaPublicKeyErrorKind::InvalidPublicExponent(Asn1ErrorKind::UnexpectedTag),
        ),
    ] {
        let encoded = spki_with(RSA_ENCRYPTION_OID_DER, Some(vec![0x05, 0]), &payload, 0);
        assert_eq!(
            validate_with(&encoded, policy(8, 4096, &[3]), Asn1Limits::default())
                .unwrap_err()
                .kind(),
            kind
        );
    }
}

#[test]
fn rejects_non_positive_even_and_out_of_range_moduli() {
    let cases = [
        (
            integer_signed(&[0]),
            RsaPublicKeyErrorKind::NonPositiveModulus,
        ),
        (
            integer_signed(&[0x80]),
            RsaPublicKeyErrorKind::NonPositiveModulus,
        ),
        (
            positive_integer(&[0x01, 0x00]),
            RsaPublicKeyErrorKind::EvenModulus,
        ),
        (
            positive_integer(&modulus(2047)),
            RsaPublicKeyErrorKind::ModulusBitLengthOutOfRange,
        ),
        (
            positive_integer(&modulus(4097)),
            RsaPublicKeyErrorKind::ModulusBitLengthOutOfRange,
        ),
    ];
    for (modulus_integer, kind) in cases {
        let payload = rsa_key(modulus_integer, exponent(3));
        let encoded = spki_with(RSA_ENCRYPTION_OID_DER, Some(vec![0x05, 0]), &payload, 0);
        assert_eq!(
            validate_with(&encoded, policy(2048, 4096, &[3]), Asn1Limits::default())
                .unwrap_err()
                .kind(),
            kind
        );
    }
}

#[test]
fn rejects_invalid_unlisted_and_modulus_sized_exponents() {
    let huge_exponent = positive_integer(&[1, 0, 0, 0, 0, 0, 0, 0, 0]);
    let cases = [
        (
            integer_signed(&[0xff]),
            RsaPublicKeyErrorKind::InvalidPublicExponent(Asn1ErrorKind::NegativeInteger),
        ),
        (exponent(0), RsaPublicKeyErrorKind::PublicExponentTooSmall),
        (exponent(2), RsaPublicKeyErrorKind::PublicExponentTooSmall),
        (exponent(4), RsaPublicKeyErrorKind::EvenPublicExponent),
        (
            huge_exponent,
            RsaPublicKeyErrorKind::InvalidPublicExponent(Asn1ErrorKind::IntegerOverflow),
        ),
        (
            exponent(17),
            RsaPublicKeyErrorKind::PublicExponentNotAllowed,
        ),
    ];
    for (exponent_integer, kind) in cases {
        let payload = rsa_key(positive_integer(&modulus(2048)), exponent_integer);
        let encoded = spki_with(RSA_ENCRYPTION_OID_DER, Some(vec![0x05, 0]), &payload, 0);
        assert_eq!(
            validate_with(
                &encoded,
                policy(2048, 4096, &[3, 65_537]),
                Asn1Limits::default()
            )
            .unwrap_err()
            .kind(),
            kind
        );
    }

    let small_modulus = rsa_key(positive_integer(&[5]), exponent(5));
    let encoded = spki_with(
        RSA_ENCRYPTION_OID_DER,
        Some(vec![0x05, 0]),
        &small_modulus,
        0,
    );
    assert_eq!(
        validate_with(&encoded, policy(1, 8, &[5]), Asn1Limits::default())
            .unwrap_err()
            .kind(),
        RsaPublicKeyErrorKind::PublicExponentNotLessThanModulus
    );
}

#[test]
fn enforces_nested_depth_element_tag_and_value_limits() {
    let encoded = valid_spki(2048, 65_537);
    let depth = Asn1Limits {
        max_depth: 1,
        ..Asn1Limits::default()
    };
    assert_eq!(
        validate_with(&encoded, policy(2048, 4096, &[65_537]), depth)
            .unwrap_err()
            .kind(),
        RsaPublicKeyErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    );

    let elements = Asn1Limits {
        max_total_elements: 1,
        ..Asn1Limits::default()
    };
    assert_eq!(
        validate_with(&encoded, policy(2048, 4096, &[65_537]), elements)
            .unwrap_err()
            .kind(),
        RsaPublicKeyErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );

    let mut tag = Asn1Limits::default();
    tag.der.max_tag_number = 1;
    assert!(matches!(
        validate_with(&encoded, policy(2048, 4096, &[65_537]), tag)
            .unwrap_err()
            .kind(),
        RsaPublicKeyErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));

    let mut value = Asn1Limits::default();
    value.der.max_value_len = 64;
    assert!(matches!(
        validate_with(&encoded, policy(2048, 4096, &[65_537]), value)
            .unwrap_err()
            .kind(),
        RsaPublicKeyErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
}

#[test]
fn errors_are_payload_local_and_redacted() {
    let payload = sequence(&[
        positive_integer(&modulus(2048)),
        exponent(17),
        vec![0x05, 0],
    ]);
    let encoded = spki_with(RSA_ENCRYPTION_OID_DER, Some(vec![0x05, 0]), &payload, 0);
    let error = validate_with(
        &encoded,
        policy(2048, 4096, &[65_537]),
        Asn1Limits::default(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPublicKeyErrorKind::PublicExponentNotAllowed
    );
    assert!(error.offset() > 0);
    let rendered = error.to_string();
    assert!(!rendered.contains("65537"));
    assert!(!rendered.contains("17"));
    assert!(!rendered.contains("2a8648"));
}
