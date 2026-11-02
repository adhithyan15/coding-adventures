use rsa_pss_encoded_message_layout::{
    RsaPssEncodedMessageLayoutErrorKind, RsaPssEncodedMessageLayoutPolicyErrorKind,
};
use rsa_pss_mgf1_sha256::Mgf1Sha256LimitsErrorKind;
use rsa_pss_sha256::{
    validate_rsa_pss_sha256_encoded_message, RsaPssSha256ErrorKind, RsaPssSha256Limits,
    RsaPssSha256LimitsErrorKind,
};

const MESSAGE_DIGEST: [u8; 32] = [
    0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
    0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
];

// Independently generated from RFC 8017's algorithm using Python hashlib:
// mHash = SHA-256("abc"), salt = 00..1f, emBits = 2047, emLen = 256.
// SHA-256(EM) = 943a9713b79c1f8412971478372b475f61734faca8306f5e3eefe91a288abf83.
const ENCODED_MESSAGE_HEX: &str = concat!(
    "5461d181ca04b8245926d45f0d349a5190e90cb955e6df200175f91abb966d9d",
    "6b4de159d029dad206b022b150b8d077472dadf99581d5c8f084ec7bb467788e",
    "0ecccb6e2d9686a429b6cd0bcf3f58812cf1f409c5116aef41d291bbd898cf1",
    "cc63aee3bad8ea6cb0837e9589ca8a22500d84e3ef16aae917b98947fdc20079",
    "357066d7b69cacc8010afcedaf51955c9bc0c54274df63bdac3ed67749ca3e03",
    "5836f50dbcc3be18afb44024184bf5e70b62594a1edf416b6a721fe4e0907c2e",
    "b7f465712e45885c4c77b6a3570661a48fcce1ee75afa19008e25e5a032e43b",
    "f3fafa6580f36ad92f340631f3a4d18cc67aa34ad64f706021d6f1634d5b020e",
    "bc",
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

fn limits() -> RsaPssSha256Limits {
    RsaPssSha256Limits::new(256).unwrap()
}

#[test]
fn validates_independent_sha256_pss_vector_and_complete_data_block() {
    let encoded = encoded_message();
    let mut scratch = [0xa5; 223];

    let validated = validate_rsa_pss_sha256_encoded_message(
        &encoded,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap();

    assert_eq!(
        format!("{validated:?}"),
        "ValidatedRsaPssSha256EncodedMessage"
    );
    assert_eq!(&scratch[..190], &[0u8; 190]);
    assert_eq!(scratch[190], 0x01);
    assert_eq!(&scratch[191..], &(0u8..32).collect::<Vec<_>>());
    assert_eq!(limits().maximum_encoded_message_bytes(), 256);
}

#[test]
fn rejects_layout_and_scratch_lengths_before_scratch_mutation() {
    let mut wrong_trailer = encoded_message();
    *wrong_trailer.last_mut().unwrap() = 0xbd;
    let mut scratch = [0xa5; 223];
    let error = validate_rsa_pss_sha256_encoded_message(
        &wrong_trailer,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha256ErrorKind::Layout(RsaPssEncodedMessageLayoutErrorKind::InvalidTrailerField)
    );
    assert_eq!(scratch, [0xa5; 223]);

    let encoded = encoded_message();
    let mut short_scratch = [0x5a; 222];
    let error = validate_rsa_pss_sha256_encoded_message(
        &encoded,
        2047,
        &MESSAGE_DIGEST,
        &mut short_scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha256ErrorKind::DataBlockScratchLengthMismatch
    );
    assert_eq!(short_scratch, [0x5a; 222]);
}

#[test]
fn rejects_forbidden_high_bits_padding_delimiter_and_hash_mismatch() {
    let mut high_bit = encoded_message();
    high_bit[0] |= 0x80;
    let mut scratch = [0u8; 223];
    let error = validate_rsa_pss_sha256_encoded_message(
        &high_bit,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha256ErrorKind::Layout(RsaPssEncodedMessageLayoutErrorKind::NonzeroUnusedHighBits)
    );

    let mut invalid_padding = encoded_message();
    invalid_padding[0] ^= 0x01;
    let error = validate_rsa_pss_sha256_encoded_message(
        &invalid_padding,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), RsaPssSha256ErrorKind::InvalidPadding);

    let mut invalid_delimiter = encoded_message();
    invalid_delimiter[190] ^= 0x01;
    let error = validate_rsa_pss_sha256_encoded_message(
        &invalid_delimiter,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), RsaPssSha256ErrorKind::InvalidDelimiter);

    let mut wrong_digest = MESSAGE_DIGEST;
    wrong_digest[31] ^= 0x01;
    let error = validate_rsa_pss_sha256_encoded_message(
        &encoded_message(),
        2047,
        &wrong_digest,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), RsaPssSha256ErrorKind::HashMismatch);
}

#[test]
fn validates_limits_and_redacts_all_failures() {
    let too_small = RsaPssSha256Limits::new(65).unwrap_err();
    assert_eq!(
        too_small.kind(),
        RsaPssSha256LimitsErrorKind::Layout(
            RsaPssEncodedMessageLayoutPolicyErrorKind::MaximumEncodedMessageTooSmall
        )
    );

    if usize::BITS > 32 {
        let too_large = RsaPssSha256Limits::new(usize::MAX).unwrap_err();
        assert_eq!(
            too_large.kind(),
            RsaPssSha256LimitsErrorKind::MaskGeneration(
                Mgf1Sha256LimitsErrorKind::MaximumMaskTooLong
            )
        );
    }

    let mut scratch = [0u8; 223];
    let error = validate_rsa_pss_sha256_encoded_message(
        &encoded_message(),
        2047,
        &[0x7d; 32],
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
        assert!(!rendered.contains("65"));
        assert!(!rendered.contains("0x7d"));
        assert!(!rendered.contains("223"));
    }
}
