use rsa_pss_encoded_message_layout::{
    RsaPssEncodedMessageLayoutErrorKind, RsaPssEncodedMessageLayoutPolicyErrorKind,
};
use rsa_pss_mgf1_sha384::Mgf1Sha384LimitsErrorKind;
use rsa_pss_sha384::{
    validate_rsa_pss_sha384_encoded_message, RsaPssSha384ErrorKind, RsaPssSha384Limits,
    RsaPssSha384LimitsErrorKind,
};

const MESSAGE_DIGEST: [u8; 48] = [
    0xcb, 0x00, 0x75, 0x3f, 0x45, 0xa3, 0x5e, 0x8b, 0xb5, 0xa0, 0x3d, 0x69, 0x9a, 0xc6, 0x50, 0x07,
    0x27, 0x2c, 0x32, 0xab, 0x0e, 0xde, 0xd1, 0x63, 0x1a, 0x8b, 0x60, 0x5a, 0x43, 0xff, 0x5b, 0xed,
    0x80, 0x86, 0x07, 0x2b, 0xa1, 0xe7, 0xcc, 0x23, 0x58, 0xba, 0xec, 0xa1, 0x34, 0xc8, 0x25, 0xa7,
];

// Independently generated using Python hashlib:
// mHash = SHA-384("abc"), salt = 00..2f, emBits = 2047, emLen = 256.
// SHA-384(EM) = c1a1b4236090da92309a9ca918f819dec4f5d8e642cae4a59e81b85f44611e8443c8e1f4db2fd9a60c49e11cc70c1da2.
const ENCODED_MESSAGE_HEX: &str = concat!(
    "11ea0df527edf009b798116866fb7aa9becee15a4e58c82fea82404247377a97",
    "da309281d3550efae249d82871698c823b255a36baf6a83fbbc3d36969ad6704",
    "1950913562d0fcfc571cbe80cdada4e100944c41b778d43e653c50daa1f56723",
    "a9abec1acbe4edb96ea0cfb775d7218354b00e54259bc3146b5a405bbd92d360",
    "14afb99d115281e633d0fd08402003d529ebbf3e7be6f2de7d9b0de553bd0efa",
    "3b764aba0e2c0af3118322ebd5e4d277aa9bd1b5c255eb5526a588dd2b813437",
    "b7f5c9206bc60fcc7dd3dd1fcb168f61c118f86c5883b515add5653c185ae8c9",
    "2839ce2ce1ad562749b20a3be6d8538d8db8425ffb9df9df1be2bc5320fde6bc",
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
fn limits() -> RsaPssSha384Limits {
    RsaPssSha384Limits::new(256).unwrap()
}

#[test]
fn validates_independent_sha384_pss_vector_and_complete_data_block() {
    let encoded = encoded_message();
    let mut scratch = [0xa5; 207];
    let validated = validate_rsa_pss_sha384_encoded_message(
        &encoded,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap();

    assert_eq!(
        format!("{validated:?}"),
        "ValidatedRsaPssSha384EncodedMessage"
    );
    assert_eq!(&scratch[..158], &[0u8; 158]);
    assert_eq!(scratch[158], 0x01);
    assert_eq!(&scratch[159..], &(0u8..48).collect::<Vec<_>>());
    assert_eq!(limits().maximum_encoded_message_bytes(), 256);
}

#[test]
fn rejects_layout_and_scratch_lengths_before_scratch_mutation() {
    let mut wrong_trailer = encoded_message();
    *wrong_trailer.last_mut().unwrap() = 0xbd;
    let mut scratch = [0xa5; 207];
    let error = validate_rsa_pss_sha384_encoded_message(
        &wrong_trailer,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha384ErrorKind::Layout(RsaPssEncodedMessageLayoutErrorKind::InvalidTrailerField,)
    );
    assert_eq!(scratch, [0xa5; 207]);

    let mut short_scratch = [0x5a; 206];
    let error = validate_rsa_pss_sha384_encoded_message(
        &encoded_message(),
        2047,
        &MESSAGE_DIGEST,
        &mut short_scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha384ErrorKind::DataBlockScratchLengthMismatch
    );
    assert_eq!(short_scratch, [0x5a; 206]);
}

#[test]
fn rejects_forbidden_high_bits_padding_delimiter_and_hash_mismatch() {
    let mut scratch = [0u8; 207];
    let mut high_bit = encoded_message();
    high_bit[0] |= 0x80;
    let error = validate_rsa_pss_sha384_encoded_message(
        &high_bit,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssSha384ErrorKind::Layout(RsaPssEncodedMessageLayoutErrorKind::NonzeroUnusedHighBits,)
    );

    let mut invalid_padding = encoded_message();
    invalid_padding[0] ^= 0x01;
    let error = validate_rsa_pss_sha384_encoded_message(
        &invalid_padding,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), RsaPssSha384ErrorKind::InvalidPadding);

    let mut invalid_delimiter = encoded_message();
    invalid_delimiter[158] ^= 0x01;
    let error = validate_rsa_pss_sha384_encoded_message(
        &invalid_delimiter,
        2047,
        &MESSAGE_DIGEST,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), RsaPssSha384ErrorKind::InvalidDelimiter);

    let mut wrong_digest = MESSAGE_DIGEST;
    wrong_digest[47] ^= 0x01;
    let error = validate_rsa_pss_sha384_encoded_message(
        &encoded_message(),
        2047,
        &wrong_digest,
        &mut scratch,
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), RsaPssSha384ErrorKind::HashMismatch);
}

#[test]
fn validates_limits_and_redacts_all_failures() {
    let too_small = RsaPssSha384Limits::new(97).unwrap_err();
    assert_eq!(
        too_small.kind(),
        RsaPssSha384LimitsErrorKind::Layout(
            RsaPssEncodedMessageLayoutPolicyErrorKind::MaximumEncodedMessageTooSmall,
        )
    );

    if usize::BITS > 32 {
        let too_large = RsaPssSha384Limits::new(usize::MAX).unwrap_err();
        assert_eq!(
            too_large.kind(),
            RsaPssSha384LimitsErrorKind::MaskGeneration(
                Mgf1Sha384LimitsErrorKind::MaximumMaskTooLong,
            )
        );
    }

    let mut scratch = [0u8; 207];
    let error = validate_rsa_pss_sha384_encoded_message(
        &encoded_message(),
        2047,
        &[0x7d; 48],
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
        assert!(!rendered.contains("97"));
        assert!(!rendered.contains("0x7d"));
        assert!(!rendered.contains("207"));
    }
}
