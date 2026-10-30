use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_pss_sha384::{RsaPssSha384ErrorKind, RsaPssSha384LimitsErrorKind};
use rsa_pss_sha384_signature::{
    verify_rsa_pss_sha384_signature, RsaPssSha384SignatureErrorKind, RsaPssSha384SignatureLimits,
    RsaPssSha384SignatureLimitsErrorKind,
};
use rsa_public_operation::{RsaPublicOperationErrorKind, RsaPublicOperationLimitsErrorKind};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

const MESSAGE_DIGEST: [u8; 48] = [
    0xcb, 0x00, 0x75, 0x3f, 0x45, 0xa3, 0x5e, 0x8b, 0xb5, 0xa0, 0x3d, 0x69, 0x9a, 0xc6, 0x50, 0x07,
    0x27, 0x2c, 0x32, 0xab, 0x0e, 0xde, 0xd1, 0x63, 0x1a, 0x8b, 0x60, 0x5a, 0x43, 0xff, 0x5b, 0xed,
    0x80, 0x86, 0x07, 0x2b, 0xa1, 0xe7, 0xcc, 0x23, 0x58, 0xba, 0xec, 0xa1, 0x34, 0xc8, 0x25, 0xa7,
];

// Generated and independently verified with OpenSSL 3.6.3:
//   printf abc | openssl dgst -sha384 -sign key.pem \
//     -sigopt rsa_padding_mode:pss -sigopt rsa_pss_saltlen:48 \
//     -sigopt rsa_mgf1_md:sha384
// Public SPKI SHA-256: ddaff9ef5e680b024ec629063ce762e9450f7189ac0c624cd24c8105eee1e3f9
// Signature SHA-256:   aaec33e44b5d1eaaa5522d584b835bb59af517ddada524c14666c7180522b592
const PUBLIC_KEY_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a0282010100",
    "8e19de3e4510adc9a157c077f1be08a32c351b3e65a47a1f4601f325900aec55",
    "d9a7911b3c26d6d9b47ec8e789491d228e71cc7559ecc5b23e9a50345f72f02a",
    "17271c63acd3a577f9120b97cdffc3f0254bfe01024519096f763db3bbb28e79",
    "094bcc46e184b46799ec49d5309081d37603b440fce446348a829ad3ced082bc",
    "cff16e8371e4bbc3858e25ef437356a30ff7fc6c14d9ac512c1483571d30271c",
    "18702de4b6b78d9b6ad2f5a4ba581631997fa83b6ba4053d3fb07ce10897b776",
    "093c282f84da0e08674008c2cc00d1da85d39c3c66e8c1abf447644a456388ae",
    "c718b6889915cc1afd1058b78fd0eeee35fd0fb0196c7473438f6aa6d12547af",
    "0203010001",
);

const SIGNATURE_HEX: &str = concat!(
    "2638a63dcf02877f80d58ecd0e7e0bd0668d2afd4c6deb0e4e9d1ed593ee570f",
    "7de7e0a3738ff7fcf429beca5eadbe530efc34bfea5ee83ce62620e80799b7e0",
    "e854f1c08ba65518d76ece5416253cb203909eb758351d3e6ada5c624fbef7a6",
    "5804f0b74808f6aba3831afd8d4668726d141ef96eac1556b5a3b064068882cf",
    "98dc0e57f43be91841281588d52e077012aeaa0bc32dd6542bf24bfdebe6b887",
    "79939d0356f5b24e18d4a5d44427a84c31cc4e78cf717b28ea08141f911db9e9",
    "25d62162c042b5b448f24a06b2f91776e0fb6fed9d0fcaba629c9b35ec8f1cc6",
    "757e71a2582ed90f21e4fa20c23e21f8cdf793b19593a3a0744b70343805802a",
);

// Deterministically generated from two probable primes, then independently
// checked with Python's exact modular exponentiation. The 2,049-bit modulus
// makes RSAVP1 return 257 octets while RFC 8017 PSS uses a 256-octet EM.
// Modulus SHA-256:   b51bfccfb0a68f1b39249d421dc2ee060acebc298bf44427ea7326c6df88586c
// Signature SHA-256: ad157d2f8cff8db3c6bce6fe88cbe56313c9a2952667a45031ab3636ee17be24
// EM SHA-256:        ebd716454c9f50e9757b7805b8b156455e6052128da6c6a01e23977a531b2e37
const ODD_WIDTH_MODULUS_HEX: &str = concat!(
    "01639872eec01d14bed52fbc0272b9640a022e29e111b5fc2b3255b0e9b45082",
    "01d5e3a40ebac09de9f2850afcc898b86fb20db478fdd62cb9bb3fe2b1ae6b89",
    "8e5821957e25e8b642db8201358f17eeaf4d81b9fafd5b4c230c66a09d7a6402",
    "44d8515416d510931b95f0e0a9128ecefa902912b94d4cda8cf1e731fd91e90f",
    "c03ef5ea7bfa30aadb2a042cadff6ac2504e766a7adff1f78a3c63a285702782",
    "e69b08bbe1f76d5406d1bda99ef3db8e85be7d7fce7e88593cfa54fac77aeb65",
    "b961ef319f57ce5a7f2123b34fd62f2f5295c855a83dd0a87a76d32851d3ddee",
    "297955b78cc6a02579c6c747705b55ba787db722a25c108cf72d48caf296812fd1",
);

const ODD_WIDTH_SIGNATURE_HEX: &str = concat!(
    "005ffa4066671c9f358c3da7ef1c3c37e3d8cb63c715037737b9bd6b980a14f3",
    "230601cf8d49b69d5a265665b885cb9aef0864417cfaded54a96d501e5254d7b",
    "3c331567e129790d26422b48c81a049506bb9674a55df3304bf06435e3ba374c",
    "a14600420892977e5965f7ba039c38a903aed555795865c27ad01eaf889d90c5",
    "d21a7e2b254f705711826d4307596e24bd86fff3d92efc2557022d0bf5ef034d",
    "1b43104c9943a9944d54a8537c8c3061050dff7d8ce042509839096d98a0ef1a",
    "c146c73c9f609d38ea4685cf83a8c947116c546c3766266df9f816a3636d7b78",
    "60019a8dc6fef156ed2bd66e39e88f23b89a6b47f58af5c984f3010cee9ce17ddf",
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

fn spki(modulus: &[u8]) -> Vec<u8> {
    let exponent = positive_integer(&65_537u64.to_be_bytes()[5..]);
    let key = sequence(&[positive_integer(modulus), exponent]);
    let algorithm = sequence(&[tlv(0x06, RSA_ENCRYPTION_OID_DER), vec![0x05, 0x00]]);
    let mut bit_string = vec![0];
    bit_string.extend_from_slice(&key);
    sequence(&[algorithm, tlv(0x03, &bit_string)])
}

fn validated_key(encoded: &[u8], modulus_bits: usize) -> ValidatedRsaPublicKey<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    let subject_public_key_info = decode_x509_subject_public_key_info(&mut decoder, root).unwrap();
    let policy = RsaPublicKeyPolicy::new(modulus_bits, modulus_bits, &[65_537]).unwrap();
    validate_rsa_public_key(subject_public_key_info, policy, Asn1Limits::default()).unwrap()
}

fn limits() -> RsaPssSha384SignatureLimits {
    RsaPssSha384SignatureLimits::new(256, 256).unwrap()
}

#[test]
fn verifies_independent_signature_and_binds_exact_inputs() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);
    let mut scratch = [0xa5; 207];

    let verified =
        verify_rsa_pss_sha384_signature(key, &signature, &MESSAGE_DIGEST, &mut scratch, limits())
            .unwrap();

    assert_eq!(verified.public_key().modulus(), key.modulus());
    assert_eq!(verified.signature(), signature);
    assert_eq!(verified.message_digest(), &MESSAGE_DIGEST);
    assert_eq!(format!("{verified:?}"), "VerifiedRsaPssSha384Signature");
    assert_eq!(&scratch[..158], &[0u8; 158]);
    assert_eq!(scratch[158], 0x01);
}

#[test]
fn verifies_modulus_width_one_byte_larger_than_encoded_message() {
    let modulus = decode_hex(ODD_WIDTH_MODULUS_HEX);
    let encoded_key = spki(&modulus);
    let key = validated_key(&encoded_key, 2049);
    let signature = decode_hex(ODD_WIDTH_SIGNATURE_HEX);
    let mut scratch = [0xa5; 207];

    let verified = verify_rsa_pss_sha384_signature(
        key,
        &signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        RsaPssSha384SignatureLimits::new(257, 256).unwrap(),
    )
    .unwrap();

    assert_eq!(verified.public_key().modulus_bits(), 2049);
    assert_eq!(verified.signature(), signature);
    assert_eq!(&scratch[..158], &[0u8; 158]);
    assert_eq!(scratch[158], 0x01);
}

#[test]
fn rejects_nonzero_modulus_width_prefix_before_scratch_mutation() {
    let modulus = decode_hex(ODD_WIDTH_MODULUS_HEX);
    let encoded_key = spki(&modulus);
    let key = validated_key(&encoded_key, 2049);
    let mut signature = modulus;
    for octet in signature.iter_mut().rev() {
        if *octet != 0 {
            *octet -= 1;
            break;
        }
        *octet = 0xff;
    }
    let mut scratch = [0xa5; 207];

    let error = verify_rsa_pss_sha384_signature(
        key,
        &signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        RsaPssSha384SignatureLimits::new(257, 256).unwrap(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        RsaPssSha384SignatureErrorKind::NonzeroEncodedMessagePrefix
    );
    assert_eq!(scratch, [0xa5; 207]);
}

#[test]
fn rejects_public_operation_failures_before_scratch_mutation() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);

    let mut scratch = [0xa5; 207];
    let short = verify_rsa_pss_sha384_signature(
        key,
        &signature[..255],
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        short.kind(),
        RsaPssSha384SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::SignatureLengthMismatch
        )
    );
    assert_eq!(scratch, [0xa5; 207]);

    let small_limits = RsaPssSha384SignatureLimits::new(255, 256).unwrap();
    let too_large = verify_rsa_pss_sha384_signature(
        key,
        &signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        small_limits,
    )
    .unwrap_err();
    assert_eq!(
        too_large.kind(),
        RsaPssSha384SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::ModulusTooLong
        )
    );
    assert_eq!(scratch, [0xa5; 207]);
}

#[test]
fn rejects_encoded_message_and_digest_failures() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);
    let mut scratch = [0u8; 207];

    let mut tampered_signature = signature.clone();
    tampered_signature[255] ^= 0x01;
    let tampered = verify_rsa_pss_sha384_signature(
        key,
        &tampered_signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert!(matches!(
        tampered.kind(),
        RsaPssSha384SignatureErrorKind::EncodedMessage(_)
    ));

    let mut wrong_digest = MESSAGE_DIGEST;
    wrong_digest[47] ^= 0x01;
    let mismatch =
        verify_rsa_pss_sha384_signature(key, &signature, &wrong_digest, &mut scratch, limits())
            .unwrap_err();
    assert_eq!(
        mismatch.kind(),
        RsaPssSha384SignatureErrorKind::EncodedMessage(RsaPssSha384ErrorKind::HashMismatch)
    );
}

#[test]
fn rejects_wrong_scratch_length_before_scratch_mutation() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);
    let mut scratch = [0xa5; 206];

    let error =
        verify_rsa_pss_sha384_signature(key, &signature, &MESSAGE_DIGEST, &mut scratch, limits())
            .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha384SignatureErrorKind::EncodedMessage(
            RsaPssSha384ErrorKind::DataBlockScratchLengthMismatch
        )
    );
    assert_eq!(scratch, [0xa5; 206]);
}

#[test]
fn validates_limits_and_redacts_failures() {
    let zero_modulus = RsaPssSha384SignatureLimits::new(0, 256).unwrap_err();
    assert_eq!(
        zero_modulus.kind(),
        RsaPssSha384SignatureLimitsErrorKind::PublicOperation(
            RsaPublicOperationLimitsErrorKind::ZeroMaximumModulusBytes
        )
    );
    let small_message = RsaPssSha384SignatureLimits::new(256, 97).unwrap_err();
    assert_eq!(
        small_message.kind(),
        RsaPssSha384SignatureLimitsErrorKind::EncodedMessage(
            RsaPssSha384LimitsErrorKind::Layout(
                rsa_pss_encoded_message_layout::RsaPssEncodedMessageLayoutPolicyErrorKind::MaximumEncodedMessageTooSmall
            )
        )
    );

    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let mut scratch = [0xa5; 207];
    let error =
        verify_rsa_pss_sha384_signature(key, &[0x7d; 255], &MESSAGE_DIGEST, &mut scratch, limits())
            .unwrap_err();
    assert_eq!(
        format!("{zero_modulus:?}"),
        "RsaPssSha384SignatureLimitsError { kind: PublicOperation(ZeroMaximumModulusBytes) }"
    );
    assert_eq!(
        format!("{small_message:?}"),
        "RsaPssSha384SignatureLimitsError { kind: EncodedMessage(Layout(MaximumEncodedMessageTooSmall)) }"
    );
    assert_eq!(
        format!("{error:?}"),
        "RsaPssSha384SignatureError { kind: PublicOperation(SignatureLengthMismatch) }"
    );
    for rendered in [
        format!("{zero_modulus:?}"),
        zero_modulus.to_string(),
        format!("{small_message:?}"),
        small_message.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("207"));
        assert!(!rendered.contains("255"));
        assert!(!rendered.contains("97"));
        assert!(!rendered.contains("0x7d"));
    }
}
