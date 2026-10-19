use rsa_pkcs1_v1_5_sha256::{
    validate_sha256_encoded_message, Sha256EncodedMessageErrorKind, ValidatedSha256EncodedMessage,
    MINIMUM_ENCODED_MESSAGE_LENGTH, MINIMUM_PADDING_LENGTH, SHA256_DIGEST_INFO_LENGTH,
    SHA256_DIGEST_LENGTH,
};

const ABC_SHA256: [u8; SHA256_DIGEST_LENGTH] = [
    0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
    0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
];

const DIGEST_INFO_PREFIX: [u8; 19] = [
    0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
    0x00, 0x04, 0x20,
];

fn encoded_message(padding_length: usize, digest: &[u8; SHA256_DIGEST_LENGTH]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(3 + padding_length + SHA256_DIGEST_INFO_LENGTH);
    encoded.extend_from_slice(&[0x00, 0x01]);
    encoded.extend(std::iter::repeat_n(0xff, padding_length));
    encoded.push(0x00);
    encoded.extend_from_slice(&DIGEST_INFO_PREFIX);
    encoded.extend_from_slice(digest);
    encoded
}

fn assert_kind(encoded: &[u8], kind: Sha256EncodedMessageErrorKind) {
    let error = validate_sha256_encoded_message(encoded, &ABC_SHA256).unwrap_err();
    assert_eq!(error.kind(), kind);
}

#[test]
fn validates_minimum_and_longer_exact_messages() {
    for padding_length in [MINIMUM_PADDING_LENGTH, 9, 202] {
        let encoded = encoded_message(padding_length, &ABC_SHA256);
        let evidence = validate_sha256_encoded_message(&encoded, &ABC_SHA256).unwrap();
        assert_eq!(format!("{evidence:?}"), "ValidatedSha256EncodedMessage");
    }
}

#[test]
fn public_lengths_match_the_rfc_shape() {
    assert_eq!(SHA256_DIGEST_LENGTH, 32);
    assert_eq!(SHA256_DIGEST_INFO_LENGTH, 51);
    assert_eq!(MINIMUM_PADDING_LENGTH, 8);
    assert_eq!(MINIMUM_ENCODED_MESSAGE_LENGTH, 62);
    assert_eq!(core::mem::size_of::<ValidatedSha256EncodedMessage>(), 0);
}

#[test]
fn rejects_every_input_shorter_than_the_minimum() {
    for length in 0..MINIMUM_ENCODED_MESSAGE_LENGTH {
        let encoded = vec![0xff; length];
        assert_kind(
            &encoded,
            Sha256EncodedMessageErrorKind::EncodedMessageTooShort,
        );
    }
}

#[test]
fn rejects_either_invalid_header_octet() {
    for index in [0, 1] {
        let mut encoded = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA256);
        encoded[index] ^= 0x01;
        assert_kind(&encoded, Sha256EncodedMessageErrorKind::InvalidBlockHeader);
    }
}

#[test]
fn rejects_every_non_ff_padding_position() {
    let canonical = encoded_message(32, &ABC_SHA256);
    for index in 2..34 {
        let mut encoded = canonical.clone();
        encoded[index] = if index % 2 == 0 { 0x00 } else { 0xfe };
        assert_kind(&encoded, Sha256EncodedMessageErrorKind::InvalidPadding);
    }
}

#[test]
fn rejects_missing_or_displaced_separator() {
    let mut missing = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA256);
    let separator = 2 + MINIMUM_PADDING_LENGTH;
    missing[separator] = 0xff;
    assert_kind(&missing, Sha256EncodedMessageErrorKind::InvalidSeparator);

    let mut early = encoded_message(MINIMUM_PADDING_LENGTH + 1, &ABC_SHA256);
    early[2 + MINIMUM_PADDING_LENGTH] = 0x00;
    assert_kind(&early, Sha256EncodedMessageErrorKind::InvalidPadding);
}

#[test]
fn rejects_every_single_byte_digest_info_prefix_mutation() {
    let canonical = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA256);
    let prefix_start = 3 + MINIMUM_PADDING_LENGTH;
    for prefix_offset in 0..DIGEST_INFO_PREFIX.len() {
        let mut encoded = canonical.clone();
        encoded[prefix_start + prefix_offset] ^= 0x01;
        assert_kind(&encoded, Sha256EncodedMessageErrorKind::InvalidDigestInfo);
    }
}

#[test]
fn rejects_digest_mismatches_without_an_index() {
    for digest_index in [0, SHA256_DIGEST_LENGTH / 2, SHA256_DIGEST_LENGTH - 1] {
        let mut encoded = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA256);
        let encoded_index = encoded.len() - SHA256_DIGEST_LENGTH + digest_index;
        encoded[encoded_index] ^= 0x80;
        assert_kind(&encoded, Sha256EncodedMessageErrorKind::DigestMismatch);
    }
}

#[test]
fn rejects_appended_and_truncated_data() {
    let canonical = encoded_message(MINIMUM_PADDING_LENGTH + 1, &ABC_SHA256);

    let mut appended = canonical.clone();
    appended.push(0x00);
    assert_kind(&appended, Sha256EncodedMessageErrorKind::InvalidPadding);

    let truncated = &canonical[..canonical.len() - 1];
    assert_kind(truncated, Sha256EncodedMessageErrorKind::InvalidSeparator);
}

#[test]
fn errors_are_stable_and_redacted() {
    let secret_marker = "decafbad";
    let mut encoded = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA256);
    encoded[0] = 0x7d;
    let error = validate_sha256_encoded_message(&encoded, &ABC_SHA256).unwrap_err();

    assert_eq!(
        error.kind(),
        Sha256EncodedMessageErrorKind::InvalidBlockHeader
    );
    let debug = format!("{error:?}");
    let display = error.to_string();
    assert_eq!(
        debug,
        "Sha256EncodedMessageError { kind: InvalidBlockHeader }"
    );
    assert_eq!(
        display,
        "PKCS #1 v1.5 SHA-256 encoded-message error InvalidBlockHeader"
    );
    assert!(!debug.contains(secret_marker));
    assert!(!display.contains(secret_marker));
    assert!(!debug.contains("125"));
    assert!(!display.contains("125"));
}
