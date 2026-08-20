//! The portable D18F encrypted-message profile.
//!
//! This layer preserves the existing D18M version 1 bytes while adding the
//! validation, stable errors, and lossless JSON contract shared by all six
//! language implementations.

use std::str::FromStr;

use coding_adventures_json_parser::try_parse_json;
use coding_adventures_json_serializer::serialize;
use coding_adventures_json_value::{from_ast, JsonNumber, JsonValue};
use coding_adventures_uuid::UUID;

use crate::{
    authenticated_message_header, decrypt_message, encrypt_message, wire, ChannelCryptoError,
    ChannelId, ChannelMasterKey, EncryptedMessage, KeyEpoch, MessageFields, MessageHeader,
    OriginatorSigningKey, Sequence,
};

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const JSON_FIELDS: [&str; 13] = [
    "record_type",
    "wire_version",
    "message_id",
    "timestamp_ns",
    "originator_id_b64",
    "channel_id",
    "sequence",
    "key_epoch",
    "content_type",
    "plaintext_hash_hex",
    "ciphertext_b64",
    "authentication_tag_b64",
    "originator_signature_b64",
];

/// Maximum accepted UTF-8 bytes in one diagnostic JSON record.
///
/// The bound accommodates a base64-encoded 64 MiB ciphertext plus the fixed
/// envelope without allowing an unbounded parser input.
pub const MAX_MESSAGE_JSON_BYTES: usize = 90 * 1024 * 1024;

/// Stable D18F failure classes used across language boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageProfileError {
    /// Record magic or JSON record type is not `D18M`.
    InvalidMagic,
    /// Binary or JSON wire version is not 1.
    UnsupportedVersion,
    /// A declared or fixed binary field is incomplete.
    TruncatedRecord,
    /// Bytes remain after a complete binary record.
    TrailingBytes,
    /// A bounded field or total JSON input exceeds its limit.
    LengthLimitExceeded,
    /// The binary content type is not valid UTF-8.
    InvalidUtf8,
    /// A UUID, MIME value, decimal, base64, hexadecimal, or field shape is invalid.
    InvalidField,
    /// JSON syntax, keys, required fields, or value types are invalid.
    InvalidJson,
    /// No channel master key exists for the message's key epoch.
    MissingEpochKey,
    /// The Ed25519 signature over the authenticated header is invalid.
    InvalidSignature,
    /// XChaCha20-Poly1305 rejected the message.
    AuthenticationFailed,
    /// Recovered plaintext does not match the authenticated SHA-256 digest.
    PlaintextHashMismatch,
}

impl MessageProfileError {
    /// Return the normative D18F machine-readable error code.
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidMagic => "invalid_magic",
            Self::UnsupportedVersion => "unsupported_version",
            Self::TruncatedRecord => "truncated_record",
            Self::TrailingBytes => "trailing_bytes",
            Self::LengthLimitExceeded => "length_limit_exceeded",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::InvalidField => "invalid_field",
            Self::InvalidJson => "invalid_json",
            Self::MissingEpochKey => "missing_epoch_key",
            Self::InvalidSignature => "invalid_signature",
            Self::AuthenticationFailed => "authentication_failed",
            Self::PlaintextHashMismatch => "plaintext_hash_mismatch",
        }
    }
}

impl core::fmt::Display for MessageProfileError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for MessageProfileError {}

impl From<wire::ChannelWireError> for MessageProfileError {
    fn from(error: wire::ChannelWireError) -> Self {
        match error {
            wire::ChannelWireError::InvalidMagic => Self::InvalidMagic,
            wire::ChannelWireError::UnsupportedVersion(_) => Self::UnsupportedVersion,
            wire::ChannelWireError::Truncated => Self::TruncatedRecord,
            wire::ChannelWireError::TrailingBytes => Self::TrailingBytes,
            wire::ChannelWireError::LengthLimitExceeded { .. } => Self::LengthLimitExceeded,
            wire::ChannelWireError::InvalidUtf8(_) => Self::InvalidUtf8,
        }
    }
}

impl From<ChannelCryptoError> for MessageProfileError {
    fn from(error: ChannelCryptoError) -> Self {
        match error {
            ChannelCryptoError::InvalidMessageSignature => Self::InvalidSignature,
            ChannelCryptoError::AuthenticationFailed => Self::AuthenticationFailed,
            ChannelCryptoError::PlaintextHashMismatch => Self::PlaintextHashMismatch,
            _ => Self::InvalidField,
        }
    }
}

/// Stateful RFC 9562 UUID-v7 generator with same-millisecond ordering.
///
/// Callers inject both Unix epoch milliseconds and ten entropy bytes. When the
/// supplied millisecond does not advance, the 74-bit random portion is
/// incremented, so returned UUID bytes remain strictly ordered. This makes the
/// clock and entropy deterministic in tests without weakening production
/// callers' choice of sources.
#[derive(Clone, Debug, Default)]
pub struct MonotonicUuidV7Generator {
    last_timestamp_ms: Option<u64>,
    last_random: u128,
}

impl MonotonicUuidV7Generator {
    /// Construct an empty generator.
    pub const fn new() -> Self {
        Self {
            last_timestamp_ms: None,
            last_random: 0,
        }
    }

    /// Generate the next ordered UUID-v7 from injected source values.
    pub fn next(
        &mut self,
        timestamp_ms: u64,
        entropy: [u8; 10],
    ) -> Result<[u8; 16], MessageProfileError> {
        const MAX_TIMESTAMP: u64 = (1u64 << 48) - 1;
        const RANDOM_MASK: u128 = (1u128 << 74) - 1;
        if timestamp_ms > MAX_TIMESTAMP {
            return Err(MessageProfileError::InvalidField);
        }
        let supplied_random = u128::from_be_bytes([
            0, 0, 0, 0, 0, 0, entropy[0], entropy[1], entropy[2], entropy[3], entropy[4],
            entropy[5], entropy[6], entropy[7], entropy[8], entropy[9],
        ]) & RANDOM_MASK;
        let (effective_timestamp, random) = match self.last_timestamp_ms {
            None => (timestamp_ms, supplied_random),
            Some(previous) if timestamp_ms > previous => (timestamp_ms, supplied_random),
            Some(previous) if self.last_random < RANDOM_MASK => (previous, self.last_random + 1),
            Some(previous) if previous < MAX_TIMESTAMP => (previous + 1, 0),
            Some(_) => return Err(MessageProfileError::InvalidField),
        };
        self.last_timestamp_ms = Some(effective_timestamp);
        self.last_random = random;

        let random_a = (random >> 62) & 0x0fff;
        let random_b = random & ((1u128 << 62) - 1);
        let value = (u128::from(effective_timestamp) << 80)
            | (7u128 << 76)
            | (random_a << 64)
            | (2u128 << 62)
            | random_b;
        Ok(value.to_be_bytes())
    }
}

/// Validate the high-level D18F creation and delivery rules.
pub fn validate_message_fields(fields: &MessageFields) -> Result<(), MessageProfileError> {
    validate_uuid_v7(fields.message_id())?;
    validate_uuid_v7(fields.channel_id().0)?;
    if fields.originator_id().is_empty() {
        return Err(MessageProfileError::InvalidField);
    }
    if fields.originator_id().len() > wire::MAX_IDENTITY_BYTES {
        return Err(MessageProfileError::LengthLimitExceeded);
    }
    if fields.content_type().len() > wire::MAX_CONTENT_TYPE_BYTES {
        return Err(MessageProfileError::LengthLimitExceeded);
    }
    validate_mime(fields.content_type())
}

/// Validate, hash, sign, and encrypt one portable D18 message.
pub fn message_create(
    fields: MessageFields,
    plaintext: &[u8],
    signing_key: &OriginatorSigningKey,
    channel_master_key: &ChannelMasterKey,
) -> Result<EncryptedMessage, MessageProfileError> {
    validate_message_fields(&fields)?;
    if plaintext.len() > wire::MAX_CIPHERTEXT_BYTES {
        return Err(MessageProfileError::LengthLimitExceeded);
    }
    Ok(encrypt_message(
        fields,
        plaintext,
        channel_master_key,
        signing_key,
    ))
}

/// Verify and decrypt one message with an explicitly selected epoch key.
pub fn message_verify(
    message: &EncryptedMessage,
    originator_public_key: &[u8; 32],
    channel_master_key: &ChannelMasterKey,
) -> Result<Vec<u8>, MessageProfileError> {
    validate_message_fields(message.header().fields())?;
    decrypt_message(message, channel_master_key, originator_public_key).map_err(Into::into)
}

/// Resolve the named key epoch before signature and AEAD verification.
pub fn message_verify_with_key_resolver<'a, F>(
    message: &EncryptedMessage,
    originator_public_key: &[u8; 32],
    key_for_epoch: F,
) -> Result<Vec<u8>, MessageProfileError>
where
    F: FnOnce(KeyEpoch) -> Option<&'a ChannelMasterKey>,
{
    validate_message_fields(message.header().fields())?;
    let key = key_for_epoch(message.header().fields().key_epoch())
        .ok_or(MessageProfileError::MissingEpochKey)?;
    decrypt_message(message, key, originator_public_key).map_err(Into::into)
}

/// Serialize one message as the unchanged D18M version 1 binary record.
pub fn message_serialize(message: &EncryptedMessage) -> Result<Vec<u8>, MessageProfileError> {
    wire::encode_message(message).map_err(Into::into)
}

/// Structurally decode one D18M version 1 binary record.
pub fn message_deserialize(bytes: &[u8]) -> Result<EncryptedMessage, MessageProfileError> {
    wire::decode_message(bytes).map_err(Into::into)
}

/// Encode one message as canonical, lossless D18F JSON bytes.
pub fn message_to_json(message: &EncryptedMessage) -> Result<Vec<u8>, MessageProfileError> {
    let fields = message.header().fields();
    let value = JsonValue::Object(vec![
        ("record_type".into(), JsonValue::String("D18M".into())),
        (
            "wire_version".into(),
            JsonValue::Number(JsonNumber::Integer(i64::from(wire::WIRE_VERSION))),
        ),
        (
            "message_id".into(),
            JsonValue::String(UUID::from_bytes(fields.message_id()).to_string()),
        ),
        (
            "timestamp_ns".into(),
            JsonValue::String(fields.timestamp_ns().to_string()),
        ),
        (
            "originator_id_b64".into(),
            JsonValue::String(encode_base64(fields.originator_id())),
        ),
        (
            "channel_id".into(),
            JsonValue::String(UUID::from_bytes(fields.channel_id().0).to_string()),
        ),
        (
            "sequence".into(),
            JsonValue::String(fields.sequence().0.to_string()),
        ),
        (
            "key_epoch".into(),
            JsonValue::String(fields.key_epoch().0.to_string()),
        ),
        (
            "content_type".into(),
            JsonValue::String(fields.content_type().to_owned()),
        ),
        (
            "plaintext_hash_hex".into(),
            JsonValue::String(encode_hex(&message.header().plaintext_hash())),
        ),
        (
            "ciphertext_b64".into(),
            JsonValue::String(encode_base64(message.ciphertext())),
        ),
        (
            "authentication_tag_b64".into(),
            JsonValue::String(encode_base64(&message.authentication_tag())),
        ),
        (
            "originator_signature_b64".into(),
            JsonValue::String(encode_base64(&message.originator_signature())),
        ),
    ]);
    let json = serialize(&value).map_err(|_| MessageProfileError::InvalidJson)?;
    if json.len() > MAX_MESSAGE_JSON_BYTES {
        return Err(MessageProfileError::LengthLimitExceeded);
    }
    Ok(json.into_bytes())
}

/// Structurally decode lossless D18F JSON into an immutable message.
pub fn message_from_json(bytes: &[u8]) -> Result<EncryptedMessage, MessageProfileError> {
    if bytes.len() > MAX_MESSAGE_JSON_BYTES {
        return Err(MessageProfileError::LengthLimitExceeded);
    }
    let source = std::str::from_utf8(bytes).map_err(|_| MessageProfileError::InvalidJson)?;
    let ast = try_parse_json(source).map_err(|_| MessageProfileError::InvalidJson)?;
    let value = from_ast(&ast).map_err(|_| MessageProfileError::InvalidJson)?;
    let object = match &value {
        JsonValue::Object(object) => object,
        _ => return Err(MessageProfileError::InvalidJson),
    };
    validate_json_keys(object)?;
    if string_field(object, "record_type")? != "D18M" {
        return Err(MessageProfileError::InvalidMagic);
    }
    match field(object, "wire_version")? {
        JsonValue::Number(JsonNumber::Integer(1)) => {}
        JsonValue::Number(_) => return Err(MessageProfileError::UnsupportedVersion),
        _ => return Err(MessageProfileError::InvalidJson),
    }

    let message_id = decode_uuid_v7(string_field(object, "message_id")?)?;
    let timestamp_ns = decode_decimal(string_field(object, "timestamp_ns")?)?;
    let originator_id = decode_base64(
        string_field(object, "originator_id_b64")?,
        wire::MAX_IDENTITY_BYTES,
    )?;
    let channel_id = ChannelId(decode_uuid_v7(string_field(object, "channel_id")?)?);
    let sequence = Sequence(decode_decimal(string_field(object, "sequence")?)?);
    let key_epoch = KeyEpoch(decode_decimal(string_field(object, "key_epoch")?)?);
    let content_type = string_field(object, "content_type")?.to_owned();
    if content_type.len() > wire::MAX_CONTENT_TYPE_BYTES {
        return Err(MessageProfileError::LengthLimitExceeded);
    }
    let plaintext_hash = decode_hex_array(string_field(object, "plaintext_hash_hex")?)?;
    let ciphertext = decode_base64(
        string_field(object, "ciphertext_b64")?,
        wire::MAX_CIPHERTEXT_BYTES,
    )?;
    let authentication_tag = decode_base64_array(string_field(object, "authentication_tag_b64")?)?;
    let originator_signature =
        decode_base64_array(string_field(object, "originator_signature_b64")?)?;

    Ok(EncryptedMessage {
        header: MessageHeader {
            fields: MessageFields::new(
                message_id,
                timestamp_ns,
                originator_id,
                channel_id,
                sequence,
                key_epoch,
                content_type,
            ),
            plaintext_hash,
        },
        ciphertext,
        authentication_tag,
        originator_signature,
    })
}

fn validate_json_keys(object: &[(String, JsonValue)]) -> Result<(), MessageProfileError> {
    if object.len() != JSON_FIELDS.len() {
        return Err(MessageProfileError::InvalidJson);
    }
    for (index, (key, _)) in object.iter().enumerate() {
        if !JSON_FIELDS.contains(&key.as_str())
            || object[..index].iter().any(|(earlier, _)| earlier == key)
        {
            return Err(MessageProfileError::InvalidJson);
        }
    }
    Ok(())
}

fn field<'a>(
    object: &'a [(String, JsonValue)],
    name: &str,
) -> Result<&'a JsonValue, MessageProfileError> {
    object
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .ok_or(MessageProfileError::InvalidJson)
}

fn string_field<'a>(
    object: &'a [(String, JsonValue)],
    name: &str,
) -> Result<&'a str, MessageProfileError> {
    match field(object, name)? {
        JsonValue::String(value) => Ok(value),
        _ => Err(MessageProfileError::InvalidJson),
    }
}

fn validate_uuid_v7(bytes: [u8; 16]) -> Result<(), MessageProfileError> {
    let uuid = UUID::from_bytes(bytes);
    if uuid.version() != 7 || uuid.variant() != "rfc4122" {
        return Err(MessageProfileError::InvalidField);
    }
    Ok(())
}

fn decode_uuid_v7(value: &str) -> Result<[u8; 16], MessageProfileError> {
    let uuid = UUID::from_str(value).map_err(|_| MessageProfileError::InvalidField)?;
    if uuid.to_string() != value {
        return Err(MessageProfileError::InvalidField);
    }
    validate_uuid_v7(uuid.bytes())?;
    Ok(uuid.bytes())
}

fn decode_decimal(value: &str) -> Result<u64, MessageProfileError> {
    if value != "0"
        && (value.starts_with('0')
            || value.is_empty()
            || !value.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(MessageProfileError::InvalidField);
    }
    value
        .parse::<u64>()
        .map_err(|_| MessageProfileError::InvalidField)
}

fn validate_mime(value: &str) -> Result<(), MessageProfileError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii() && (0x20..=0x7e).contains(&byte))
    {
        return Err(MessageProfileError::InvalidField);
    }
    let bytes = value.as_bytes();
    let mut index = 0;
    consume_token(bytes, &mut index)?;
    if bytes.get(index) != Some(&b'/') {
        return Err(MessageProfileError::InvalidField);
    }
    index += 1;
    consume_token(bytes, &mut index)?;
    while index < bytes.len() {
        consume_spaces(bytes, &mut index);
        if bytes.get(index) != Some(&b';') {
            return Err(MessageProfileError::InvalidField);
        }
        index += 1;
        consume_spaces(bytes, &mut index);
        consume_token(bytes, &mut index)?;
        consume_spaces(bytes, &mut index);
        if bytes.get(index) != Some(&b'=') {
            return Err(MessageProfileError::InvalidField);
        }
        index += 1;
        consume_spaces(bytes, &mut index);
        if bytes.get(index) == Some(&b'\"') {
            consume_quoted_value(bytes, &mut index)?;
        } else {
            consume_token(bytes, &mut index)?;
        }
    }
    Ok(())
}

fn consume_token(bytes: &[u8], index: &mut usize) -> Result<(), MessageProfileError> {
    let start = *index;
    while bytes.get(*index).is_some_and(|byte| is_mime_token(*byte)) {
        *index += 1;
    }
    if *index == start {
        return Err(MessageProfileError::InvalidField);
    }
    Ok(())
}

fn consume_spaces(bytes: &[u8], index: &mut usize) {
    while bytes.get(*index) == Some(&b' ') {
        *index += 1;
    }
}

fn consume_quoted_value(bytes: &[u8], index: &mut usize) -> Result<(), MessageProfileError> {
    *index += 1;
    loop {
        match bytes.get(*index) {
            Some(b'\"') => {
                *index += 1;
                return Ok(());
            }
            Some(b'\\') => {
                *index += 1;
                if bytes.get(*index).is_none() {
                    return Err(MessageProfileError::InvalidField);
                }
                *index += 1;
            }
            Some(_) => *index += 1,
            None => return Err(MessageProfileError::InvalidField),
        }
    }
}

fn is_mime_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

fn encode_base64(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = u32::from(*chunk.get(1).unwrap_or(&0));
        let third = u32::from(*chunk.get(2).unwrap_or(&0));
        let word = (first << 16) | (second << 8) | third;
        output.push(char::from(BASE64[((word >> 18) & 63) as usize]));
        output.push(char::from(BASE64[((word >> 12) & 63) as usize]));
        output.push(if chunk.len() > 1 {
            char::from(BASE64[((word >> 6) & 63) as usize])
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            char::from(BASE64[(word & 63) as usize])
        } else {
            '='
        });
    }
    output
}

fn decode_base64(value: &str, maximum: usize) -> Result<Vec<u8>, MessageProfileError> {
    if !value.len().is_multiple_of(4) {
        return Err(MessageProfileError::InvalidField);
    }
    if value.len() / 4 * 3 > maximum.saturating_add(2) {
        return Err(MessageProfileError::LengthLimitExceeded);
    }
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(value.len() / 4 * 3);
    for (index, chunk) in bytes.as_chunks::<4>().0.iter().enumerate() {
        let final_chunk = index + 1 == bytes.len() / 4;
        let a = base64_digit(chunk[0])?;
        let b = base64_digit(chunk[1])?;
        let c = if chunk[2] == b'=' {
            0
        } else {
            base64_digit(chunk[2])?
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            base64_digit(chunk[3])?
        };
        if (!final_chunk && (chunk[2] == b'=' || chunk[3] == b'='))
            || (chunk[2] == b'=' && chunk[3] != b'=')
        {
            return Err(MessageProfileError::InvalidField);
        }
        let word = (u32::from(a) << 18) | (u32::from(b) << 12) | (u32::from(c) << 6) | u32::from(d);
        output.push(((word >> 16) & 255) as u8);
        if chunk[2] != b'=' {
            output.push(((word >> 8) & 255) as u8);
        }
        if chunk[3] != b'=' {
            output.push((word & 255) as u8);
        }
    }
    if output.len() > maximum {
        return Err(MessageProfileError::LengthLimitExceeded);
    }
    if encode_base64(&output) != value {
        return Err(MessageProfileError::InvalidField);
    }
    Ok(output)
}

fn decode_base64_array<const N: usize>(value: &str) -> Result<[u8; N], MessageProfileError> {
    decode_base64(value, N)?
        .try_into()
        .map_err(|_| MessageProfileError::InvalidField)
}

fn base64_digit(byte: u8) -> Result<u8, MessageProfileError> {
    BASE64
        .iter()
        .position(|candidate| *candidate == byte)
        .map(|index| index as u8)
        .ok_or(MessageProfileError::InvalidField)
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 15)]));
    }
    output
}

fn decode_hex_array<const N: usize>(value: &str) -> Result<[u8; N], MessageProfileError> {
    if value.len() != N * 2
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(MessageProfileError::InvalidField);
    }
    let mut output = [0u8; N];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        output[index] = hex_digit(pair[0]) * 16 + hex_digit(pair[1]);
    }
    Ok(output)
}

fn hex_digit(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => 0,
    }
}

/// Return the exact authenticated header for conformance comparison.
pub fn message_authenticated_header(message: &EncryptedMessage) -> Vec<u8> {
    authenticated_message_header(message.header())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MESSAGE_ID: [u8; 16] = [
        0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x81, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd,
        0xef,
    ];
    const CHANNEL_ID: [u8; 16] = [
        0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde,
        0xf0,
    ];

    fn fields() -> MessageFields {
        MessageFields::new(
            MESSAGE_ID,
            1_725_000_000_000_000_000,
            b"originator".to_vec(),
            ChannelId(CHANNEL_ID),
            Sequence(42),
            KeyEpoch(3),
            "application/json;version=1".into(),
        )
    }

    fn message() -> (EncryptedMessage, OriginatorSigningKey, ChannelMasterKey) {
        let signing_key = OriginatorSigningKey::from_seed([0x11; 32]);
        let cmk = ChannelMasterKey::from_bytes([0x22; 32]);
        let message = message_create(fields(), br#"{"ok":true}"#, &signing_key, &cmk).unwrap();
        (message, signing_key, cmk)
    }

    #[test]
    fn json_and_binary_round_trip_losslessly() {
        let (message, signing_key, cmk) = message();
        let binary = message_serialize(&message).unwrap();
        let json = message_to_json(&message).unwrap();
        let decoded_json = message_from_json(&json).unwrap();
        let decoded_binary = message_deserialize(&binary).unwrap();

        assert_eq!(message_serialize(&decoded_json).unwrap(), binary);
        assert_eq!(message_to_json(&decoded_binary).unwrap(), json);
        assert_eq!(
            message_verify(&decoded_json, &signing_key.public_key(), &cmk).unwrap(),
            br#"{"ok":true}"#
        );
    }

    #[test]
    fn json_input_order_is_irrelevant_but_output_order_is_canonical() {
        let (message, _, _) = message();
        let canonical = String::from_utf8(message_to_json(&message).unwrap()).unwrap();
        let mut pairs = match from_ast(&try_parse_json(&canonical).unwrap()).unwrap() {
            JsonValue::Object(pairs) => pairs,
            _ => unreachable!(),
        };
        pairs.reverse();
        let reversed = serialize(&JsonValue::Object(pairs)).unwrap();
        let decoded = message_from_json(reversed.as_bytes()).unwrap();
        assert_eq!(
            String::from_utf8(message_to_json(&decoded).unwrap()).unwrap(),
            canonical
        );
    }

    #[test]
    fn rejects_noncanonical_json_encodings_and_keys() {
        let (message, _, _) = message();
        let canonical = String::from_utf8(message_to_json(&message).unwrap()).unwrap();
        let duplicate = canonical.replacen(
            "\"record_type\":\"D18M\"",
            "\"record_type\":\"D18M\",\"record_type\":\"D18M\"",
            1,
        );
        assert!(matches!(
            message_from_json(duplicate.as_bytes()),
            Err(MessageProfileError::InvalidJson)
        ));
        let leading_zero = canonical.replacen("\"sequence\":\"42\"", "\"sequence\":\"042\"", 1);
        assert!(matches!(
            message_from_json(leading_zero.as_bytes()),
            Err(MessageProfileError::InvalidField)
        ));
        let uppercase_uuid = canonical.replacen("018f47a0", "018F47A0", 1);
        assert!(matches!(
            message_from_json(uppercase_uuid.as_bytes()),
            Err(MessageProfileError::InvalidField)
        ));
    }

    #[test]
    fn validates_uuid_identity_mime_and_epoch_resolution_before_crypto() {
        let signing_key = OriginatorSigningKey::from_seed([0x11; 32]);
        let cmk = ChannelMasterKey::from_bytes([0x22; 32]);
        let invalid_uuid = MessageFields::new(
            [0; 16],
            1,
            b"originator".to_vec(),
            ChannelId(CHANNEL_ID),
            Sequence(1),
            KeyEpoch(3),
            "text/plain".into(),
        );
        assert!(matches!(
            message_create(invalid_uuid, b"x", &signing_key, &cmk),
            Err(MessageProfileError::InvalidField)
        ));

        let (message, _, _) = message();
        assert_eq!(
            message_verify_with_key_resolver(&message, &signing_key.public_key(), |_| None),
            Err(MessageProfileError::MissingEpochKey)
        );
        assert_eq!(
            validate_mime("multipart/related;boundary=\"chief;part\""),
            Ok(())
        );
        assert_eq!(
            validate_mime("text/plain\n"),
            Err(MessageProfileError::InvalidField)
        );
    }

    #[test]
    fn same_millisecond_uuid_v7_values_are_strictly_ordered() {
        let mut generator = MonotonicUuidV7Generator::new();
        let mut previous = [0; 16];
        for index in 0..1_000 {
            let current = generator.next(1_725_000_000_000, [0x55; 10]).unwrap();
            assert_eq!(UUID::from_bytes(current).version(), 7);
            assert_eq!(UUID::from_bytes(current).variant(), "rfc4122");
            if index > 0 {
                assert!(previous < current);
            }
            previous = current;
        }
    }

    #[test]
    fn wire_and_crypto_errors_have_stable_codes() {
        assert_eq!(
            MessageProfileError::from(wire::ChannelWireError::Truncated).code(),
            "truncated_record"
        );
        assert_eq!(
            MessageProfileError::from(ChannelCryptoError::InvalidMessageSignature).code(),
            "invalid_signature"
        );
    }
}
