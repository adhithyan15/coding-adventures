//! Bounded CoAP GET request and response framing.

#![forbid(unsafe_code)]

use std::fmt;

pub const VERSION: &str = "0.1.0";
pub const COAP_VERSION: u8 = 1;
pub const MAX_TOKEN_BYTES: usize = 8;
pub const MAX_PATH_SEGMENTS: usize = 16;
pub const MAX_PATH_BYTES: usize = 512;
pub const MAX_DATAGRAM_BYTES: usize = 8_192;
pub const CONTENT_FORMAT_TEXT_PLAIN: u16 = 0;
pub const CONTENT_FORMAT_APPLICATION_JSON: u16 = 50;

const CODE_EMPTY: u8 = 0;
const CODE_GET: u8 = 1;
const OPTION_ETAG: u32 = 4;
const OPTION_OBSERVE: u32 = 6;
const OPTION_LOCATION_PATH: u32 = 8;
const OPTION_URI_PATH: u32 = 11;
const OPTION_CONTENT_FORMAT: u32 = 12;
const OPTION_MAX_AGE: u32 = 14;
const OPTION_ACCEPT: u32 = 17;
const OPTION_LOCATION_QUERY: u32 = 20;
const OPTION_BLOCK2: u32 = 23;
const OPTION_SIZE2: u32 = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Confirmable,
    NonConfirmable,
    Acknowledgement,
    Reset,
}

impl MessageType {
    const fn wire(self) -> u8 {
        match self {
            Self::Confirmable => 0,
            Self::NonConfirmable => 1,
            Self::Acknowledgement => 2,
            Self::Reset => 3,
        }
    }

    fn from_wire(value: u8) -> Self {
        match value {
            0 => Self::Confirmable,
            1 => Self::NonConfirmable,
            2 => Self::Acknowledgement,
            _ => Self::Reset,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseCode {
    pub class: u8,
    pub detail: u8,
}

impl ResponseCode {
    pub const CONTENT: Self = Self {
        class: 2,
        detail: 5,
    };

    pub const fn is_success(self) -> bool {
        self.class == 2
    }

    pub const fn wire(self) -> u8 {
        (self.class << 5) | self.detail
    }
}

impl fmt::Display for ResponseCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{:02}", self.class, self.detail)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestContext {
    pub message_id: u16,
    pub token: Vec<u8>,
}

impl RequestContext {
    pub fn new(message_id: u16, token: Vec<u8>) -> Result<Self, CoapError> {
        if token.is_empty() || token.len() > MAX_TOKEN_BYTES {
            return Err(CoapError::InvalidTokenLength(token.len()));
        }
        Ok(Self { message_id, token })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetRequest {
    path_segments: Vec<String>,
    accept: Option<u16>,
}

impl GetRequest {
    pub fn new(path: &str) -> Result<Self, CoapError> {
        if !path.starts_with('/') || path.contains(['?', '#']) {
            return Err(CoapError::InvalidPath(
                "path must begin with `/` and contain no query or fragment".to_string(),
            ));
        }
        let segments = path[1..].split('/').collect::<Vec<_>>();
        if segments.is_empty()
            || segments.len() > MAX_PATH_SEGMENTS
            || segments.iter().any(|segment| segment.is_empty())
        {
            return Err(CoapError::InvalidPath(format!(
                "path must contain between 1 and {MAX_PATH_SEGMENTS} non-empty segments"
            )));
        }
        let total_bytes = segments.iter().map(|segment| segment.len()).sum::<usize>();
        if total_bytes > MAX_PATH_BYTES
            || segments.iter().any(|segment| {
                segment.len() > u8::MAX as usize
                    || segment.chars().any(|character| character.is_control())
            })
        {
            return Err(CoapError::InvalidPath(format!(
                "path segments must be control-free, at most 255 bytes each, and at most {MAX_PATH_BYTES} bytes total"
            )));
        }
        Ok(Self {
            path_segments: segments.into_iter().map(ToString::to_string).collect(),
            accept: None,
        })
    }

    pub fn with_accept(mut self, content_format: u16) -> Self {
        self.accept = Some(content_format);
        self
    }

    pub fn path(&self) -> String {
        format!("/{}", self.path_segments.join("/"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoapResponse {
    pub message_type: MessageType,
    pub message_id: u16,
    pub code: ResponseCode,
    pub content_format: Option<u16>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedResponse {
    EmptyAcknowledgement { message_id: u16 },
    Response(CoapResponse),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoapError {
    InvalidTokenLength(usize),
    InvalidPath(String),
    DatagramTooShort(usize),
    DatagramTooLarge { actual: usize, maximum: usize },
    UnsupportedVersion(u8),
    InvalidCode(u8),
    InvalidEmptyMessage,
    InvalidOptionNibble,
    TruncatedOption,
    OptionNumberOverflow,
    NonCanonicalOptionValue { option: u32 },
    DuplicateContentFormat,
    UnsupportedOption(u32),
    UnknownCriticalOption(u32),
    PayloadMarkerWithoutPayload,
    TokenMismatch,
    MessageIdMismatch { expected: u16, actual: u16 },
    ResetResponse,
}

impl fmt::Display for CoapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTokenLength(actual) => write!(
                formatter,
                "CoAP token length must be between 1 and {MAX_TOKEN_BYTES}, got {actual}"
            ),
            Self::InvalidPath(message) => write!(formatter, "invalid CoAP URI path: {message}"),
            Self::DatagramTooShort(actual) => {
                write!(formatter, "CoAP datagram is too short: {actual} bytes")
            }
            Self::DatagramTooLarge { actual, maximum } => write!(
                formatter,
                "CoAP datagram is {actual} bytes, exceeding the {maximum}-byte limit"
            ),
            Self::UnsupportedVersion(actual) => {
                write!(
                    formatter,
                    "CoAP version must be {COAP_VERSION}, got {actual}"
                )
            }
            Self::InvalidCode(code) => write!(formatter, "invalid CoAP response code 0x{code:02x}"),
            Self::InvalidEmptyMessage => write!(formatter, "malformed empty CoAP message"),
            Self::InvalidOptionNibble => write!(formatter, "CoAP option uses reserved nibble 15"),
            Self::TruncatedOption => write!(formatter, "truncated CoAP option"),
            Self::OptionNumberOverflow => write!(formatter, "CoAP option number overflow"),
            Self::NonCanonicalOptionValue { option } => write!(
                formatter,
                "CoAP option {option} uses a non-canonical unsigned value"
            ),
            Self::DuplicateContentFormat => {
                write!(formatter, "CoAP response repeats Content-Format")
            }
            Self::UnsupportedOption(option) => {
                write!(
                    formatter,
                    "CoAP option {option} is outside this bounded runtime"
                )
            }
            Self::UnknownCriticalOption(option) => {
                write!(formatter, "unknown critical CoAP option {option}")
            }
            Self::PayloadMarkerWithoutPayload => {
                write!(
                    formatter,
                    "CoAP payload marker is not followed by a payload"
                )
            }
            Self::TokenMismatch => write!(formatter, "CoAP response token mismatch"),
            Self::MessageIdMismatch { expected, actual } => write!(
                formatter,
                "CoAP acknowledgement message id mismatch: expected {expected}, got {actual}"
            ),
            Self::ResetResponse => write!(formatter, "CoAP peer reset the exchange"),
        }
    }
}

impl std::error::Error for CoapError {}

pub fn encode_confirmable_get(
    context: &RequestContext,
    request: &GetRequest,
) -> Result<Vec<u8>, CoapError> {
    if context.token.is_empty() || context.token.len() > MAX_TOKEN_BYTES {
        return Err(CoapError::InvalidTokenLength(context.token.len()));
    }
    let mut bytes = Vec::with_capacity(4 + context.token.len() + request.path().len() + 8);
    bytes.push(
        (COAP_VERSION << 6) | (MessageType::Confirmable.wire() << 4) | context.token.len() as u8,
    );
    bytes.push(CODE_GET);
    bytes.extend_from_slice(&context.message_id.to_be_bytes());
    bytes.extend_from_slice(&context.token);

    let mut previous = 0;
    for segment in &request.path_segments {
        encode_option(
            &mut bytes,
            &mut previous,
            OPTION_URI_PATH,
            segment.as_bytes(),
        )?;
    }
    if let Some(accept) = request.accept {
        let value = encode_uint(u64::from(accept));
        encode_option(&mut bytes, &mut previous, OPTION_ACCEPT, &value)?;
    }
    if bytes.len() > MAX_DATAGRAM_BYTES {
        return Err(CoapError::DatagramTooLarge {
            actual: bytes.len(),
            maximum: MAX_DATAGRAM_BYTES,
        });
    }
    Ok(bytes)
}

pub fn encode_empty_ack(message_id: u16) -> [u8; 4] {
    [
        (COAP_VERSION << 6) | (MessageType::Acknowledgement.wire() << 4),
        CODE_EMPTY,
        message_id.to_be_bytes()[0],
        message_id.to_be_bytes()[1],
    ]
}

pub fn decode_response(
    bytes: &[u8],
    context: &RequestContext,
) -> Result<DecodedResponse, CoapError> {
    if bytes.len() < 4 {
        return Err(CoapError::DatagramTooShort(bytes.len()));
    }
    if bytes.len() > MAX_DATAGRAM_BYTES {
        return Err(CoapError::DatagramTooLarge {
            actual: bytes.len(),
            maximum: MAX_DATAGRAM_BYTES,
        });
    }
    let version = bytes[0] >> 6;
    if version != COAP_VERSION {
        return Err(CoapError::UnsupportedVersion(version));
    }
    let message_type = MessageType::from_wire((bytes[0] >> 4) & 0x03);
    if message_type == MessageType::Reset {
        return Err(CoapError::ResetResponse);
    }
    let token_length = usize::from(bytes[0] & 0x0f);
    if token_length > MAX_TOKEN_BYTES || bytes.len() < 4 + token_length {
        return Err(CoapError::InvalidTokenLength(token_length));
    }
    let code = bytes[1];
    let message_id = u16::from_be_bytes([bytes[2], bytes[3]]);

    if code == CODE_EMPTY {
        if message_type != MessageType::Acknowledgement || token_length != 0 || bytes.len() != 4 {
            return Err(CoapError::InvalidEmptyMessage);
        }
        if message_id != context.message_id {
            return Err(CoapError::MessageIdMismatch {
                expected: context.message_id,
                actual: message_id,
            });
        }
        return Ok(DecodedResponse::EmptyAcknowledgement { message_id });
    }

    let response_code = ResponseCode {
        class: code >> 5,
        detail: code & 0x1f,
    };
    if !matches!(response_code.class, 2 | 4 | 5) {
        return Err(CoapError::InvalidCode(code));
    }
    if &bytes[4..4 + token_length] != context.token.as_slice() {
        return Err(CoapError::TokenMismatch);
    }
    if message_type == MessageType::Acknowledgement && message_id != context.message_id {
        return Err(CoapError::MessageIdMismatch {
            expected: context.message_id,
            actual: message_id,
        });
    }

    let (content_format, payload) = decode_options_and_payload(bytes, 4 + token_length)?;
    Ok(DecodedResponse::Response(CoapResponse {
        message_type,
        message_id,
        code: response_code,
        content_format,
        payload,
    }))
}

fn decode_options_and_payload(
    bytes: &[u8],
    mut cursor: usize,
) -> Result<(Option<u16>, Vec<u8>), CoapError> {
    let mut option_number = 0u32;
    let mut content_format = None;
    while cursor < bytes.len() {
        if bytes[cursor] == 0xff {
            cursor += 1;
            if cursor == bytes.len() {
                return Err(CoapError::PayloadMarkerWithoutPayload);
            }
            return Ok((content_format, bytes[cursor..].to_vec()));
        }
        let header = bytes[cursor];
        cursor += 1;
        let delta = decode_option_component(header >> 4, bytes, &mut cursor)?;
        let length = decode_option_component(header & 0x0f, bytes, &mut cursor)?;
        option_number = option_number
            .checked_add(delta as u32)
            .ok_or(CoapError::OptionNumberOverflow)?;
        let end = cursor
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or(CoapError::TruncatedOption)?;
        let value = &bytes[cursor..end];
        cursor = end;

        match option_number {
            OPTION_CONTENT_FORMAT => {
                if content_format.is_some() {
                    return Err(CoapError::DuplicateContentFormat);
                }
                let parsed = decode_uint(option_number, value)?;
                content_format = Some(u16::try_from(parsed).map_err(|_| {
                    CoapError::NonCanonicalOptionValue {
                        option: option_number,
                    }
                })?);
            }
            OPTION_OBSERVE | OPTION_BLOCK2 => {
                return Err(CoapError::UnsupportedOption(option_number));
            }
            OPTION_ETAG
            | OPTION_LOCATION_PATH
            | OPTION_MAX_AGE
            | OPTION_LOCATION_QUERY
            | OPTION_SIZE2 => {}
            unknown if unknown % 2 == 1 => {
                return Err(CoapError::UnknownCriticalOption(unknown));
            }
            _ => {}
        }
    }
    Ok((content_format, Vec::new()))
}

fn encode_option(
    bytes: &mut Vec<u8>,
    previous: &mut u32,
    number: u32,
    value: &[u8],
) -> Result<(), CoapError> {
    let delta = number
        .checked_sub(*previous)
        .ok_or(CoapError::OptionNumberOverflow)? as usize;
    let (delta_nibble, delta_extension) = encode_option_component(delta)?;
    let (length_nibble, length_extension) = encode_option_component(value.len())?;
    bytes.push((delta_nibble << 4) | length_nibble);
    bytes.extend_from_slice(&delta_extension);
    bytes.extend_from_slice(&length_extension);
    bytes.extend_from_slice(value);
    *previous = number;
    Ok(())
}

fn encode_option_component(value: usize) -> Result<(u8, Vec<u8>), CoapError> {
    match value {
        0..=12 => Ok((value as u8, Vec::new())),
        13..=268 => Ok((13, vec![(value - 13) as u8])),
        269..=65_804 => Ok((14, ((value - 269) as u16).to_be_bytes().to_vec())),
        _ => Err(CoapError::OptionNumberOverflow),
    }
}

fn decode_option_component(
    nibble: u8,
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<usize, CoapError> {
    match nibble {
        0..=12 => Ok(usize::from(nibble)),
        13 => {
            let value = *bytes.get(*cursor).ok_or(CoapError::TruncatedOption)?;
            *cursor += 1;
            Ok(usize::from(value) + 13)
        }
        14 => {
            let extension = bytes
                .get(*cursor..*cursor + 2)
                .ok_or(CoapError::TruncatedOption)?;
            *cursor += 2;
            Ok(usize::from(u16::from_be_bytes([extension[0], extension[1]])) + 269)
        }
        _ => Err(CoapError::InvalidOptionNibble),
    }
}

fn encode_uint(value: u64) -> Vec<u8> {
    if value == 0 {
        return Vec::new();
    }
    let bytes = value.to_be_bytes();
    bytes[bytes.iter().position(|byte| *byte != 0).unwrap_or(7)..].to_vec()
}

fn decode_uint(option: u32, value: &[u8]) -> Result<u64, CoapError> {
    if value.len() > 8 || (value.len() > 1 && value[0] == 0) {
        return Err(CoapError::NonCanonicalOptionValue { option });
    }
    Ok(value.iter().fold(0u64, |accumulator, byte| {
        (accumulator << 8) | u64::from(*byte)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> RequestContext {
        RequestContext::new(0x7d38, vec![0x53, 0x19]).unwrap()
    }

    #[test]
    fn encodes_confirmable_get_with_path_and_accept() {
        let request = GetRequest::new("/sensors/temperature")
            .unwrap()
            .with_accept(CONTENT_FORMAT_APPLICATION_JSON);
        assert_eq!(
            encode_confirmable_get(&context(), &request).unwrap(),
            [
                0x42, 0x01, 0x7d, 0x38, 0x53, 0x19, 0xb7, b's', b'e', b'n', b's', b'o', b'r', b's',
                0x0b, b't', b'e', b'm', b'p', b'e', b'r', b'a', b't', b'u', b'r', b'e', 0x61, 50,
            ]
        );
        assert_eq!(request.path(), "/sensors/temperature");
    }

    #[test]
    fn decodes_correlated_piggybacked_content() {
        let bytes = [
            0x62, 0x45, 0x7d, 0x38, 0x53, 0x19, 0xc1, 50, 0xff, b'{', b'}',
        ];
        let response = decode_response(&bytes, &context()).unwrap();
        assert_eq!(
            response,
            DecodedResponse::Response(CoapResponse {
                message_type: MessageType::Acknowledgement,
                message_id: 0x7d38,
                code: ResponseCode::CONTENT,
                content_format: Some(CONTENT_FORMAT_APPLICATION_JSON),
                payload: b"{}".to_vec(),
            })
        );
    }

    #[test]
    fn accepts_empty_ack_then_correlated_separate_response() {
        assert_eq!(
            decode_response(&[0x60, 0, 0x7d, 0x38], &context()).unwrap(),
            DecodedResponse::EmptyAcknowledgement { message_id: 0x7d38 }
        );
        let separate = [
            0x42, 0x45, 0xab, 0xcd, 0x53, 0x19, 0xc0, 0xff, b'2', b'1', b'.', b'5',
        ];
        let response = decode_response(&separate, &context()).unwrap();
        let DecodedResponse::Response(response) = response else {
            panic!("expected separate response");
        };
        assert_eq!(response.message_type, MessageType::Confirmable);
        assert_eq!(response.content_format, Some(CONTENT_FORMAT_TEXT_PLAIN));
        assert_eq!(response.payload, b"21.5");
        assert_eq!(encode_empty_ack(response.message_id), [0x60, 0, 0xab, 0xcd]);
    }

    #[test]
    fn rejects_cross_request_responses() {
        let wrong_token = [0x61, 0x45, 0x7d, 0x38, 0x99];
        assert_eq!(
            decode_response(&wrong_token, &context()),
            Err(CoapError::TokenMismatch)
        );
        let wrong_message = [0x62, 0x45, 0, 1, 0x53, 0x19];
        assert_eq!(
            decode_response(&wrong_message, &context()),
            Err(CoapError::MessageIdMismatch {
                expected: 0x7d38,
                actual: 1,
            })
        );
    }

    #[test]
    fn rejects_observe_blockwise_and_unknown_critical_options() {
        let observe = [0x62, 0x45, 0x7d, 0x38, 0x53, 0x19, 0x60];
        assert_eq!(
            decode_response(&observe, &context()),
            Err(CoapError::UnsupportedOption(OPTION_OBSERVE))
        );
        let block2 = [0x62, 0x45, 0x7d, 0x38, 0x53, 0x19, 0xd0, 10, 0xa0];
        assert_eq!(
            decode_response(&block2, &context()),
            Err(CoapError::UnsupportedOption(OPTION_BLOCK2))
        );
        let critical = [0x62, 0x45, 0x7d, 0x38, 0x53, 0x19, 0x10];
        assert_eq!(
            decode_response(&critical, &context()),
            Err(CoapError::UnknownCriticalOption(1))
        );
    }

    #[test]
    fn rejects_malformed_frames_and_paths() {
        assert!(GetRequest::new("temperature").is_err());
        assert!(GetRequest::new("/a//b").is_err());
        assert!(RequestContext::new(1, Vec::new()).is_err());
        assert_eq!(
            decode_response(&[0x40, 0x45, 0, 1], &context()),
            Err(CoapError::TokenMismatch)
        );
        assert_eq!(
            decode_response(&[0x62, 0x45, 0x7d, 0x38, 0x53, 0x19, 0xf0], &context()),
            Err(CoapError::InvalidOptionNibble)
        );
        assert_eq!(
            decode_response(&[0x62, 0x45, 0x7d, 0x38, 0x53, 0x19, 0xff], &context()),
            Err(CoapError::PayloadMarkerWithoutPayload)
        );
    }
}
