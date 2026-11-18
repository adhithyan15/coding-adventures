use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_pss_sha512::{RsaPssSha512ErrorKind, RsaPssSha512LimitsErrorKind};
use rsa_pss_sha512_signature::{
    verify_rsa_pss_sha512_signature, RsaPssSha512SignatureErrorKind, RsaPssSha512SignatureLimits,
    RsaPssSha512SignatureLimitsErrorKind,
};
use rsa_public_operation::{RsaPublicOperationErrorKind, RsaPublicOperationLimitsErrorKind};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

const MESSAGE_DIGEST: [u8; 64] = [
    0xdd, 0xaf, 0x35, 0xa1, 0x93, 0x61, 0x7a, 0xba, 0xcc, 0x41, 0x73, 0x49, 0xae, 0x20, 0x41, 0x31,
    0x12, 0xe6, 0xfa, 0x4e, 0x89, 0xa9, 0x7e, 0xa2, 0x0a, 0x9e, 0xee, 0xe6, 0x4b, 0x55, 0xd3, 0x9a,
    0x21, 0x92, 0x99, 0x2a, 0x27, 0x4f, 0xc1, 0xa8, 0x36, 0xba, 0x3c, 0x23, 0xa3, 0xfe, 0xeb, 0xbd,
    0x45, 0x4d, 0x44, 0x23, 0x64, 0x3c, 0xe8, 0x0e, 0x2a, 0x9a, 0xc9, 0x4f, 0xa5, 0x4c, 0xa4, 0x9f,
];

// Generated and independently verified with OpenSSL 3.6.3:
//   printf abc | openssl dgst -sha512 -sign key.pem \
//     -sigopt rsa_padding_mode:pss -sigopt rsa_pss_saltlen:64 \
//     -sigopt rsa_mgf1_md:sha512
// Public SPKI SHA-256: 3cfdd6d9b4c303d699050a010575b4cfc8f89fdc1a41a21b063c4b18a9358e05
// Signature SHA-256:   afff6f4c4835d9218b0ec3639c146f430bb56edc21733896b748adaaa2d01f71
const PUBLIC_KEY_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a0282010100",
    "bde0b2abcbbc66461d6e51bb3ece6f6cd7b73a1bcb3dba7821da304d53971642",
    "22381f9e8f2f22a6462deafbfffc986563341f4f0001a8817a5068a80244f43c",
    "796cbd3e0d399673b085ff0762be16d8efba264f072ce155b282cb9858c46caf",
    "c62fc54a6f810496377a6727857d444d9d89ee4ad046d3c48d208b6c73a950ac",
    "537a79c4225dd2bdfe160d99a52fbbc46daf0d566a77760a1c68eab100e1f239",
    "187a54ab977f210bb0e841d282532100ccaf37fd2e49be355be0f14eeb67bf5b",
    "bccbbc8ee623e7e02617eec9e2f9c7cd36729822cf6ec7104dcc67765c024e1a",
    "6492a650a5e61f471e3efa5e74b9c09a4f0c82304c14c190616387b0ddfb7f87",
    "0203010001",
);

const SIGNATURE_HEX: &str = concat!(
    "154ea21898e84ffb4fbf03053e7bb2d18a2cd18ac068811faca6a553ce2b1f35",
    "52b5bfe3d75bf97fdff3216e9016c8f116b8646886d9e3a6c2e8ff277f4ba251",
    "1ba8c3ebafe8ac3cac0538be9dceef1c1c1d3a87ba2646d013dcfc152ad1ef5b",
    "10f5d190bddd3db4d175db239e91603d60323bb259e376c454113c134c6b7fe2",
    "e1909ad54540ce2fcb1dc3ebded6249c6d88f1b068b71fb7ab94f29f2bbc43fd",
    "cac67dd6760c821de2a9e7858492b99fbb96c95c8d446de5e109965012f0890f",
    "2adf636891e8f40b428de083b3f018f84d18a4240fbc18ee008d23eb7134fd7",
    "aff3795d68f25873707b333f0346f50464ad2316f719ae28fadeec45d91fb4275",
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

fn limits() -> RsaPssSha512SignatureLimits {
    RsaPssSha512SignatureLimits::new(256, 256).unwrap()
}

#[test]
fn verifies_independent_signature_and_binds_exact_inputs() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);
    let mut scratch = [0xa5; 191];

    let verified =
        verify_rsa_pss_sha512_signature(key, &signature, &MESSAGE_DIGEST, &mut scratch, limits())
            .unwrap();

    assert_eq!(verified.public_key().modulus(), key.modulus());
    assert_eq!(verified.signature(), signature);
    assert_eq!(verified.message_digest(), &MESSAGE_DIGEST);
    assert_eq!(format!("{verified:?}"), "VerifiedRsaPssSha512Signature");
    assert_eq!(&scratch[..126], &[0u8; 126]);
    assert_eq!(scratch[126], 0x01);
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
    let mut scratch = [0xa5; 191];

    let error = verify_rsa_pss_sha512_signature(
        key,
        &signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        RsaPssSha512SignatureLimits::new(257, 256).unwrap(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        RsaPssSha512SignatureErrorKind::NonzeroEncodedMessagePrefix
    );
    assert_eq!(scratch, [0xa5; 191]);
}

#[test]
fn rejects_public_operation_failures_before_scratch_mutation() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);

    let mut scratch = [0xa5; 191];
    let short = verify_rsa_pss_sha512_signature(
        key,
        &signature[..255],
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        short.kind(),
        RsaPssSha512SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::SignatureLengthMismatch
        )
    );
    assert_eq!(scratch, [0xa5; 191]);

    let small_limits = RsaPssSha512SignatureLimits::new(255, 256).unwrap();
    let too_large = verify_rsa_pss_sha512_signature(
        key,
        &signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        small_limits,
    )
    .unwrap_err();
    assert_eq!(
        too_large.kind(),
        RsaPssSha512SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::ModulusTooLong
        )
    );
    assert_eq!(scratch, [0xa5; 191]);
}

#[test]
fn rejects_encoded_message_and_digest_failures() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);
    let mut scratch = [0u8; 191];

    let mut tampered_signature = signature.clone();
    tampered_signature[255] ^= 0x01;
    let tampered = verify_rsa_pss_sha512_signature(
        key,
        &tampered_signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert!(matches!(
        tampered.kind(),
        RsaPssSha512SignatureErrorKind::EncodedMessage(_)
    ));

    let mut wrong_digest = MESSAGE_DIGEST;
    wrong_digest[63] ^= 0x01;
    let mismatch =
        verify_rsa_pss_sha512_signature(key, &signature, &wrong_digest, &mut scratch, limits())
            .unwrap_err();
    assert_eq!(
        mismatch.kind(),
        RsaPssSha512SignatureErrorKind::EncodedMessage(RsaPssSha512ErrorKind::HashMismatch)
    );
}

#[test]
fn rejects_wrong_scratch_length_before_scratch_mutation() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);
    let mut scratch = [0xa5; 190];

    let error =
        verify_rsa_pss_sha512_signature(key, &signature, &MESSAGE_DIGEST, &mut scratch, limits())
            .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha512SignatureErrorKind::EncodedMessage(
            RsaPssSha512ErrorKind::DataBlockScratchLengthMismatch
        )
    );
    assert_eq!(scratch, [0xa5; 190]);
}

#[test]
fn validates_limits_and_redacts_failures() {
    let zero_modulus = RsaPssSha512SignatureLimits::new(0, 256).unwrap_err();
    assert_eq!(
        zero_modulus.kind(),
        RsaPssSha512SignatureLimitsErrorKind::PublicOperation(
            RsaPublicOperationLimitsErrorKind::ZeroMaximumModulusBytes
        )
    );
    let small_message = RsaPssSha512SignatureLimits::new(256, 129).unwrap_err();
    assert_eq!(
        small_message.kind(),
        RsaPssSha512SignatureLimitsErrorKind::EncodedMessage(
            RsaPssSha512LimitsErrorKind::Layout(
                rsa_pss_encoded_message_layout::RsaPssEncodedMessageLayoutPolicyErrorKind::MaximumEncodedMessageTooSmall
            )
        )
    );

    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let mut scratch = [0xa5; 191];
    let error =
        verify_rsa_pss_sha512_signature(key, &[0x7d; 255], &MESSAGE_DIGEST, &mut scratch, limits())
            .unwrap_err();
    assert_eq!(
        format!("{zero_modulus:?}"),
        "RsaPssSha512SignatureLimitsError { kind: PublicOperation(ZeroMaximumModulusBytes) }"
    );
    assert_eq!(
        format!("{small_message:?}"),
        "RsaPssSha512SignatureLimitsError { kind: EncodedMessage(Layout(MaximumEncodedMessageTooSmall)) }"
    );
    assert_eq!(
        format!("{error:?}"),
        "RsaPssSha512SignatureError { kind: PublicOperation(SignatureLengthMismatch) }"
    );
    for rendered in [
        format!("{zero_modulus:?}"),
        zero_modulus.to_string(),
        format!("{small_message:?}"),
        small_message.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("191"));
        assert!(!rendered.contains("255"));
        assert!(!rendered.contains("129"));
        assert!(!rendered.contains("0x7d"));
    }
}
