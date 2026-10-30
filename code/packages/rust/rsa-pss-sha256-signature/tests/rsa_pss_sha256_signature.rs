use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_pss_sha256::{RsaPssSha256ErrorKind, RsaPssSha256LimitsErrorKind};
use rsa_pss_sha256_signature::{
    verify_rsa_pss_sha256_signature, RsaPssSha256SignatureErrorKind, RsaPssSha256SignatureLimits,
    RsaPssSha256SignatureLimitsErrorKind,
};
use rsa_public_operation::{RsaPublicOperationErrorKind, RsaPublicOperationLimitsErrorKind};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

const MESSAGE_DIGEST: [u8; 32] = [
    0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
    0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
];

// Generated and independently verified with OpenSSL 3.6.3:
//   printf abc | openssl dgst -sha256 -sign key.pem \
//     -sigopt rsa_padding_mode:pss -sigopt rsa_pss_saltlen:32 \
//     -sigopt rsa_mgf1_md:sha256
// Public SPKI SHA-256: 7369fc50941bc8eef1d27359b9e2501c935523184376cbc18887b80982bbd80c
// Signature SHA-256:   da331678c307f61c3055f0023b3858440e3b935ac84753094b5d9dd7404870fb
const PUBLIC_KEY_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a0282010100",
    "ac96fb34646a306afec2a8545dcaaef78c069ca9bdc2f6fb66af02040351b113",
    "5ebd7c4b18b532e6a338c4fea764701d2acd04258c0dc898878ea009e9ae61d9",
    "dda6e2a28a905225c063a1f96abcab6e8b1148f4c135cd8b38e494afa17f264f",
    "c8ae075b22287ff98eed6eb293f1819a36f4aa0e5020ddb696dbd76f592af5b0",
    "f23d6f41b16998ba03d96f597d0da8370f12c4c443879c70eccf7b45893345f2",
    "9337a011b55d064eea3e0ad22762d83717502e65ef2163fa0437c7affe0be0b10",
    "e14f9fd92167c2122e9f644d39b4c0947985f51e457b19c1bceb366e0e2e289e",
    "a289b0ba47355ea9e85dd7288d2ac10e71639bfaad5688b8c1dfee024e8df870",
    "203010001",
);

const SIGNATURE_HEX: &str = concat!(
    "1812c438852ffeda56668b298f2258356d22391f2ad7db12770d08f47e57d73c",
    "9257615fa19f5d68243510ded463a48ddd9852a1de96182d24d8fa922bcc15e9",
    "e57c2a024c798395da5bf4c3bb60286bbe01642cbff13ef8c9fa15f56c633430",
    "5d47f20f31fd9af14b3bf08237d4061e7a836748756e0b76acaa30c40e4d8e0",
    "0e3956198a25d285e2e1953677cc5b04ee3ee53025b4441b41067c7d07974014",
    "7993394b185ebe60922469d7a183348aeb284d278482eb9cd8b4189fd2a15e5f",
    "1054dc4e32aa8493b787b0a243e7b19d2fe53ffa89609019fbc4a64285d65448",
    "856a49b501c105db93df05e661b43c6c71e3a5412cc7647c86eb20856f4e840f3",
);

// Deterministically generated from two probable primes, then independently
// checked with Python's exact modular exponentiation. The 2,049-bit modulus
// makes RSAVP1 return 257 octets while RFC 8017 PSS uses a 256-octet EM.
// Modulus SHA-256:   829d000725151f23c7e97615741d49e1b772c97bbb5e0bb3f33e222a20a6f3ed
// Signature SHA-256: 6b2986291a6fedca53bf2527ba5036ce119c6d62659b01cfece2b3d7f3466289
// EM SHA-256:        943a9713b79c1f8412971478372b475f61734faca8306f5e3eefe91a288abf83
const ODD_WIDTH_MODULUS_HEX: &str = concat!(
    "0169d393260bb466b9c9e41238e2d18d81edbb6ae3752fccd1fbf7a08c9f41a6",
    "909c231d2d5af9270de609153f793d97127d2e139441495ccef282776a32104a8",
    "80d3674b731f6481961dffdaa1d4982fb7b7c9a520cea7dabf5ec50a6baff895",
    "80d5f2b66c70a7d5facd36870dcc6377a9e50a8aed0dddbe646b06cc037ac31d",
    "38ce0bc3a34073cbe1de333047b6194e3742f5fa3e9b40deb20ab72ee04572b8",
    "1b40bbbc63559211e20fd7422b4a6cd2db6aec66123905d530ce21ecf10db7ae",
    "a3d81aefe45a96ff87e7deef38a4ef7331205b9fe9725c86bf5743ab28ed067c",
    "ea44ae606eeb0f4e9419e386df07de4d4fb822d7d43f8730deafefcf41b8801a9",
);

const ODD_WIDTH_SIGNATURE_HEX: &str = concat!(
    "005f87e840997c6ad47b82151e79246cf8e21539237eb48788df4ae035214aa07",
    "fc38647d307793f59458c37b054c7910588d2beb08083000bad7ac821ca0babea",
    "91055200fc44456ce04cf479ec9f508713293ffd1c5b0feae5110af0e77cc557c",
    "6a86eb4fe8798d3686b0c4bc875d3dd33d19a78f7fcd7e13eed2120d4a071cd",
    "77bb72d30d77e72099e1b526a343df64269b165a95f53d36834fecd9bbee4e1a",
    "dcc1241d9ee8e2fb823d9f9fe672142a885fe235fa119b80280cebfe6fd41990d",
    "10e71562f1040a3f18d266d44f89df8e45d0830a4743615361d4ff0bc3ab66cd",
    "8654481f3120a635c40d599f46c00e7fca2db3d7b9e7b55de7ccde65f52f417",
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

fn limits() -> RsaPssSha256SignatureLimits {
    RsaPssSha256SignatureLimits::new(256, 256).unwrap()
}

#[test]
fn verifies_independent_signature_and_binds_exact_inputs() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);
    let mut scratch = [0xa5; 223];

    let verified =
        verify_rsa_pss_sha256_signature(key, &signature, &MESSAGE_DIGEST, &mut scratch, limits())
            .unwrap();

    assert_eq!(verified.public_key().modulus(), key.modulus());
    assert_eq!(verified.signature(), signature);
    assert_eq!(verified.message_digest(), &MESSAGE_DIGEST);
    assert_eq!(format!("{verified:?}"), "VerifiedRsaPssSha256Signature");
    assert_eq!(&scratch[..190], &[0u8; 190]);
    assert_eq!(scratch[190], 0x01);
}

#[test]
fn verifies_modulus_width_one_byte_larger_than_encoded_message() {
    let modulus = decode_hex(ODD_WIDTH_MODULUS_HEX);
    let encoded_key = spki(&modulus);
    let key = validated_key(&encoded_key, 2049);
    let signature = decode_hex(ODD_WIDTH_SIGNATURE_HEX);
    let mut scratch = [0xa5; 223];

    let verified = verify_rsa_pss_sha256_signature(
        key,
        &signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        RsaPssSha256SignatureLimits::new(257, 256).unwrap(),
    )
    .unwrap();

    assert_eq!(verified.public_key().modulus_bits(), 2049);
    assert_eq!(verified.signature(), signature);
    assert_eq!(&scratch[..190], &[0u8; 190]);
    assert_eq!(scratch[190], 0x01);
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
    let mut scratch = [0xa5; 223];

    let error = verify_rsa_pss_sha256_signature(
        key,
        &signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        RsaPssSha256SignatureLimits::new(257, 256).unwrap(),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        RsaPssSha256SignatureErrorKind::NonzeroEncodedMessagePrefix
    );
    assert_eq!(scratch, [0xa5; 223]);
}

#[test]
fn rejects_public_operation_failures_before_scratch_mutation() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);

    let mut scratch = [0xa5; 223];
    let short = verify_rsa_pss_sha256_signature(
        key,
        &signature[..255],
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        short.kind(),
        RsaPssSha256SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::SignatureLengthMismatch
        )
    );
    assert_eq!(scratch, [0xa5; 223]);

    let small_limits = RsaPssSha256SignatureLimits::new(255, 256).unwrap();
    let too_large = verify_rsa_pss_sha256_signature(
        key,
        &signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        small_limits,
    )
    .unwrap_err();
    assert_eq!(
        too_large.kind(),
        RsaPssSha256SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::ModulusTooLong
        )
    );
    assert_eq!(scratch, [0xa5; 223]);
}

#[test]
fn rejects_encoded_message_and_digest_failures() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);
    let mut scratch = [0u8; 223];

    let mut tampered_signature = signature.clone();
    tampered_signature[255] ^= 0x01;
    let tampered = verify_rsa_pss_sha256_signature(
        key,
        &tampered_signature,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert!(matches!(
        tampered.kind(),
        RsaPssSha256SignatureErrorKind::EncodedMessage(_)
    ));

    let mut wrong_digest = MESSAGE_DIGEST;
    wrong_digest[31] ^= 0x01;
    let mismatch =
        verify_rsa_pss_sha256_signature(key, &signature, &wrong_digest, &mut scratch, limits())
            .unwrap_err();
    assert_eq!(
        mismatch.kind(),
        RsaPssSha256SignatureErrorKind::EncodedMessage(RsaPssSha256ErrorKind::HashMismatch)
    );
}

#[test]
fn rejects_wrong_scratch_length_before_scratch_mutation() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let signature = decode_hex(SIGNATURE_HEX);
    let mut scratch = [0xa5; 222];

    let error =
        verify_rsa_pss_sha256_signature(key, &signature, &MESSAGE_DIGEST, &mut scratch, limits())
            .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha256SignatureErrorKind::EncodedMessage(
            RsaPssSha256ErrorKind::DataBlockScratchLengthMismatch
        )
    );
    assert_eq!(scratch, [0xa5; 222]);
}

#[test]
fn validates_limits_and_redacts_failures() {
    let zero_modulus = RsaPssSha256SignatureLimits::new(0, 256).unwrap_err();
    assert_eq!(
        zero_modulus.kind(),
        RsaPssSha256SignatureLimitsErrorKind::PublicOperation(
            RsaPublicOperationLimitsErrorKind::ZeroMaximumModulusBytes
        )
    );
    let small_message = RsaPssSha256SignatureLimits::new(256, 65).unwrap_err();
    assert_eq!(
        small_message.kind(),
        RsaPssSha256SignatureLimitsErrorKind::EncodedMessage(
            RsaPssSha256LimitsErrorKind::Layout(
                rsa_pss_encoded_message_layout::RsaPssEncodedMessageLayoutPolicyErrorKind::MaximumEncodedMessageTooSmall
            )
        )
    );

    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key, 2048);
    let mut scratch = [0xa5; 223];
    let error =
        verify_rsa_pss_sha256_signature(key, &[0x7d; 255], &MESSAGE_DIGEST, &mut scratch, limits())
            .unwrap_err();
    assert_eq!(
        format!("{zero_modulus:?}"),
        "RsaPssSha256SignatureLimitsError { kind: PublicOperation(ZeroMaximumModulusBytes) }"
    );
    assert_eq!(
        format!("{small_message:?}"),
        "RsaPssSha256SignatureLimitsError { kind: EncodedMessage(Layout(MaximumEncodedMessageTooSmall)) }"
    );
    assert_eq!(
        format!("{error:?}"),
        "RsaPssSha256SignatureError { kind: PublicOperation(SignatureLengthMismatch) }"
    );
    for rendered in [
        format!("{zero_modulus:?}"),
        zero_modulus.to_string(),
        format!("{small_message:?}"),
        small_message.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("223"));
        assert!(!rendered.contains("255"));
        assert!(!rendered.contains("65"));
        assert!(!rendered.contains("0x7d"));
    }
}
