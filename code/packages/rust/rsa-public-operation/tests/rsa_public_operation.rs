use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::{
    recover_encoded_message, RsaPublicOperationErrorKind, RsaPublicOperationLimits,
    RsaPublicOperationLimitsErrorKind,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

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
    tlv(0x30, &fields.concat())
}

fn positive_integer(magnitude: &[u8]) -> Vec<u8> {
    let mut contents = Vec::with_capacity(magnitude.len() + 1);
    if magnitude.first().is_some_and(|byte| byte & 0x80 != 0) {
        contents.push(0);
    }
    contents.extend_from_slice(magnitude);
    tlv(0x02, &contents)
}

fn exponent(value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len() - 1);
    positive_integer(&bytes[first..])
}

fn validated_key<'a>(encoded: &'a [u8], allowed_exponents: &[u64]) -> ValidatedRsaPublicKey<'a> {
    let mut outer = Asn1Decoder::new(Asn1Limits::default());
    let root = outer.decode_exact(encoded).unwrap();
    let spki = decode_x509_subject_public_key_info(&mut outer, root).unwrap();
    let policy = RsaPublicKeyPolicy::new(2, encoded.len() * 8, allowed_exponents).unwrap();
    validate_rsa_public_key(spki, policy, Asn1Limits::default()).unwrap()
}

fn spki(modulus: &[u8], public_exponent: u64) -> Vec<u8> {
    let key = sequence(&[positive_integer(modulus), exponent(public_exponent)]);
    let algorithm = sequence(&[tlv(0x06, RSA_ENCRYPTION_OID_DER), vec![0x05, 0x00]]);
    let mut bit_string = vec![0];
    bit_string.extend_from_slice(&key);
    sequence(&[algorithm, tlv(0x03, &bit_string)])
}

fn decode_hex(hexadecimal: &str) -> Vec<u8> {
    assert!(hexadecimal.len().is_multiple_of(2));
    hexadecimal
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}

#[test]
fn validates_nonzero_operational_limit_without_retaining_values() {
    let error = RsaPublicOperationLimits::new(0).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPublicOperationLimitsErrorKind::ZeroMaximumModulusBytes
    );
    assert!(!error.to_string().contains('0'));

    let limits = RsaPublicOperationLimits::new(512).unwrap();
    assert_eq!(limits.maximum_modulus_bytes(), 512);
}

#[test]
fn applies_textbook_rsa_public_operation() {
    // Textbook RSA: n = 61 * 53 = 3233, e = 17, and 2790^17 mod n = 1452.
    let encoded = spki(&[0x0c, 0xa1], 17);
    let key = validated_key(&encoded, &[17]);
    let recovered = recover_encoded_message(
        key,
        &[0x0a, 0xe6],
        RsaPublicOperationLimits::new(2).unwrap(),
    )
    .unwrap();
    assert_eq!(recovered, [0x05, 0xac]);
}

#[test]
fn preserves_exact_modulus_width_for_zero_one_and_leading_zero_results() {
    let modulus = decode_hex("d5bbb96d30086ec484eba3d7f9caeb07a55e650837da1e731b1b4d8f6f4e9a13");
    let encoded = spki(&modulus, 65_537);
    let key = validated_key(&encoded, &[65_537]);
    let limits = RsaPublicOperationLimits::new(32).unwrap();

    let zero = recover_encoded_message(key, &[0; 32], limits).unwrap();
    assert_eq!(zero, vec![0; 32]);

    let mut one_signature = vec![0; 32];
    one_signature[31] = 1;
    let one = recover_encoded_message(key, &one_signature, limits).unwrap();
    assert_eq!(one, one_signature);

    let signature = decode_hex("0000000000000000000000000000000000000000000000000000000000000029");
    let recovered = recover_encoded_message(key, &signature, limits).unwrap();
    assert_eq!(
        recovered,
        decode_hex("0000f6f6bde5ced079f64828818daf047a34c94b04ac8204e6005d3f55ed1a35")
    );
}

#[test]
fn matches_independent_vectors_for_sparse_and_dense_public_exponents() {
    let modulus = decode_hex("d5bbb96d30086ec484eba3d7f9caeb07a55e650837da1e731b1b4d8f6f4e9a13");
    let signature = decode_hex("23456789abcdef00112233445566778899aabbccddeeff00123456789abcdef0");
    for (public_exponent, expected) in [
        (
            3,
            "2ba356cb0ee64bd5fdcd324a62d3d00b60547ec2aaa744f5149862a80e7a7f4c",
        ),
        (
            17,
            "c1a9959d58b7d2c00657b858076128526d3632d3d674f5c8cced7810a39f2665",
        ),
        (
            65_537,
            "3fc7f1c646def9bd1a2089f2f68e2c21ac734cae56814fa5ffa5ed0defbd0ed1",
        ),
    ] {
        let encoded = spki(&modulus, public_exponent);
        let key = validated_key(&encoded, &[public_exponent]);
        let recovered =
            recover_encoded_message(key, &signature, RsaPublicOperationLimits::new(32).unwrap())
                .unwrap();
        assert_eq!(recovered, decode_hex(expected));
    }
}

#[test]
fn matches_independent_2048_bit_vector() {
    // Python 3.14's exact built-in `pow(signature, 65537, modulus)` produced
    // this fixed cross-implementation vector. The patterned inputs exercise
    // every byte position without importing a second bignum implementation.
    let modulus = decode_hex(concat!(
        "a972bb044d96df2871ba034c95de2770b9024b94dd266fb8014a93dc256eb700",
        "4992db246db6ff4891da236cb5fe4790d9226bb4fd468fd8216ab3fc458ed720",
        "69b2fb448dd61f68b1fa438cd51e67b0f9428bd41d66aff8418ad31c65aef740",
        "89d21b64adf63f88d11a63acf53e87d01962abf43d86cf1861aaf33c85ce176",
        "0a9f23b84cd165fa8f13a83cc155ea7f03982cb145da6ef3881ca135ca5ee378",
        "0c9125ba4ed367fc8115aa3ec357ec71059a2eb347dc60f58a1ea337cc50e57a",
        "0e9327bc40d569fe8317ac30c559ee73079c20b549de62f78c10a539ce52e77c",
        "009529be42d76bf08519ae32c75be075099e22b74bd064f98e12a73bc054e97e1",
    ));
    let signature = decode_hex(concat!(
        "0724415e7b98b5d2ef0c294663809dbad7f4112e4b6885a2bfdcf91633506d8a",
        "a7c4e1fe1b3855728facc9e603203d5a7794b1ceeb0825425f7c99b6d3f00d2a",
        "4764819ebbd8f5122f4c6986a3c0ddfa1734516e8ba8c5e2ff1c39567390adca",
        "e704213e5b7895b2cfec092643607d9ab7d4f10e2b4865829fbcd9f613304d6a",
        "87a4c1defb1835526f8ca9c6e3001d3a577491aecbe805223f5c7996b3d0ed0a",
        "2744617e9bb8d5f20f2c496683a0bddaf714314e6b88a5c2dffc193653708daa",
        "c7e4011e3b587592afcce90623405d7a97b4d1ee0b2845627f9cb9d6f3102d4",
        "a6784a1bedbf815324f6c89a6c3e0fd1a3754718eabc8e5021f3c597693b0cdea",
    ));
    let expected = decode_hex(concat!(
        "846f8e85cbec947a8684ed523b10877c9b1efd1d61c96eee79276afdde6de657",
        "3b22e7003a2126ba5086171c11b24290bf18e321f3b03c04be161ee4d543fd0c",
        "8ae5610890ef9b1c77af58f4ea478e089ae7e3a8a5cd49c1f5cf2b90e9e9853",
        "4875a02aac5c443a4aae63c63d413ee9c44fed6b9005ded056abe60c4f88c413",
        "a70d34e21802310be06d1141b3c973f565239790ecf7e8d816d291b2dcce0a43",
        "ea4373f7c5cde5abc29725f6a428b87acf9a42fa7ce9df1240c733ed0cf5502e",
        "09de03e0ead764fad5a2d87c8ccc4e4b70c60c19b01ce9443eecf19c8c95b59",
        "1c55a24d1844f1b64b5f4f66de48007a186cbfc85739bc5809bf2c361bc5e9ff6d",
    ));
    let encoded = spki(&modulus, 65_537);
    let key = validated_key(&encoded, &[65_537]);
    let recovered =
        recover_encoded_message(key, &signature, RsaPublicOperationLimits::new(256).unwrap())
            .unwrap();
    assert_eq!(recovered, expected);
}

#[test]
fn rejects_modulus_above_operational_limit_before_signature_checks() {
    let modulus = decode_hex("d5bbb96d30086ec484eba3d7f9caeb07a55e650837da1e731b1b4d8f6f4e9a13");
    let encoded = spki(&modulus, 65_537);
    let key = validated_key(&encoded, &[65_537]);
    let error =
        recover_encoded_message(key, &[], RsaPublicOperationLimits::new(31).unwrap()).unwrap_err();
    assert_eq!(error.kind(), RsaPublicOperationErrorKind::ModulusTooLong);
}

#[test]
fn rejects_short_and_long_signatures_without_retaining_lengths_or_bytes() {
    let encoded = spki(&[0x0c, 0xa1], 17);
    let key = validated_key(&encoded, &[17]);
    let limits = RsaPublicOperationLimits::new(2).unwrap();

    for signature in [vec![0xa1], vec![0, 0x0a, 0xe6]] {
        let error = recover_encoded_message(key, &signature, limits).unwrap_err();
        assert_eq!(
            error.kind(),
            RsaPublicOperationErrorKind::SignatureLengthMismatch
        );
        let rendered = error.to_string();
        assert!(!rendered.contains("3233"));
        assert!(!rendered.contains("2790"));
    }
}

#[test]
fn rejects_signature_representatives_equal_to_or_greater_than_modulus() {
    let encoded = spki(&[0x0c, 0xa1], 17);
    let key = validated_key(&encoded, &[17]);
    let limits = RsaPublicOperationLimits::new(2).unwrap();

    for signature in [[0x0c, 0xa1], [0x0c, 0xa2], [0xff, 0xff]] {
        let error = recover_encoded_message(key, &signature, limits).unwrap_err();
        assert_eq!(
            error.kind(),
            RsaPublicOperationErrorKind::SignatureRepresentativeOutOfRange
        );
    }
}

#[test]
fn source_denies_unsafe_and_exposes_no_private_rsa_operation() {
    let source = include_str!("../src/lib.rs");
    assert!(source.contains("#![deny(unsafe_code)]"));
    for forbidden in [
        "pub fn private_exponent",
        "pub fn decrypt",
        "fn decrypt(",
        "pub fn sign",
        "fn sign(",
    ] {
        assert!(
            !source.contains(forbidden),
            "found forbidden API: {forbidden}"
        );
    }
}
