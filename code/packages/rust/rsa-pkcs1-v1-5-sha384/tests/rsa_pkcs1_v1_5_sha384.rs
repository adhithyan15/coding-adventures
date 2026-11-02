use rsa_pkcs1_v1_5_sha384::{
    validate_sha384_encoded_message, Sha384EncodedMessageErrorKind, ValidatedSha384EncodedMessage,
    MINIMUM_ENCODED_MESSAGE_LENGTH, MINIMUM_PADDING_LENGTH, SHA384_DIGEST_INFO_LENGTH,
    SHA384_DIGEST_LENGTH,
};

// SHA-384("abc"), independently confirmed with the platform `shasum -a 384`
// implementation before capture.
const ABC_SHA384: [u8; SHA384_DIGEST_LENGTH] = [
    0xcb, 0x00, 0x75, 0x3f, 0x45, 0xa3, 0x5e, 0x8b, 0xb5, 0xa0, 0x3d, 0x69, 0x9a, 0xc6, 0x50, 0x07,
    0x27, 0x2c, 0x32, 0xab, 0x0e, 0xde, 0xd1, 0x63, 0x1a, 0x8b, 0x60, 0x5a, 0x43, 0xff, 0x5b, 0xed,
    0x80, 0x86, 0x07, 0x2b, 0xa1, 0xe7, 0xcc, 0x23, 0x58, 0xba, 0xec, 0xa1, 0x34, 0xc8, 0x25, 0xa7,
];

const DIGEST_INFO_PREFIX: [u8; 19] = [
    0x30, 0x41, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02, 0x05,
    0x00, 0x04, 0x30,
];

fn encoded_message(padding_length: usize, digest: &[u8; SHA384_DIGEST_LENGTH]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(3 + padding_length + SHA384_DIGEST_INFO_LENGTH);
    encoded.extend_from_slice(&[0x00, 0x01]);
    encoded.extend(std::iter::repeat_n(0xff, padding_length));
    encoded.push(0x00);
    encoded.extend_from_slice(&DIGEST_INFO_PREFIX);
    encoded.extend_from_slice(digest);
    encoded
}

fn assert_kind(encoded: &[u8], kind: Sha384EncodedMessageErrorKind) {
    let error = validate_sha384_encoded_message(encoded, &ABC_SHA384).unwrap_err();
    assert_eq!(error.kind(), kind);
}

#[test]
fn validates_minimum_and_longer_exact_messages() {
    for padding_length in [MINIMUM_PADDING_LENGTH, 9, 186] {
        let encoded = encoded_message(padding_length, &ABC_SHA384);
        let evidence = validate_sha384_encoded_message(&encoded, &ABC_SHA384).unwrap();
        assert_eq!(format!("{evidence:?}"), "ValidatedSha384EncodedMessage");
    }
}

#[test]
fn public_lengths_match_the_rfc_shape() {
    assert_eq!(SHA384_DIGEST_LENGTH, 48);
    assert_eq!(SHA384_DIGEST_INFO_LENGTH, 67);
    assert_eq!(MINIMUM_PADDING_LENGTH, 8);
    assert_eq!(MINIMUM_ENCODED_MESSAGE_LENGTH, 78);
    assert_eq!(core::mem::size_of::<ValidatedSha384EncodedMessage>(), 0);
}

#[test]
fn rejects_every_input_shorter_than_the_minimum() {
    for length in 0..MINIMUM_ENCODED_MESSAGE_LENGTH {
        let encoded = vec![0xff; length];
        assert_kind(
            &encoded,
            Sha384EncodedMessageErrorKind::EncodedMessageTooShort,
        );
    }
}

#[test]
fn rejects_either_invalid_header_octet() {
    for index in [0, 1] {
        let mut encoded = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA384);
        encoded[index] ^= 0x01;
        assert_kind(&encoded, Sha384EncodedMessageErrorKind::InvalidBlockHeader);
    }
}

#[test]
fn rejects_every_non_ff_padding_position() {
    let canonical = encoded_message(32, &ABC_SHA384);
    for index in 2..34 {
        let mut encoded = canonical.clone();
        encoded[index] = if index % 2 == 0 { 0x00 } else { 0xfe };
        assert_kind(&encoded, Sha384EncodedMessageErrorKind::InvalidPadding);
    }
}

#[test]
fn rejects_missing_or_displaced_separator() {
    let mut missing = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA384);
    let separator = 2 + MINIMUM_PADDING_LENGTH;
    missing[separator] = 0xff;
    assert_kind(&missing, Sha384EncodedMessageErrorKind::InvalidSeparator);

    let mut early = encoded_message(MINIMUM_PADDING_LENGTH + 1, &ABC_SHA384);
    early[2 + MINIMUM_PADDING_LENGTH] = 0x00;
    assert_kind(&early, Sha384EncodedMessageErrorKind::InvalidPadding);
}

#[test]
fn rejects_every_single_byte_digest_info_prefix_mutation() {
    let canonical = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA384);
    let prefix_start = 3 + MINIMUM_PADDING_LENGTH;
    for prefix_offset in 0..DIGEST_INFO_PREFIX.len() {
        let mut encoded = canonical.clone();
        encoded[prefix_start + prefix_offset] ^= 0x01;
        assert_kind(&encoded, Sha384EncodedMessageErrorKind::InvalidDigestInfo);
    }
}

#[test]
fn rejects_digest_mismatches_without_an_index() {
    for digest_index in [0, SHA384_DIGEST_LENGTH / 2, SHA384_DIGEST_LENGTH - 1] {
        let mut encoded = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA384);
        let encoded_index = encoded.len() - SHA384_DIGEST_LENGTH + digest_index;
        encoded[encoded_index] ^= 0x80;
        assert_kind(&encoded, Sha384EncodedMessageErrorKind::DigestMismatch);
    }
}

#[test]
fn rejects_appended_and_truncated_data() {
    let canonical = encoded_message(MINIMUM_PADDING_LENGTH + 1, &ABC_SHA384);

    let mut appended = canonical.clone();
    appended.push(0x00);
    assert_kind(&appended, Sha384EncodedMessageErrorKind::InvalidPadding);

    let truncated = &canonical[..canonical.len() - 1];
    assert_kind(truncated, Sha384EncodedMessageErrorKind::InvalidSeparator);
}

#[test]
fn errors_are_stable_and_redacted() {
    let payload_marker = "decafbad";
    let mut encoded = encoded_message(MINIMUM_PADDING_LENGTH, &ABC_SHA384);
    encoded[0] = 0x7d;
    let error = validate_sha384_encoded_message(&encoded, &ABC_SHA384).unwrap_err();

    assert_eq!(
        error.kind(),
        Sha384EncodedMessageErrorKind::InvalidBlockHeader
    );
    let debug = format!("{error:?}");
    let display = error.to_string();
    assert_eq!(
        debug,
        "Sha384EncodedMessageError { kind: InvalidBlockHeader }"
    );
    assert_eq!(
        display,
        "PKCS #1 v1.5 SHA-384 encoded-message error InvalidBlockHeader"
    );
    assert!(!debug.contains(payload_marker));
    assert!(!display.contains(payload_marker));
    assert!(!debug.contains("125"));
    assert!(!display.contains("125"));
}
