use rsa_pkcs1_v1_5_sha512::{
    validate_sha512_encoded_message, Sha512EncodedMessageErrorKind, ValidatedSha512EncodedMessage,
    MINIMUM_ENCODED_MESSAGE_LENGTH, MINIMUM_PADDING_LENGTH, SHA512_DIGEST_INFO_LENGTH,
    SHA512_DIGEST_LENGTH,
};

// SHA-512("abc"), independently confirmed with `shasum -a 512`.
const ABC_SHA512: [u8; SHA512_DIGEST_LENGTH] = [
    0xdd, 0xaf, 0x35, 0xa1, 0x93, 0x61, 0x7a, 0xba, 0xcc, 0x41, 0x73, 0x49, 0xae, 0x20, 0x41, 0x31,
    0x12, 0xe6, 0xfa, 0x4e, 0x89, 0xa9, 0x7e, 0xa2, 0x0a, 0x9e, 0xee, 0xe6, 0x4b, 0x55, 0xd3, 0x9a,
    0x21, 0x92, 0x99, 0x2a, 0x27, 0x4f, 0xc1, 0xa8, 0x36, 0xba, 0x3c, 0x23, 0xa3, 0xfe, 0xeb, 0xbd,
    0x45, 0x4d, 0x44, 0x23, 0x64, 0x3c, 0xe8, 0x0e, 0x2a, 0x9a, 0xc9, 0x4f, 0xa5, 0x4c, 0xa4, 0x9f,
];

const DIGEST_INFO_PREFIX: [u8; 19] = [
    0x30, 0x51, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03, 0x05,
    0x00, 0x04, 0x40,
];

fn encoded_message(padding_length: usize, digest: &[u8; SHA512_DIGEST_LENGTH]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(3 + padding_length + SHA512_DIGEST_INFO_LENGTH);
    encoded.extend_from_slice(&[0x00, 0x01]);
    encoded.extend(std::iter::repeat_n(0xff, padding_length));
    encoded.push(0x00);
    encoded.extend_from_slice(&DIGEST_INFO_PREFIX);
    encoded.extend_from_slice(digest);
    encoded
}

fn assert_kind(encoded: &[u8], kind: Sha512EncodedMessageErrorKind) {
    let error = validate_sha512_encoded_message(encoded, &ABC_SHA512).unwrap_err();
    assert_eq!(error.kind(), kind);
}

#[test]
fn validates_minimum_and_longer_exact_messages() {
    for padding_length in [MINIMUM_PADDING_LENGTH, 9, 170] {
        let encoded = encoded_message(padding_length, &ABC_SHA512);
        let evidence = validate_sha512_encoded_message(&encoded, &ABC_SHA512).unwrap();
        assert_eq!(format!("{evidence:?}"), "ValidatedSha512EncodedMessage");
    }
}

#[test]
fn public_lengths_match_the_rfc_shape() {
    assert_eq!(SHA512_DIGEST_LENGTH, 64);
    assert_eq!(SHA512_DIGEST_INFO_LENGTH, 83);
    assert_eq!(MINIMUM_PADDING_LENGTH, 8);
    assert_eq!(MINIMUM_ENCODED_MESSAGE_LENGTH, 94);
    assert_eq!(core::mem::size_of::<ValidatedSha512EncodedMessage>(), 0);
}

#[test]
fn rejects_every_input_shorter_than_the_minimum() {
    for length in 0..MINIMUM_ENCODED_MESSAGE_LENGTH {
        assert_kind(
            &vec![0xff; length],
            Sha512EncodedMessageErrorKind::EncodedMessageTooShort,
        );
    }
}

#[test]
fn rejects_either_invalid_header_octet() {
    for index in [0, 1] {
        let mut encoded = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA512);
        encoded[index] ^= 0x01;
        assert_kind(&encoded, Sha512EncodedMessageErrorKind::InvalidBlockHeader);
    }
}

#[test]
fn rejects_every_non_ff_padding_position() {
    let canonical = encoded_message(32, &ABC_SHA512);
    for index in 2..34 {
        let mut encoded = canonical.clone();
        encoded[index] = if index % 2 == 0 { 0x00 } else { 0xfe };
        assert_kind(&encoded, Sha512EncodedMessageErrorKind::InvalidPadding);
    }
}

#[test]
fn rejects_missing_or_displaced_separator() {
    let mut missing = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA512);
    missing[2 + MINIMUM_PADDING_LENGTH] = 0xff;
    assert_kind(&missing, Sha512EncodedMessageErrorKind::InvalidSeparator);

    let mut early = encoded_message(MINIMUM_PADDING_LENGTH + 1, &ABC_SHA512);
    early[2 + MINIMUM_PADDING_LENGTH] = 0x00;
    assert_kind(&early, Sha512EncodedMessageErrorKind::InvalidPadding);
}

#[test]
fn rejects_every_single_byte_digest_info_prefix_mutation() {
    let canonical = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA512);
    let prefix_start = 3 + MINIMUM_PADDING_LENGTH;
    for offset in 0..DIGEST_INFO_PREFIX.len() {
        let mut encoded = canonical.clone();
        encoded[prefix_start + offset] ^= 0x01;
        assert_kind(&encoded, Sha512EncodedMessageErrorKind::InvalidDigestInfo);
    }
}

#[test]
fn rejects_every_digest_mismatch_without_an_index() {
    for digest_index in 0..SHA512_DIGEST_LENGTH {
        let mut encoded = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA512);
        let encoded_index = encoded.len() - SHA512_DIGEST_LENGTH + digest_index;
        encoded[encoded_index] ^= 0x80;
        assert_kind(&encoded, Sha512EncodedMessageErrorKind::DigestMismatch);
    }
}

#[test]
fn rejects_appended_and_truncated_data() {
    let canonical = encoded_message(MINIMUM_PADDING_LENGTH + 1, &ABC_SHA512);
    let mut appended = canonical.clone();
    appended.push(0x00);
    assert_kind(&appended, Sha512EncodedMessageErrorKind::InvalidPadding);
    assert_kind(
        &canonical[..canonical.len() - 1],
        Sha512EncodedMessageErrorKind::InvalidSeparator,
    );
}

#[test]
fn errors_are_stable_and_redacted() {
    let mut encoded = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA512);
    encoded[0] = 0x7d;
    let error = validate_sha512_encoded_message(&encoded, &ABC_SHA512).unwrap_err();
    assert_eq!(
        error.kind(),
        Sha512EncodedMessageErrorKind::InvalidBlockHeader
    );
    assert_eq!(
        format!("{error:?}"),
        "Sha512EncodedMessageError { kind: InvalidBlockHeader }"
    );
    assert_eq!(
        error.to_string(),
        "PKCS #1 v1.5 SHA-512 encoded-message error InvalidBlockHeader"
    );
    assert!(!format!("{error:?}").contains("125"));
    assert!(!error.to_string().contains("125"));
}
