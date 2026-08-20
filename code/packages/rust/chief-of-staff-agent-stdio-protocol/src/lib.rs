//! Bounded JSON-lines protocol for D18 Level 4 any-language agents.
//!
//! This package is a pure codec. Process spawning, pipe I/O, publication, and
//! acknowledgement belong to the host adapter that injects these lines.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_json_parser::try_parse_json;
use coding_adventures_json_serializer::{serialize, JsonSerializerError};
use coding_adventures_json_value::{from_ast, JsonValue};
use core::fmt::{self, Display, Formatter};

/// Exact protocol discriminator carried by every Level 4 record.
pub const PROTOCOL: &str = "chief-agent-stdio-v1";
/// Maximum UTF-8 bytes in an identity or channel identifier.
pub const MAX_IDENTIFIER_BYTES: usize = 4 * 1024;
/// Maximum UTF-8 bytes in an authenticated MIME content type.
pub const MAX_CONTENT_TYPE_BYTES: usize = 1024;
/// Maximum decoded plaintext payload size.
pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
/// Maximum encoded JSON line size including its terminator.
pub const MAX_LINE_BYTES: usize = 90 * 1024 * 1024;

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Verified channel input delivered by the host to a Level 4 process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentInput {
    message_id: String,
    channel_id: String,
    sequence: u64,
    timestamp_ns: u64,
    content_type: String,
    payload: Vec<u8>,
}

impl AgentInput {
    /// Validate and construct one host-to-agent message record.
    pub fn new(
        message_id: impl Into<String>,
        channel_id: impl Into<String>,
        sequence: u64,
        timestamp_ns: u64,
        content_type: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self, ProtocolError> {
        let value = Self {
            message_id: message_id.into(),
            channel_id: channel_id.into(),
            sequence,
            timestamp_ns,
            content_type: content_type.into(),
            payload: payload.into(),
        };
        validate_identifier(&value.message_id, "message_id")?;
        validate_identifier(&value.channel_id, "channel_id")?;
        validate_content_type(&value.content_type)?;
        validate_payload(&value.payload)?;
        Ok(value)
    }

    /// Borrow the input message identity used for response correlation.
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    /// Borrow the authorized input channel identity.
    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }

    /// Return the durable channel sequence.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Return the authenticated monotonic timestamp in nanoseconds.
    pub fn timestamp_ns(&self) -> u64 {
        self.timestamp_ns
    }

    /// Borrow the authenticated MIME content type.
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Borrow the verified plaintext payload.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

/// Validated response returned by a Level 4 process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentResponse {
    input_message_id: String,
    content_type: String,
    payload: Vec<u8>,
}

impl AgentResponse {
    /// Validate and construct one agent-to-host response record.
    pub fn new(
        input_message_id: impl Into<String>,
        content_type: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self, ProtocolError> {
        let value = Self {
            input_message_id: input_message_id.into(),
            content_type: content_type.into(),
            payload: payload.into(),
        };
        validate_identifier(&value.input_message_id, "input_message_id")?;
        validate_content_type(&value.content_type)?;
        validate_payload(&value.payload)?;
        Ok(value)
    }

    /// Borrow the exact input identity this response completes.
    pub fn input_message_id(&self) -> &str {
        &self.input_message_id
    }

    /// Borrow the output MIME content type.
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Borrow the output plaintext payload.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

/// Fail-closed validation or codec error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    /// A field was absent, duplicated, unexpected, or structurally invalid.
    InvalidRecord(&'static str),
    /// A string or byte field exceeded its protocol bound.
    LimitExceeded(&'static str),
    /// The protocol discriminator is unsupported.
    UnsupportedProtocol,
    /// The record kind is invalid for the requested direction.
    WrongRecordKind,
    /// Base64 text was malformed, non-canonical, or decoded past the bound.
    InvalidBase64,
    /// A response named a message other than the one currently in flight.
    CorrelationMismatch,
    /// Repository JSON serialization rejected the record.
    Serialization,
}

impl Display for ProtocolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRecord(field) => write!(formatter, "agent-stdio: invalid {field}"),
            Self::LimitExceeded(field) => write!(formatter, "agent-stdio: {field} exceeds limit"),
            Self::UnsupportedProtocol => formatter.write_str("agent-stdio: unsupported protocol"),
            Self::WrongRecordKind => formatter.write_str("agent-stdio: wrong record kind"),
            Self::InvalidBase64 => formatter.write_str("agent-stdio: invalid canonical base64"),
            Self::CorrelationMismatch => {
                formatter.write_str("agent-stdio: response correlation mismatch")
            }
            Self::Serialization => formatter.write_str("agent-stdio: JSON serialization failed"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<JsonSerializerError> for ProtocolError {
    fn from(_: JsonSerializerError) -> Self {
        Self::Serialization
    }
}

/// Encode one validated host input as compact JSON followed by LF.
pub fn encode_input_line(input: &AgentInput) -> Result<String, ProtocolError> {
    let json = JsonValue::Object(vec![
        (
            "protocol".to_string(),
            JsonValue::String(PROTOCOL.to_string()),
        ),
        ("kind".to_string(), JsonValue::String("message".to_string())),
        (
            "message_id".to_string(),
            JsonValue::String(input.message_id.clone()),
        ),
        (
            "channel_id".to_string(),
            JsonValue::String(input.channel_id.clone()),
        ),
        (
            "sequence".to_string(),
            JsonValue::String(input.sequence.to_string()),
        ),
        (
            "timestamp_ns".to_string(),
            JsonValue::String(input.timestamp_ns.to_string()),
        ),
        (
            "content_type".to_string(),
            JsonValue::String(input.content_type.clone()),
        ),
        (
            "payload_b64".to_string(),
            JsonValue::String(encode_base64(&input.payload)),
        ),
    ]);
    let mut line = serialize(&json)?;
    line.push('\n');
    if line.len() > MAX_LINE_BYTES {
        return Err(ProtocolError::LimitExceeded("line"));
    }
    Ok(line)
}

/// Decode one LF- or CRLF-terminated response for the current input message.
pub fn decode_response_line(
    line: &str,
    expected_message_id: &str,
) -> Result<AgentResponse, ProtocolError> {
    validate_identifier(expected_message_id, "expected_message_id")?;
    if line.len() > MAX_LINE_BYTES {
        return Err(ProtocolError::LimitExceeded("line"));
    }
    let json = line
        .strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .ok_or(ProtocolError::InvalidRecord("line terminator"))?;
    if json.contains(['\n', '\r']) {
        return Err(ProtocolError::InvalidRecord("line framing"));
    }
    let ast = try_parse_json(json).map_err(|_| ProtocolError::InvalidRecord("JSON"))?;
    let value = from_ast(&ast).map_err(|_| ProtocolError::InvalidRecord("JSON"))?;
    let fields = match &value {
        JsonValue::Object(fields) => fields,
        _ => return Err(ProtocolError::InvalidRecord("response object")),
    };
    if fields.len() != 5 || has_duplicate_keys(fields) {
        return Err(ProtocolError::InvalidRecord("response fields"));
    }
    let protocol = string_field(fields, "protocol")?;
    if protocol != PROTOCOL {
        return Err(ProtocolError::UnsupportedProtocol);
    }
    if string_field(fields, "kind")? != "response" {
        return Err(ProtocolError::WrongRecordKind);
    }
    let input_message_id = string_field(fields, "input_message_id")?;
    if input_message_id != expected_message_id {
        return Err(ProtocolError::CorrelationMismatch);
    }
    let content_type = string_field(fields, "content_type")?;
    let payload_b64 = string_field(fields, "payload_b64")?;
    AgentResponse::new(input_message_id, content_type, decode_base64(payload_b64)?)
}

fn string_field<'a>(
    fields: &'a [(String, JsonValue)],
    name: &'static str,
) -> Result<&'a str, ProtocolError> {
    match fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
    {
        Some(JsonValue::String(value)) => Ok(value),
        _ => Err(ProtocolError::InvalidRecord(name)),
    }
}

fn has_duplicate_keys(fields: &[(String, JsonValue)]) -> bool {
    fields
        .iter()
        .enumerate()
        .any(|(index, (key, _))| fields[..index].iter().any(|(earlier, _)| earlier == key))
}

fn validate_identifier(value: &str, field: &'static str) -> Result<(), ProtocolError> {
    if value.is_empty() {
        return Err(ProtocolError::InvalidRecord(field));
    }
    if value.len() > MAX_IDENTIFIER_BYTES {
        return Err(ProtocolError::LimitExceeded(field));
    }
    Ok(())
}

fn validate_content_type(value: &str) -> Result<(), ProtocolError> {
    if value.is_empty() {
        return Err(ProtocolError::InvalidRecord("content_type"));
    }
    if value.len() > MAX_CONTENT_TYPE_BYTES {
        return Err(ProtocolError::LimitExceeded("content_type"));
    }
    Ok(())
}

fn validate_payload(value: &[u8]) -> Result<(), ProtocolError> {
    if value.len() > MAX_PAYLOAD_BYTES {
        return Err(ProtocolError::LimitExceeded("payload"));
    }
    Ok(())
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

fn decode_base64(value: &str) -> Result<Vec<u8>, ProtocolError> {
    if !value.len().is_multiple_of(4) || value.len() / 4 * 3 > MAX_PAYLOAD_BYTES + 2 {
        return Err(ProtocolError::InvalidBase64);
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
            return Err(ProtocolError::InvalidBase64);
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
    if output.len() > MAX_PAYLOAD_BYTES || encode_base64(&output) != value {
        return Err(ProtocolError::InvalidBase64);
    }
    Ok(output)
}

fn base64_digit(byte: u8) -> Result<u8, ProtocolError> {
    BASE64
        .iter()
        .position(|candidate| *candidate == byte)
        .map(|index| index as u8)
        .ok_or(ProtocolError::InvalidBase64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> AgentInput {
        AgentInput::new(
            "0198-input",
            "weather-requests",
            u64::MAX,
            9_007_199_254_740_993,
            "application/octet-stream",
            [0, 255, 128, 65],
        )
        .unwrap()
    }

    fn response_line(payload_b64: &str) -> String {
        format!(
            "{{\"protocol\":\"{PROTOCOL}\",\"kind\":\"response\",\"input_message_id\":\"0198-input\",\"content_type\":\"application/octet-stream\",\"payload_b64\":\"{payload_b64}\"}}\n"
        )
    }

    #[test]
    fn input_encoding_is_one_lossless_json_line() {
        let line = encode_input_line(&input()).unwrap();
        assert_eq!(line.matches('\n').count(), 1);
        assert!(line.ends_with('\n'));
        assert!(line.contains("\"sequence\":\"18446744073709551615\""));
        assert!(line.contains("\"timestamp_ns\":\"9007199254740993\""));
        assert!(line.contains("\"payload_b64\":\"AP+AQQ==\""));
        assert_eq!(input().message_id(), "0198-input");
        assert_eq!(input().channel_id(), "weather-requests");
        assert_eq!(input().sequence(), u64::MAX);
        assert_eq!(input().timestamp_ns(), 9_007_199_254_740_993);
        assert_eq!(input().content_type(), "application/octet-stream");
        assert_eq!(input().payload(), [0, 255, 128, 65]);
    }

    #[test]
    fn response_decoding_accepts_lf_and_crlf() {
        let response = decode_response_line(&response_line("AP+AQQ=="), "0198-input").unwrap();
        assert_eq!(response.input_message_id(), "0198-input");
        assert_eq!(response.content_type(), "application/octet-stream");
        assert_eq!(response.payload(), [0, 255, 128, 65]);

        let crlf = response_line("").replace("\n", "\r\n");
        assert_eq!(
            decode_response_line(&crlf, "0198-input").unwrap().payload(),
            b""
        );
    }

    #[test]
    fn response_constructor_and_error_text_are_stable() {
        let response = AgentResponse::new("input", "text/plain", b"ok".to_vec()).unwrap();
        assert_eq!(response.payload(), b"ok");
        assert!(ProtocolError::CorrelationMismatch
            .to_string()
            .contains("correlation"));
        assert!(ProtocolError::UnsupportedProtocol
            .to_string()
            .contains("unsupported"));
        assert!(ProtocolError::WrongRecordKind.to_string().contains("kind"));
        assert!(ProtocolError::Serialization
            .to_string()
            .contains("serialization"));
    }

    #[test]
    fn constructors_enforce_identity_content_and_payload_bounds() {
        assert!(matches!(
            AgentInput::new("", "channel", 0, 0, "text/plain", []),
            Err(ProtocolError::InvalidRecord("message_id"))
        ));
        assert!(matches!(
            AgentResponse::new("message", "", []),
            Err(ProtocolError::InvalidRecord("content_type"))
        ));
        assert!(matches!(
            AgentResponse::new("x".repeat(MAX_IDENTIFIER_BYTES + 1), "text/plain", []),
            Err(ProtocolError::LimitExceeded("input_message_id"))
        ));
        assert!(matches!(
            AgentResponse::new("message", "x".repeat(MAX_CONTENT_TYPE_BYTES + 1), []),
            Err(ProtocolError::LimitExceeded("content_type"))
        ));
        assert!(matches!(
            AgentResponse::new("message", "text/plain", vec![0; MAX_PAYLOAD_BYTES + 1]),
            Err(ProtocolError::LimitExceeded("payload"))
        ));
    }

    #[test]
    fn response_requires_exact_shape_protocol_kind_and_correlation() {
        let wrong_protocol = response_line("").replace(PROTOCOL, "future");
        assert_eq!(
            decode_response_line(&wrong_protocol, "0198-input"),
            Err(ProtocolError::UnsupportedProtocol)
        );
        let wrong_kind = response_line("").replace("response", "message");
        assert_eq!(
            decode_response_line(&wrong_kind, "0198-input"),
            Err(ProtocolError::WrongRecordKind)
        );
        assert_eq!(
            decode_response_line(&response_line(""), "different"),
            Err(ProtocolError::CorrelationMismatch)
        );
        let extra = response_line("").replace("}\n", ",\"extra\":true}\n");
        assert!(matches!(
            decode_response_line(&extra, "0198-input"),
            Err(ProtocolError::InvalidRecord("response fields"))
        ));
        let duplicate = response_line("").replace(
            "\"kind\":\"response\"",
            "\"kind\":\"response\",\"kind\":\"response\"",
        );
        assert!(matches!(
            decode_response_line(&duplicate, "0198-input"),
            Err(ProtocolError::InvalidRecord("response fields"))
        ));
    }

    #[test]
    fn malformed_json_fields_and_framing_fail_closed() {
        for line in [
            "not-json\n",
            "[]\n",
            "{}\n",
            "{\"protocol\":1,\"kind\":\"response\",\"input_message_id\":\"0198-input\",\"content_type\":\"text/plain\",\"payload_b64\":\"\"}\n",
            "{\"protocol\":\"chief-agent-stdio-v1\",\"kind\":\"response\",\"input_message_id\":\"0198-input\",\"content_type\":\"text/plain\",\"payload_b64\":\"\"}",
            "{}\n{}\n",
        ] {
            assert!(decode_response_line(line, "0198-input").is_err(), "{line:?}");
        }
        assert!(decode_response_line(&response_line(""), "").is_err());
    }

    #[test]
    fn canonical_base64_is_required() {
        for invalid in ["%%%", "A===", "AA=A", "AA==AAAA", "AB==", "AAB="] {
            assert_eq!(
                decode_response_line(&response_line(invalid), "0198-input"),
                Err(ProtocolError::InvalidBase64),
                "{invalid}"
            );
        }
    }

    #[test]
    fn base64_round_trips_all_tail_lengths() {
        for payload in [b"".as_slice(), b"a", b"ab", b"abc", b"abcd", b"abcde"] {
            let encoded = encode_base64(payload);
            assert_eq!(decode_base64(&encoded).unwrap(), payload);
        }
    }
}
