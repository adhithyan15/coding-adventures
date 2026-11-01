use rsa_pss_encoded_message_layout::{
    RsaPssEncodedMessageLayoutErrorKind, RsaPssEncodedMessageLayoutPolicyErrorKind,
};
use rsa_pss_mgf1_sha512::Mgf1Sha512LimitsErrorKind;
use rsa_pss_sha512::{
    validate_rsa_pss_sha512_encoded_message, RsaPssSha512ErrorKind, RsaPssSha512Limits,
    RsaPssSha512LimitsErrorKind,
};

const MESSAGE_DIGEST: [u8; 64] = [
    0xdd, 0xaf, 0x35, 0xa1, 0x93, 0x61, 0x7a, 0xba, 0xcc, 0x41, 0x73, 0x49, 0xae, 0x20, 0x41, 0x31,
    0x12, 0xe6, 0xfa, 0x4e, 0x89, 0xa9, 0x7e, 0xa2, 0x0a, 0x9e, 0xee, 0xe6, 0x4b, 0x55, 0xd3, 0x9a,
    0x21, 0x92, 0x99, 0x2a, 0x27, 0x4f, 0xc1, 0xa8, 0x36, 0xba, 0x3c, 0x23, 0xa3, 0xfe, 0xeb, 0xbd,
    0x45, 0x4d, 0x44, 0x23, 0x64, 0x3c, 0xe8, 0x0e, 0x2a, 0x9a, 0xc9, 0x4f, 0xa5, 0x4c, 0xa4, 0x9f,
];

// Independently generated from RFC 8017's algorithm using Python hashlib:
// mHash = SHA-512("abc"), salt = 00..3f, emBits = 2047, emLen = 256.
// SHA-512(EM) = 9279f1d6241807d80035d2b3150e031a7e0e8be635673806d351dda43f0db5a6
//               fbdc05729dc964693baf9582e3ebd369f90dce40b345e269ce7b56dc0fece522.
const ENCODED_MESSAGE_HEX: &str = concat!(
    "62d8cce9dc62fdd7ffedc682ec673896e5443defb2969ffda43b97af3875b6f4",
    "e349ceef25a40dd136d4c71a048bab5b309ddfb75f849818f96bb136ba887fc0",
    "b914d5afd444fb4d2eb981876f5b107ef024fb7fe4c870723600fbf9d0c32ed",
    "43230930c3f1b209588aa432d417b736ed49bff0661b2ff7a8b8722289b2e64",
    "abcfa8d76f3764396b8648788ccf4f661eee67507a5955c53936ccde89798917",
    "159c7bf5ccaa6fc6e5bac3e3a63dcd365efdba4219c28dd886ef0ace9d5e563",
    "f0c3c780980a3192fbe6e8b29b7e02ec9a30c82e23e0dbb31c9f42fd6ed233",
    "2cf14cd5eb228386c4e009bb7688026611ddb171d20a91d5b474416492c83b01",
    "c00bc",
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

fn encoded_message() -> Vec<u8> {
    decode_hex(ENCODED_MESSAGE_HEX)
}

fn limits() -> RsaPssSha512Limits {
    RsaPssSha512Limits::new(256).unwrap()
}

#[test]
fn validates_independent_sha512_pss_vector_and_complete_data_block() {
    let encoded = encoded_message();
    let mut scratch = [0xa5; 191];

    let validated = validate_rsa_pss_sha512_encoded_message(
        &encoded,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap();

    assert_eq!(
        format!("{validated:?}"),
        "ValidatedRsaPssSha512EncodedMessage"
    );
    assert_eq!(&scratch[..126], &[0u8; 126]);
    assert_eq!(scratch[126], 0x01);
    assert_eq!(&scratch[127..], &(0u8..64).collect::<Vec<_>>());
    assert_eq!(limits().maximum_encoded_message_bytes(), 256);
}

#[test]
fn rejects_layout_and_scratch_lengths_before_scratch_mutation() {
    let mut wrong_trailer = encoded_message();
    *wrong_trailer.last_mut().unwrap() = 0xbd;
    let mut scratch = [0xa5; 191];
    let error = validate_rsa_pss_sha512_encoded_message(
        &wrong_trailer,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha512ErrorKind::Layout(RsaPssEncodedMessageLayoutErrorKind::InvalidTrailerField)
    );
    assert_eq!(scratch, [0xa5; 191]);

    let mut short_scratch = [0x5a; 190];
    let error = validate_rsa_pss_sha512_encoded_message(
        &encoded_message(),
        2047,
        &MESSAGE_DIGEST,
        &mut short_scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha512ErrorKind::DataBlockScratchLengthMismatch
    );
    assert_eq!(short_scratch, [0x5a; 190]);
}

#[test]
fn rejects_forbidden_high_bits_padding_delimiter_and_hash_mismatch() {
    let mut scratch = [0u8; 191];
    let mut high_bit = encoded_message();
    high_bit[0] |= 0x80;
    let error = validate_rsa_pss_sha512_encoded_message(
        &high_bit,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha512ErrorKind::Layout(RsaPssEncodedMessageLayoutErrorKind::NonzeroUnusedHighBits)
    );

    let mut invalid_padding = encoded_message();
    invalid_padding[0] ^= 0x01;
    let error = validate_rsa_pss_sha512_encoded_message(
        &invalid_padding,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), RsaPssSha512ErrorKind::InvalidPadding);

    let mut invalid_delimiter = encoded_message();
    invalid_delimiter[126] ^= 0x01;
    let error = validate_rsa_pss_sha512_encoded_message(
        &invalid_delimiter,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), RsaPssSha512ErrorKind::InvalidDelimiter);

    let mut wrong_digest = MESSAGE_DIGEST;
    wrong_digest[63] ^= 0x01;
    let error = validate_rsa_pss_sha512_encoded_message(
        &encoded_message(),
        2047,
        &wrong_digest,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), RsaPssSha512ErrorKind::HashMismatch);
}

#[test]
fn validates_limits_and_redacts_all_failures() {
    let too_small = RsaPssSha512Limits::new(129).unwrap_err();
    assert_eq!(
        too_small.kind(),
        RsaPssSha512LimitsErrorKind::Layout(
            RsaPssEncodedMessageLayoutPolicyErrorKind::MaximumEncodedMessageTooSmall
        )
    );

    if usize::BITS > 32 {
        let too_large = RsaPssSha512Limits::new(usize::MAX).unwrap_err();
        assert_eq!(
            too_large.kind(),
            RsaPssSha512LimitsErrorKind::MaskGeneration(
                Mgf1Sha512LimitsErrorKind::MaximumMaskTooLong
            )
        );
    }

    let mut scratch = [0u8; 191];
    let error = validate_rsa_pss_sha512_encoded_message(
        &encoded_message(),
        2047,
        &[0x7d; 64],
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    for rendered in [
        format!("{too_small:?}"),
        too_small.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("129"));
        assert!(!rendered.contains("0x7d"));
        assert!(!rendered.contains("191"));
    }
}
