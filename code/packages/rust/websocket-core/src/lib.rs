//! Transport-independent RFC 6455 WebSocket protocol core.
//!
//! This crate owns bounded handshake, frame, fragmentation, and control-frame
//! semantics. It performs no network I/O and delegates random client mask-key
//! generation to its runtime caller.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_sha1::sum1;
use core::fmt::{self, Display, Formatter};
use http1::{parse_request_head, parse_response_head};
use http_core::{BodyKind, Header, HttpVersion};

const WEBSOCKET_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Maximum accepted HTTP upgrade head, including its terminating CRLF line.
pub const MAX_HANDSHAKE_BYTES: usize = 16 * 1024;

/// Typed, payload-free WebSocket protocol failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketError {
    /// A complete HTTP upgrade head has not arrived yet.
    IncompleteHandshake,
    /// The HTTP upgrade head exceeded the fixed protocol bound.
    HandshakeTooLarge,
    /// The HTTP upgrade syntax or required field set is invalid.
    InvalidHandshake,
    /// A caller-supplied host or request target could inject HTTP syntax.
    HeaderInjection,
    /// A Base64 value is not canonical padded RFC 4648 encoding.
    InvalidBase64,
    /// One or more reserved frame bits were set without an extension.
    ReservedBits,
    /// The frame opcode is reserved or unknown.
    InvalidOpcode,
    /// The peer or local encoder violated its required masking direction.
    MaskDirection,
    /// An extended frame length did not use its shortest wire encoding.
    NonCanonicalLength,
    /// A declared frame payload exceeds the configured frame bound.
    FrameTooLarge,
    /// A control frame is fragmented, oversized, or structurally invalid.
    InvalidControlFrame,
    /// A close status code or UTF-8 reason is invalid.
    InvalidCloseFrame,
    /// A complete text message is not valid UTF-8.
    InvalidUtf8,
    /// Data-frame fragmentation state is invalid.
    InvalidFragmentation,
    /// An assembled message exceeds the configured message bound.
    MessageTooLarge,
    /// Data arrived after the inbound close event.
    ClosedSession,
}

impl Display for WebSocketError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::IncompleteHandshake => "websocket: incomplete handshake",
            Self::HandshakeTooLarge => "websocket: handshake exceeds limit",
            Self::InvalidHandshake => "websocket: invalid handshake",
            Self::HeaderInjection => "websocket: unsafe handshake field",
            Self::InvalidBase64 => "websocket: invalid base64",
            Self::ReservedBits => "websocket: reserved frame bits set",
            Self::InvalidOpcode => "websocket: invalid frame opcode",
            Self::MaskDirection => "websocket: invalid frame mask direction",
            Self::NonCanonicalLength => "websocket: non-canonical frame length",
            Self::FrameTooLarge => "websocket: frame exceeds limit",
            Self::InvalidControlFrame => "websocket: invalid control frame",
            Self::InvalidCloseFrame => "websocket: invalid close frame",
            Self::InvalidUtf8 => "websocket: invalid text encoding",
            Self::InvalidFragmentation => "websocket: invalid fragmentation state",
            Self::MessageTooLarge => "websocket: message exceeds limit",
            Self::ClosedSession => "websocket: data after close",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for WebSocketError {}

/// Serialized client upgrade request and the accept value its response needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientHandshake {
    bytes: Vec<u8>,
    expected_accept: String,
}

impl ClientHandshake {
    /// Borrow the complete HTTP/1.1 client upgrade request.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Borrow the exact `Sec-WebSocket-Accept` value expected from the server.
    pub fn expected_accept(&self) -> &str {
        &self.expected_accept
    }
}

/// Validated server upgrade response and consumed request-head length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerHandshake {
    response: Vec<u8>,
    consumed: usize,
}

impl ServerHandshake {
    /// Borrow the complete HTTP 101 response.
    pub fn response(&self) -> &[u8] {
        &self.response
    }

    /// Return the request bytes consumed before any coalesced frame bytes.
    pub fn consumed(&self) -> usize {
        self.consumed
    }
}

/// Build a bounded client upgrade request from a caller-generated nonce.
pub fn build_client_request(
    host: &str,
    target: &str,
    nonce: [u8; 16],
) -> Result<ClientHandshake, WebSocketError> {
    validate_host(host)?;
    validate_target(target)?;
    let key = encode_base64(&nonce);
    let expected_accept = derive_accept(&key);
    let request = format!(
        "GET {target} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
    )
    .into_bytes();
    if request.len() > MAX_HANDSHAKE_BYTES {
        return Err(WebSocketError::HandshakeTooLarge);
    }
    Ok(ClientHandshake {
        bytes: request,
        expected_accept,
    })
}

/// Validate one server HTTP 101 response and return its consumed head length.
pub fn validate_client_response(
    input: &[u8],
    expected_accept: &str,
) -> Result<usize, WebSocketError> {
    let end = complete_head(input)?;
    let parsed =
        parse_response_head(&input[..end]).map_err(|_| WebSocketError::InvalidHandshake)?;
    validate_header_bytes(&parsed.head.headers)?;
    if parsed.head.version != http_1_1()
        || parsed.head.status != 101
        || parsed.body_kind != BodyKind::None
        || !contains_header_token(&parsed.head.headers, "Upgrade", "websocket")
        || !contains_header_token(&parsed.head.headers, "Connection", "Upgrade")
        || one_header(&parsed.head.headers, "Sec-WebSocket-Accept")? != expected_accept
        || has_header(&parsed.head.headers, "Content-Length")
        || has_header(&parsed.head.headers, "Transfer-Encoding")
        || has_header(&parsed.head.headers, "Sec-WebSocket-Extensions")
        || has_header(&parsed.head.headers, "Sec-WebSocket-Protocol")
    {
        return Err(WebSocketError::InvalidHandshake);
    }
    Ok(parsed.body_offset)
}

/// Validate a server-side upgrade request and construct its HTTP 101 response.
pub fn accept_server_request(input: &[u8]) -> Result<ServerHandshake, WebSocketError> {
    let end = complete_head(input)?;
    let parsed = parse_request_head(&input[..end]).map_err(|_| WebSocketError::InvalidHandshake)?;
    validate_header_bytes(&parsed.head.headers)?;
    if parsed.head.method != "GET"
        || parsed.head.version != http_1_1()
        || parsed.body_kind != BodyKind::None
        || one_header(&parsed.head.headers, "Host")?.is_empty()
        || !contains_header_token(&parsed.head.headers, "Upgrade", "websocket")
        || !contains_header_token(&parsed.head.headers, "Connection", "Upgrade")
        || one_header(&parsed.head.headers, "Sec-WebSocket-Version")? != "13"
        || has_header(&parsed.head.headers, "Content-Length")
        || has_header(&parsed.head.headers, "Transfer-Encoding")
        || has_header(&parsed.head.headers, "Sec-WebSocket-Extensions")
        || has_header(&parsed.head.headers, "Sec-WebSocket-Protocol")
    {
        return Err(WebSocketError::InvalidHandshake);
    }
    let key = one_header(&parsed.head.headers, "Sec-WebSocket-Key")?;
    validate_websocket_key(key)?;
    let accept = derive_accept(key);
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    )
    .into_bytes();
    Ok(ServerHandshake {
        response,
        consumed: parsed.body_offset,
    })
}

/// Derive RFC 6455's padded Base64 accept value from key header text.
pub fn derive_accept(key: &str) -> String {
    let mut source = Vec::with_capacity(key.len() + WEBSOCKET_GUID.len());
    source.extend_from_slice(key.as_bytes());
    source.extend_from_slice(WEBSOCKET_GUID);
    encode_base64(&sum1(&source))
}

fn http_1_1() -> HttpVersion {
    HttpVersion { major: 1, minor: 1 }
}

fn complete_head(input: &[u8]) -> Result<usize, WebSocketError> {
    if let Some(index) = input.windows(4).position(|window| window == b"\r\n\r\n") {
        let end = index + 4;
        if end > MAX_HANDSHAKE_BYTES {
            Err(WebSocketError::HandshakeTooLarge)
        } else {
            validate_available_crlf(&input[..end])?;
            Ok(end)
        }
    } else {
        validate_available_crlf(input)?;
        if input.len() >= MAX_HANDSHAKE_BYTES {
            Err(WebSocketError::HandshakeTooLarge)
        } else {
            Err(WebSocketError::IncompleteHandshake)
        }
    }
}

fn validate_available_crlf(input: &[u8]) -> Result<(), WebSocketError> {
    for (index, byte) in input.iter().copied().enumerate() {
        if byte == b'\n' && (index == 0 || input[index - 1] != b'\r') {
            return Err(WebSocketError::InvalidHandshake);
        }
        if byte == b'\r' && index + 1 < input.len() && input[index + 1] != b'\n' {
            return Err(WebSocketError::InvalidHandshake);
        }
    }
    Ok(())
}

fn validate_host(host: &str) -> Result<(), WebSocketError> {
    if host.is_empty() || !host.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
        return Err(WebSocketError::HeaderInjection);
    }
    Ok(())
}

fn validate_target(target: &str) -> Result<(), WebSocketError> {
    if !target.starts_with('/')
        || target.contains('#')
        || !target.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
    {
        return Err(WebSocketError::HeaderInjection);
    }
    Ok(())
}

fn validate_header_bytes(headers: &[Header]) -> Result<(), WebSocketError> {
    if headers.iter().any(|header| {
        header.name.bytes().any(|byte| byte <= 0x20 || byte >= 0x7f)
            || header
                .value
                .bytes()
                .any(|byte| byte < 0x20 && byte != b'\t' || byte == 0x7f)
    }) {
        return Err(WebSocketError::InvalidHandshake);
    }
    Ok(())
}

fn header_values<'a>(headers: &'a [Header], name: &str) -> Vec<&'a str> {
    headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str())
        .collect()
}

fn has_header(headers: &[Header], name: &str) -> bool {
    !header_values(headers, name).is_empty()
}

fn one_header<'a>(headers: &'a [Header], name: &str) -> Result<&'a str, WebSocketError> {
    let mut values = header_values(headers, name).into_iter();
    let value = values.next().ok_or(WebSocketError::InvalidHandshake)?;
    if values.next().is_some() {
        return Err(WebSocketError::InvalidHandshake);
    }
    Ok(value)
}

fn contains_header_token(headers: &[Header], name: &str, token: &str) -> bool {
    header_values(headers, name).into_iter().any(|value| {
        value
            .split(',')
            .map(str::trim)
            .any(|candidate| candidate.eq_ignore_ascii_case(token))
    })
}

fn validate_websocket_key(key: &str) -> Result<(), WebSocketError> {
    let decoded = decode_base64(key)?;
    if decoded.len() != 16 || encode_base64(&decoded) != key {
        return Err(WebSocketError::InvalidBase64);
    }
    Ok(())
}

fn encode_base64(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(ALPHABET[(first >> 2) as usize] as char);
        output.push(ALPHABET[(((first & 0x03) << 4) | (second >> 4)) as usize] as char);
        if chunk.len() >= 2 {
            output.push(ALPHABET[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() == 3 {
            output.push(ALPHABET[(third & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn decode_base64(input: &str) -> Result<Vec<u8>, WebSocketError> {
    if input.is_empty() || !input.len().is_multiple_of(4) {
        return Err(WebSocketError::InvalidBase64);
    }
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(input.len() / 4 * 3);
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        let last = index + 1 == input.len() / 4;
        let pad_two = chunk[2] == b'=';
        let pad_three = chunk[3] == b'=';
        if !last && (pad_two || pad_three) || pad_two && !pad_three {
            return Err(WebSocketError::InvalidBase64);
        }
        let first = base64_value(chunk[0])?;
        let second = base64_value(chunk[1])?;
        let third = if pad_two { 0 } else { base64_value(chunk[2])? };
        let fourth = if pad_three {
            0
        } else {
            base64_value(chunk[3])?
        };
        output.push((first << 2) | (second >> 4));
        if !pad_two {
            output.push((second << 4) | (third >> 2));
        }
        if !pad_three {
            output.push((third << 6) | fourth);
        }
    }
    Ok(output)
}

fn base64_value(byte: u8) -> Result<u8, WebSocketError> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(WebSocketError::InvalidBase64),
    }
}

/// Whether the local protocol endpoint is a WebSocket client or server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointRole {
    /// Sends masked frames and receives unmasked frames.
    Client,
    /// Sends unmasked frames and receives masked frames.
    Server,
}

/// RFC 6455 frame opcode supported without extensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    /// Continuation of an open fragmented data message.
    Continuation,
    /// UTF-8 text data.
    Text,
    /// Arbitrary binary data.
    Binary,
    /// Close control.
    Close,
    /// Ping control.
    Ping,
    /// Pong control.
    Pong,
}

impl Opcode {
    fn from_nibble(value: u8) -> Result<Self, WebSocketError> {
        match value {
            0x0 => Ok(Self::Continuation),
            0x1 => Ok(Self::Text),
            0x2 => Ok(Self::Binary),
            0x8 => Ok(Self::Close),
            0x9 => Ok(Self::Ping),
            0xa => Ok(Self::Pong),
            _ => Err(WebSocketError::InvalidOpcode),
        }
    }

    fn nibble(self) -> u8 {
        match self {
            Self::Continuation => 0x0,
            Self::Text => 0x1,
            Self::Binary => 0x2,
            Self::Close => 0x8,
            Self::Ping => 0x9,
            Self::Pong => 0xa,
        }
    }

    fn is_control(self) -> bool {
        matches!(self, Self::Close | Self::Ping | Self::Pong)
    }

    fn is_data(self) -> bool {
        matches!(self, Self::Text | Self::Binary | Self::Continuation)
    }
}

/// One validated, unmasked WebSocket frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    fin: bool,
    opcode: Opcode,
    payload: Vec<u8>,
}

impl Frame {
    /// Construct one frame and validate control-frame structure.
    pub fn new(
        fin: bool,
        opcode: Opcode,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self, WebSocketError> {
        let frame = Self {
            fin,
            opcode,
            payload: payload.into(),
        };
        validate_frame(&frame)?;
        Ok(frame)
    }

    /// Construct a final text frame.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            fin: true,
            opcode: Opcode::Text,
            payload: text.into().into_bytes(),
        }
    }

    /// Construct a final binary frame.
    pub fn binary(payload: impl Into<Vec<u8>>) -> Self {
        Self {
            fin: true,
            opcode: Opcode::Binary,
            payload: payload.into(),
        }
    }

    /// Construct a final ping frame.
    pub fn ping(payload: impl Into<Vec<u8>>) -> Result<Self, WebSocketError> {
        Self::new(true, Opcode::Ping, payload)
    }

    /// Construct a final pong frame.
    pub fn pong(payload: impl Into<Vec<u8>>) -> Result<Self, WebSocketError> {
        Self::new(true, Opcode::Pong, payload)
    }

    /// Construct a close frame with an optional status code and reason.
    pub fn close(code: Option<u16>, reason: &str) -> Result<Self, WebSocketError> {
        Self::new(true, Opcode::Close, encode_close_payload(code, reason)?)
    }

    /// Whether this frame finishes its data message.
    pub fn is_final(&self) -> bool {
        self.fin
    }

    /// Return this frame's opcode.
    pub fn opcode(&self) -> Opcode {
        self.opcode
    }

    /// Borrow the unmasked application payload.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

/// Serialize one outbound frame while enforcing the local role's mask rule.
pub fn encode_frame(
    role: EndpointRole,
    frame: &Frame,
    mask_key: Option<[u8; 4]>,
) -> Result<Vec<u8>, WebSocketError> {
    validate_frame(frame)?;
    let masked = match role {
        EndpointRole::Client => {
            if mask_key.is_none() {
                return Err(WebSocketError::MaskDirection);
            }
            true
        }
        EndpointRole::Server => {
            if mask_key.is_some() {
                return Err(WebSocketError::MaskDirection);
            }
            false
        }
    };
    let payload_len = frame.payload.len();
    let extension_len = if payload_len <= 125 {
        0
    } else if payload_len <= u16::MAX as usize {
        2
    } else {
        8
    };
    let mut output = Vec::with_capacity(2 + extension_len + usize::from(masked) * 4 + payload_len);
    output.push(if frame.fin { 0x80 } else { 0 } | frame.opcode.nibble());
    let mask_bit = if masked { 0x80 } else { 0 };
    match payload_len {
        0..=125 => output.push(mask_bit | payload_len as u8),
        126..=65535 => {
            output.push(mask_bit | 126);
            output.extend_from_slice(&(payload_len as u16).to_be_bytes());
        }
        _ => {
            output.push(mask_bit | 127);
            output.extend_from_slice(&(payload_len as u64).to_be_bytes());
        }
    }
    if let Some(key) = mask_key {
        output.extend_from_slice(&key);
        output.extend(
            frame
                .payload
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ key[index % 4]),
        );
    } else {
        output.extend_from_slice(&frame.payload);
    }
    Ok(output)
}

/// Split one complete text or binary message into bounded data frames.
pub fn fragment_message(
    opcode: Opcode,
    payload: &[u8],
    max_frame_payload: usize,
) -> Result<Vec<Frame>, WebSocketError> {
    if !matches!(opcode, Opcode::Text | Opcode::Binary) || max_frame_payload == 0 {
        return Err(WebSocketError::InvalidFragmentation);
    }
    if opcode == Opcode::Text && core::str::from_utf8(payload).is_err() {
        return Err(WebSocketError::InvalidUtf8);
    }
    if payload.is_empty() {
        return Ok(vec![Frame::new(true, opcode, Vec::new())?]);
    }
    let chunks = payload.chunks(max_frame_payload).collect::<Vec<_>>();
    let last_index = chunks.len() - 1;
    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            Frame::new(
                index == last_index,
                if index == 0 {
                    opcode
                } else {
                    Opcode::Continuation
                },
                chunk.to_vec(),
            )
        })
        .collect()
}

fn validate_frame(frame: &Frame) -> Result<(), WebSocketError> {
    if frame.opcode.is_control() && (!frame.fin || frame.payload.len() > 125) {
        return Err(WebSocketError::InvalidControlFrame);
    }
    if frame.opcode == Opcode::Close {
        decode_close_payload(&frame.payload)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ParsedFrameHeader {
    fin: bool,
    opcode: Opcode,
    header_len: usize,
    payload_len: usize,
    mask_key: Option<[u8; 4]>,
}

/// Incremental inbound frame decoder with a fixed per-frame payload bound.
pub struct FrameDecoder {
    role: EndpointRole,
    max_frame_payload: usize,
    buffer: Vec<u8>,
}

impl FrameDecoder {
    /// Construct an empty decoder for frames received by this local role.
    pub fn new(role: EndpointRole, max_frame_payload: usize) -> Self {
        Self {
            role,
            max_frame_payload,
            buffer: Vec::new(),
        }
    }

    /// Return bytes retained for one incomplete frame.
    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }

    /// Add arbitrary stream bytes and return every newly completed frame.
    pub fn push(&mut self, input: &[u8]) -> Result<Vec<Frame>, WebSocketError> {
        let mut cursor = 0;
        let mut frames = Vec::new();
        loop {
            if self.buffer.len() < 2 {
                if cursor == input.len() {
                    break;
                }
                fill_to(&mut self.buffer, input, &mut cursor, 2);
                continue;
            }
            let header_len = declared_header_len(&self.buffer);
            if self.buffer.len() < header_len {
                if cursor == input.len() {
                    break;
                }
                fill_to(&mut self.buffer, input, &mut cursor, header_len);
                continue;
            }
            let header = parse_frame_header(&self.buffer[..header_len], self.role)?;
            if header.payload_len > self.max_frame_payload {
                return Err(WebSocketError::FrameTooLarge);
            }
            let total = header
                .header_len
                .checked_add(header.payload_len)
                .ok_or(WebSocketError::FrameTooLarge)?;
            if self.buffer.len() < total {
                if cursor == input.len() {
                    break;
                }
                fill_to(&mut self.buffer, input, &mut cursor, total);
                continue;
            }
            let mut payload = self.buffer[header.header_len..total].to_vec();
            if let Some(key) = header.mask_key {
                for (index, byte) in payload.iter_mut().enumerate() {
                    *byte ^= key[index % 4];
                }
            }
            let frame = Frame {
                fin: header.fin,
                opcode: header.opcode,
                payload,
            };
            validate_frame(&frame)?;
            frames.push(frame);
            self.buffer.clear();
        }
        Ok(frames)
    }
}

fn fill_to(buffer: &mut Vec<u8>, input: &[u8], cursor: &mut usize, target: usize) {
    let count = (target - buffer.len()).min(input.len() - *cursor);
    buffer.extend_from_slice(&input[*cursor..*cursor + count]);
    *cursor += count;
}

fn declared_header_len(prefix: &[u8]) -> usize {
    let extended = match prefix[1] & 0x7f {
        126 => 2,
        127 => 8,
        _ => 0,
    };
    2 + extended + if prefix[1] & 0x80 != 0 { 4 } else { 0 }
}

fn parse_frame_header(
    header: &[u8],
    role: EndpointRole,
) -> Result<ParsedFrameHeader, WebSocketError> {
    if header[0] & 0x70 != 0 {
        return Err(WebSocketError::ReservedBits);
    }
    let fin = header[0] & 0x80 != 0;
    let opcode = Opcode::from_nibble(header[0] & 0x0f)?;
    let masked = header[1] & 0x80 != 0;
    let expected_mask = role == EndpointRole::Server;
    if masked != expected_mask {
        return Err(WebSocketError::MaskDirection);
    }
    let short = header[1] & 0x7f;
    let mut offset = 2;
    let payload_u64 = match short {
        126 => {
            let length = u16::from_be_bytes([header[offset], header[offset + 1]]) as u64;
            offset += 2;
            if length < 126 {
                return Err(WebSocketError::NonCanonicalLength);
            }
            length
        }
        127 => {
            let length = u64::from_be_bytes(
                header[offset..offset + 8]
                    .try_into()
                    .expect("validated frame header length"),
            );
            offset += 8;
            if length < 65_536 || length & (1 << 63) != 0 {
                return Err(WebSocketError::NonCanonicalLength);
            }
            length
        }
        length => u64::from(length),
    };
    let payload_len = usize::try_from(payload_u64).map_err(|_| WebSocketError::FrameTooLarge)?;
    if opcode.is_control() && (!fin || payload_len > 125) {
        return Err(WebSocketError::InvalidControlFrame);
    }
    let mask_key = if masked {
        let key = header[offset..offset + 4]
            .try_into()
            .expect("validated frame mask length");
        Some(key)
    } else {
        None
    };
    Ok(ParsedFrameHeader {
        fin,
        opcode,
        header_len: header.len(),
        payload_len,
        mask_key,
    })
}

/// Validated close status and reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloseMessage {
    code: Option<u16>,
    reason: String,
}

impl CloseMessage {
    /// Return the peer-supplied close status, when present.
    pub fn code(&self) -> Option<u16> {
        self.code
    }

    /// Borrow the validated UTF-8 close reason.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// One complete inbound data or control event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageEvent {
    /// Complete validated UTF-8 text.
    Text(String),
    /// Complete arbitrary binary data.
    Binary(Vec<u8>),
    /// Ping control application bytes.
    Ping(Vec<u8>),
    /// Pong control application bytes.
    Pong(Vec<u8>),
    /// Close control status and reason.
    Close(CloseMessage),
}

/// Bounded data-message fragmentation and control-event assembler.
pub struct MessageAssembler {
    max_message_payload: usize,
    fragment: Option<(Opcode, Vec<u8>)>,
    closed: bool,
}

impl MessageAssembler {
    /// Construct an empty assembler with a fixed complete-message bound.
    pub fn new(max_message_payload: usize) -> Self {
        Self {
            max_message_payload,
            fragment: None,
            closed: false,
        }
    }

    /// Whether a non-final text or binary message is currently open.
    pub fn is_fragmented(&self) -> bool {
        self.fragment.is_some()
    }

    /// Whether an inbound close event has been accepted.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Consume one validated frame and possibly emit one complete event.
    pub fn push(&mut self, frame: Frame) -> Result<Option<MessageEvent>, WebSocketError> {
        validate_frame(&frame)?;
        if self.closed && frame.opcode.is_data() {
            return Err(WebSocketError::ClosedSession);
        }
        match frame.opcode {
            Opcode::Ping => Ok(Some(MessageEvent::Ping(frame.payload))),
            Opcode::Pong => Ok(Some(MessageEvent::Pong(frame.payload))),
            Opcode::Close => {
                let close = decode_close_payload(&frame.payload)?;
                self.fragment = None;
                self.closed = true;
                Ok(Some(MessageEvent::Close(close)))
            }
            Opcode::Text | Opcode::Binary if self.fragment.is_some() => {
                Err(WebSocketError::InvalidFragmentation)
            }
            Opcode::Text | Opcode::Binary if frame.fin => {
                self.ensure_message_bound(frame.payload.len())?;
                complete_data_event(frame.opcode, frame.payload).map(Some)
            }
            Opcode::Text | Opcode::Binary => {
                self.ensure_message_bound(frame.payload.len())?;
                self.fragment = Some((frame.opcode, frame.payload));
                Ok(None)
            }
            Opcode::Continuation => {
                let (opcode, mut payload) = self
                    .fragment
                    .take()
                    .ok_or(WebSocketError::InvalidFragmentation)?;
                let total = payload
                    .len()
                    .checked_add(frame.payload.len())
                    .ok_or(WebSocketError::MessageTooLarge)?;
                self.ensure_message_bound(total)?;
                payload.extend_from_slice(&frame.payload);
                if frame.fin {
                    complete_data_event(opcode, payload).map(Some)
                } else {
                    self.fragment = Some((opcode, payload));
                    Ok(None)
                }
            }
        }
    }

    fn ensure_message_bound(&self, length: usize) -> Result<(), WebSocketError> {
        if length > self.max_message_payload {
            Err(WebSocketError::MessageTooLarge)
        } else {
            Ok(())
        }
    }
}

/// Construct the RFC-required automatic reply for ping or close events.
pub fn control_reply(event: &MessageEvent) -> Option<Frame> {
    match event {
        MessageEvent::Ping(payload) => Some(Frame {
            fin: true,
            opcode: Opcode::Pong,
            payload: payload.clone(),
        }),
        MessageEvent::Close(close) => Some(Frame {
            fin: true,
            opcode: Opcode::Close,
            payload: encode_close_payload(close.code, &close.reason)
                .expect("validated close event remains valid"),
        }),
        MessageEvent::Text(_) | MessageEvent::Binary(_) | MessageEvent::Pong(_) => None,
    }
}

fn complete_data_event(opcode: Opcode, payload: Vec<u8>) -> Result<MessageEvent, WebSocketError> {
    match opcode {
        Opcode::Text => String::from_utf8(payload)
            .map(MessageEvent::Text)
            .map_err(|_| WebSocketError::InvalidUtf8),
        Opcode::Binary => Ok(MessageEvent::Binary(payload)),
        _ => Err(WebSocketError::InvalidFragmentation),
    }
}

fn decode_close_payload(payload: &[u8]) -> Result<CloseMessage, WebSocketError> {
    if payload.is_empty() {
        return Ok(CloseMessage {
            code: None,
            reason: String::new(),
        });
    }
    if payload.len() == 1 {
        return Err(WebSocketError::InvalidCloseFrame);
    }
    let code = u16::from_be_bytes([payload[0], payload[1]]);
    if !valid_close_code(code) {
        return Err(WebSocketError::InvalidCloseFrame);
    }
    let reason = core::str::from_utf8(&payload[2..])
        .map_err(|_| WebSocketError::InvalidCloseFrame)?
        .to_string();
    Ok(CloseMessage {
        code: Some(code),
        reason,
    })
}

fn encode_close_payload(code: Option<u16>, reason: &str) -> Result<Vec<u8>, WebSocketError> {
    match code {
        None if reason.is_empty() => Ok(Vec::new()),
        None => Err(WebSocketError::InvalidCloseFrame),
        Some(code) if !valid_close_code(code) || reason.len() > 123 => {
            Err(WebSocketError::InvalidCloseFrame)
        }
        Some(code) => {
            let mut payload = Vec::with_capacity(2 + reason.len());
            payload.extend_from_slice(&code.to_be_bytes());
            payload.extend_from_slice(reason.as_bytes());
            Ok(payload)
        }
    }
}

fn valid_close_code(code: u16) -> bool {
    matches!(
        code,
        1000 | 1001 | 1002 | 1003 | 1007 | 1008 | 1009 | 1010 | 1011 | 1012 | 1013 | 1014
    ) || (3000..=4999).contains(&code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC_KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";
    const RFC_ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

    fn request_with(lines: &[&str]) -> Vec<u8> {
        let mut request = String::from("GET /chat HTTP/1.1\r\n");
        for line in lines {
            request.push_str(line);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        request.into_bytes()
    }

    fn valid_request() -> Vec<u8> {
        request_with(&[
            "Host: server.example.com",
            "Upgrade: websocket",
            "Connection: keep-alive, Upgrade",
            "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==",
            "Sec-WebSocket-Version: 13",
        ])
    }

    fn valid_response(accept: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        )
        .into_bytes()
    }

    #[test]
    fn rfc_accept_example_and_server_handshake_preserve_coalesced_bytes() {
        assert_eq!(derive_accept(RFC_KEY), RFC_ACCEPT);
        let mut input = valid_request();
        let consumed = input.len();
        input.extend_from_slice(&[0x81, 0x01, b'\n']);
        let handshake = accept_server_request(&input).expect("valid server handshake");
        assert_eq!(handshake.consumed(), consumed);
        assert_eq!(&input[handshake.consumed()..], [0x81, 0x01, b'\n']);
        assert_eq!(handshake.response(), valid_response(RFC_ACCEPT));
    }

    #[test]
    fn client_request_and_response_round_trip() {
        let nonce = *b"the sample nonce";
        let client = build_client_request("server.example.com", "/chat?room=1", nonce)
            .expect("valid client request");
        let request = core::str::from_utf8(client.bytes()).expect("ASCII request");
        assert!(request.starts_with("GET /chat?room=1 HTTP/1.1\r\n"));
        assert!(request.contains(&format!("Sec-WebSocket-Key: {RFC_KEY}\r\n")));
        assert_eq!(client.expected_accept(), RFC_ACCEPT);

        let mut response = valid_response(client.expected_accept());
        let consumed = response.len();
        response.extend_from_slice(&[0x81, 0x00]);
        assert_eq!(
            validate_client_response(&response, client.expected_accept()).expect("valid response"),
            consumed
        );
    }

    #[test]
    fn handshake_is_incremental_strict_crlf_and_bounded() {
        assert_eq!(
            accept_server_request(b"GET / HTTP/1.1\r\n"),
            Err(WebSocketError::IncompleteHandshake)
        );
        assert_eq!(
            accept_server_request(b"GET / HTTP/1.1\n\n"),
            Err(WebSocketError::InvalidHandshake)
        );
        assert_eq!(
            accept_server_request(b"GET / HTTP/1.1\rX"),
            Err(WebSocketError::InvalidHandshake)
        );
        assert_eq!(
            accept_server_request(&vec![b'a'; MAX_HANDSHAKE_BYTES]),
            Err(WebSocketError::HandshakeTooLarge)
        );
        let mut oversized = vec![b'a'; MAX_HANDSHAKE_BYTES + 1];
        oversized.extend_from_slice(b"\r\n\r\n");
        assert_eq!(
            accept_server_request(&oversized),
            Err(WebSocketError::HandshakeTooLarge)
        );
    }

    #[test]
    fn client_builder_rejects_field_injection_and_oversize() {
        for (host, target) in [
            ("", "/"),
            ("example.com\r\nX: y", "/"),
            ("example.com", "relative"),
            ("example.com", "/bad path"),
            ("example.com", "/path#fragment"),
            ("éxample.com", "/"),
        ] {
            assert_eq!(
                build_client_request(host, target, [0; 16]),
                Err(WebSocketError::HeaderInjection)
            );
        }
        let huge_target = format!("/{}", "a".repeat(MAX_HANDSHAKE_BYTES));
        assert_eq!(
            build_client_request("example.com", &huge_target, [0; 16]),
            Err(WebSocketError::HandshakeTooLarge)
        );
    }

    #[test]
    fn server_rejects_missing_conflicting_and_unsupported_upgrade_fields() {
        let cases = vec![
            b"POST /chat HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n".to_vec(),
            b"GET /chat HTTP/1.0\r\nHost: x\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n".to_vec(),
            request_with(&["Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13"]),
            request_with(&["Host: x", "Host: y", "Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13"]),
            request_with(&["Host: x", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13"]),
            request_with(&["Host: x", "Upgrade: h2c", "Connection: keep-alive", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13"]),
            request_with(&["Host: x", "Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Version: 13"]),
            request_with(&["Host: x", "Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 12"]),
            request_with(&["Host: x", "Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13", "Sec-WebSocket-Version: 13"]),
            request_with(&["Host: x", "Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13", "Sec-WebSocket-Extensions: permessage-deflate"]),
            request_with(&["Host: x", "Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13", "Sec-WebSocket-Protocol: chat"]),
            request_with(&["Host: x", "Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13", "Content-Length: 0"]),
            request_with(&["Host: x", "Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13", "Transfer-Encoding: chunked"]),
            request_with(&["Host: x\u{7f}", "Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13"]),
        ];
        for request in cases {
            assert!(
                accept_server_request(&request).is_err(),
                "request must fail"
            );
        }
    }

    #[test]
    fn server_requires_one_canonical_sixteen_byte_key() {
        for key in [
            "not-base64",
            "YQ==",
            "dGhlIHNhbXBsZSBub25jZQ=",
            "dGhlIHNhbXBsZSBub25jZR==",
            "dGhlIHNhbXBsZSBub25jZQ!!",
        ] {
            let request = request_with(&[
                "Host: x",
                "Upgrade: websocket",
                "Connection: Upgrade",
                &format!("Sec-WebSocket-Key: {key}"),
                "Sec-WebSocket-Version: 13",
            ]);
            assert!(matches!(
                accept_server_request(&request),
                Err(WebSocketError::InvalidBase64)
            ));
        }
        let duplicate = request_with(&[
            "Host: x",
            "Upgrade: websocket",
            "Connection: Upgrade",
            "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==",
            "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==",
            "Sec-WebSocket-Version: 13",
        ]);
        assert_eq!(
            accept_server_request(&duplicate),
            Err(WebSocketError::InvalidHandshake)
        );
        assert_eq!(decode_base64("+///").unwrap(), [0xfb, 0xff, 0xff]);
        assert_eq!(
            decode_base64("YQ==YQ=="),
            Err(WebSocketError::InvalidBase64)
        );
    }

    #[test]
    fn client_rejects_invalid_upgrade_responses() {
        let cases = [
            "HTTP/1.0 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n",
            "HTTP/1.1 200 OK\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n",
            "HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n",
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: close\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n",
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: wrong\r\n\r\n",
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\nSec-WebSocket-Extensions: x\r\n\r\n",
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\nSec-WebSocket-Protocol: chat\r\n\r\n",
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\nTransfer-Encoding: chunked\r\n\r\n",
        ];
        for response in cases {
            assert_eq!(
                validate_client_response(response.as_bytes(), RFC_ACCEPT),
                Err(WebSocketError::InvalidHandshake)
            );
        }
        assert_eq!(
            validate_client_response(b"HTTP/1.1 101 Switching Protocols\r\n", RFC_ACCEPT),
            Err(WebSocketError::IncompleteHandshake)
        );
    }

    fn round_trip(role: EndpointRole, frame: &Frame, key: Option<[u8; 4]>) -> Frame {
        let wire = encode_frame(role, frame, key).expect("encode frame");
        let peer = match role {
            EndpointRole::Client => EndpointRole::Server,
            EndpointRole::Server => EndpointRole::Client,
        };
        let mut decoder = FrameDecoder::new(peer, frame.payload.len().max(1));
        decoder.push(&wire).expect("decode frame").remove(0)
    }

    #[test]
    fn frame_round_trips_small_extended_and_large_lengths() {
        for length in [0, 1, 125, 126, 65_535, 65_536] {
            let frame = Frame::binary(vec![0x5a; length]);
            assert_eq!(round_trip(EndpointRole::Server, &frame, None), frame);
            assert_eq!(
                round_trip(EndpointRole::Client, &frame, Some([1, 2, 3, 4])),
                frame
            );
        }
    }

    #[test]
    fn rfc_masked_hello_example_decodes_exactly() {
        let wire = [
            0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
        ];
        let frames = FrameDecoder::new(EndpointRole::Server, 5)
            .push(&wire)
            .expect("decode RFC masked frame");
        assert_eq!(frames, [Frame::text("Hello")]);
        assert_eq!(
            encode_frame(EndpointRole::Server, &Frame::text("Hello"), None).unwrap(),
            b"\x81\x05Hello"
        );
    }

    #[test]
    fn decoder_accepts_bytewise_input_and_multiple_frames() {
        let first = Frame::text("hello");
        let second = Frame::binary([1, 2, 3]);
        let mut wire = encode_frame(EndpointRole::Server, &first, None).unwrap();
        wire.extend_from_slice(&encode_frame(EndpointRole::Server, &second, None).unwrap());
        let mut decoder = FrameDecoder::new(EndpointRole::Client, 100);
        let mut decoded = Vec::new();
        for byte in &wire {
            decoded.extend(decoder.push(core::slice::from_ref(byte)).unwrap());
        }
        assert_eq!(decoded, [first, second]);
        assert_eq!(decoder.buffered_len(), 0);

        let partial = encode_frame(EndpointRole::Server, &Frame::text("partial"), None).unwrap();
        assert!(decoder.push(&partial[..3]).unwrap().is_empty());
        assert_eq!(decoder.buffered_len(), 3);
        assert_eq!(decoder.push(&partial[3..]).unwrap().len(), 1);

        let mut masked_extended = FrameDecoder::new(EndpointRole::Server, 126);
        assert!(masked_extended.push(&[0x82, 0xfe, 0]).unwrap().is_empty());
        assert_eq!(masked_extended.buffered_len(), 3);
    }

    #[test]
    fn every_control_and_fragment_opcode_round_trips_on_wire() {
        let frames = [
            Frame::new(false, Opcode::Continuation, [1]).unwrap(),
            Frame::ping([2]).unwrap(),
            Frame::pong([3]).unwrap(),
            Frame::close(Some(1000), "done").unwrap(),
        ];
        for frame in frames {
            assert_eq!(round_trip(EndpointRole::Server, &frame, None), frame);
            assert_eq!(
                round_trip(EndpointRole::Client, &frame, Some([4, 3, 2, 1])),
                frame
            );
        }
    }

    #[test]
    fn encoder_and_decoder_enforce_mask_direction() {
        let frame = Frame::text("hello");
        assert_eq!(
            encode_frame(EndpointRole::Client, &frame, None),
            Err(WebSocketError::MaskDirection)
        );
        assert_eq!(
            encode_frame(EndpointRole::Server, &frame, Some([0; 4])),
            Err(WebSocketError::MaskDirection)
        );
        let server_wire = encode_frame(EndpointRole::Server, &frame, None).unwrap();
        assert_eq!(
            FrameDecoder::new(EndpointRole::Server, 10).push(&server_wire),
            Err(WebSocketError::MaskDirection)
        );
        let client_wire = encode_frame(EndpointRole::Client, &frame, Some([9; 4])).unwrap();
        assert_eq!(
            FrameDecoder::new(EndpointRole::Client, 10).push(&client_wire),
            Err(WebSocketError::MaskDirection)
        );
    }

    #[test]
    fn decoder_rejects_reserved_opcode_and_noncanonical_lengths() {
        let cases = [
            (vec![0xc1, 0], WebSocketError::ReservedBits),
            (vec![0x83, 0], WebSocketError::InvalidOpcode),
            (vec![0x82, 126, 0, 125], WebSocketError::NonCanonicalLength),
            (
                vec![0x82, 127, 0, 0, 0, 0, 0, 0, 0xff, 0xff],
                WebSocketError::NonCanonicalLength,
            ),
            (
                vec![0x82, 127, 0x80, 0, 0, 0, 0, 1, 0, 0],
                WebSocketError::NonCanonicalLength,
            ),
            (vec![0x09, 0], WebSocketError::InvalidControlFrame),
            (vec![0x89, 126, 0, 126], WebSocketError::InvalidControlFrame),
        ];
        for (wire, expected) in cases {
            assert_eq!(
                FrameDecoder::new(EndpointRole::Client, usize::MAX).push(&wire),
                Err(expected)
            );
        }
    }

    #[test]
    fn decoder_rejects_declared_oversize_before_payload_arrives() {
        let mut decoder = FrameDecoder::new(EndpointRole::Client, 4);
        assert_eq!(decoder.push(&[0x82, 5]), Err(WebSocketError::FrameTooLarge));
        let mut decoder = FrameDecoder::new(EndpointRole::Client, 125);
        assert_eq!(
            decoder.push(&[0x82, 126, 0, 126]),
            Err(WebSocketError::FrameTooLarge)
        );
    }

    #[test]
    fn frame_constructors_validate_controls_and_close_payloads() {
        assert_eq!(
            Frame::new(false, Opcode::Ping, []),
            Err(WebSocketError::InvalidControlFrame)
        );
        assert_eq!(
            Frame::ping(vec![0; 126]),
            Err(WebSocketError::InvalidControlFrame)
        );
        assert_eq!(
            Frame::new(true, Opcode::Close, [1]),
            Err(WebSocketError::InvalidCloseFrame)
        );
        assert_eq!(
            Frame::close(Some(1005), "reserved"),
            Err(WebSocketError::InvalidCloseFrame)
        );
        assert_eq!(
            Frame::new(true, Opcode::Close, 1005_u16.to_be_bytes()),
            Err(WebSocketError::InvalidCloseFrame)
        );
        assert_eq!(
            Frame::close(None, "reason without code"),
            Err(WebSocketError::InvalidCloseFrame)
        );
        assert_eq!(
            Frame::close(Some(1000), &"x".repeat(124)),
            Err(WebSocketError::InvalidCloseFrame)
        );
        assert_eq!(
            Frame::new(true, Opcode::Close, [0x03, 0xe8, 0xff]),
            Err(WebSocketError::InvalidCloseFrame)
        );
        assert!(Frame::pong([1, 2]).is_ok());
        assert_eq!(Frame::close(None, "").unwrap().payload(), []);
        assert_eq!(
            complete_data_event(Opcode::Ping, Vec::new()),
            Err(WebSocketError::InvalidFragmentation)
        );
    }

    #[test]
    fn fragmentation_reassembles_split_utf8_with_interleaved_ping() {
        let source = "héllo".as_bytes();
        let fragments = fragment_message(Opcode::Text, source, 2).expect("fragment text");
        assert_eq!(fragments.len(), 3);
        assert_eq!(fragments[0].opcode(), Opcode::Text);
        assert!(!fragments[0].is_final());
        assert_eq!(fragments[2].opcode(), Opcode::Continuation);
        assert!(fragments[2].is_final());

        let mut assembler = MessageAssembler::new(100);
        assert_eq!(assembler.push(fragments[0].clone()).unwrap(), None);
        assert!(assembler.is_fragmented());
        let ping = assembler
            .push(Frame::ping(b"still here".to_vec()).unwrap())
            .unwrap()
            .expect("ping event");
        assert_eq!(ping, MessageEvent::Ping(b"still here".to_vec()));
        let pong = control_reply(&ping).expect("automatic pong");
        assert_eq!(pong.opcode(), Opcode::Pong);
        assert_eq!(pong.payload(), b"still here");
        assert!(assembler.push(fragments[1].clone()).unwrap().is_none());
        assert_eq!(
            assembler.push(fragments[2].clone()).unwrap(),
            Some(MessageEvent::Text("héllo".to_string()))
        );
        assert!(!assembler.is_fragmented());
    }

    #[test]
    fn one_mebibyte_binary_message_fragments_and_reassembles() {
        let payload = (0..1024 * 1024)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let fragments = fragment_message(Opcode::Binary, &payload, 64 * 1024).unwrap();
        assert_eq!(fragments.len(), 16);
        let mut assembler = MessageAssembler::new(payload.len());
        let mut event = None;
        for fragment in fragments {
            event = assembler.push(fragment).unwrap().or(event);
        }
        assert_eq!(event, Some(MessageEvent::Binary(payload)));
    }

    #[test]
    fn assembler_emits_binary_and_pong_events() {
        let mut assembler = MessageAssembler::new(10);
        assert_eq!(
            assembler.push(Frame::binary([1, 2, 3])).unwrap(),
            Some(MessageEvent::Binary(vec![1, 2, 3]))
        );
        assert_eq!(
            assembler.push(Frame::pong([4, 5]).unwrap()).unwrap(),
            Some(MessageEvent::Pong(vec![4, 5]))
        );
        assert!(control_reply(&MessageEvent::Binary(vec![])).is_none());
        assert!(control_reply(&MessageEvent::Pong(vec![])).is_none());
        assert!(control_reply(&MessageEvent::Text(String::new())).is_none());
    }

    #[test]
    fn assembler_rejects_fragmentation_utf8_and_size_failures() {
        let mut assembler = MessageAssembler::new(3);
        assert_eq!(
            assembler.push(Frame::new(true, Opcode::Continuation, []).unwrap()),
            Err(WebSocketError::InvalidFragmentation)
        );
        assert_eq!(
            assembler.push(Frame::new(false, Opcode::Text, b"ab".to_vec()).unwrap()),
            Ok(None)
        );
        assert_eq!(
            assembler.push(Frame::binary([1])),
            Err(WebSocketError::InvalidFragmentation)
        );

        let mut oversized = MessageAssembler::new(3);
        oversized
            .push(Frame::new(false, Opcode::Binary, [1, 2]).unwrap())
            .unwrap();
        assert_eq!(
            oversized.push(Frame::new(true, Opcode::Continuation, [3, 4]).unwrap()),
            Err(WebSocketError::MessageTooLarge)
        );
        let mut complete_oversized = MessageAssembler::new(1);
        assert_eq!(
            complete_oversized.push(Frame::binary([1, 2])),
            Err(WebSocketError::MessageTooLarge)
        );
        let mut invalid_text = MessageAssembler::new(10);
        assert_eq!(
            invalid_text.push(Frame::new(true, Opcode::Text, [0xff]).unwrap()),
            Err(WebSocketError::InvalidUtf8)
        );
    }

    #[test]
    fn fragment_helper_validates_inputs_and_empty_messages() {
        assert_eq!(
            fragment_message(Opcode::Ping, b"x", 1),
            Err(WebSocketError::InvalidFragmentation)
        );
        assert_eq!(
            fragment_message(Opcode::Binary, b"x", 0),
            Err(WebSocketError::InvalidFragmentation)
        );
        assert_eq!(
            fragment_message(Opcode::Text, &[0xff], 1),
            Err(WebSocketError::InvalidUtf8)
        );
        let empty = fragment_message(Opcode::Binary, b"", 8).unwrap();
        assert_eq!(empty, [Frame::binary([])]);
    }

    #[test]
    fn close_event_exposes_validated_status_and_builds_echo() {
        let mut assembler = MessageAssembler::new(10);
        assembler
            .push(Frame::new(false, Opcode::Binary, [1, 2]).unwrap())
            .unwrap();
        assert!(assembler.is_fragmented());
        let event = assembler
            .push(Frame::close(Some(1000), "done").unwrap())
            .unwrap()
            .expect("close event");
        let MessageEvent::Close(close) = &event else {
            panic!("expected close event");
        };
        assert_eq!(close.code(), Some(1000));
        assert_eq!(close.reason(), "done");
        assert!(assembler.is_closed());
        assert!(!assembler.is_fragmented());
        assert_eq!(
            assembler.push(Frame::text("late")),
            Err(WebSocketError::ClosedSession)
        );
        assert_eq!(
            assembler.push(Frame::pong([]).unwrap()).unwrap(),
            Some(MessageEvent::Pong(Vec::new()))
        );
        let echo = control_reply(&event).expect("close echo");
        assert_eq!(echo, Frame::close(Some(1000), "done").unwrap());

        let empty_event = MessageEvent::Close(decode_close_payload(&[]).unwrap());
        let MessageEvent::Close(empty_close) = &empty_event else {
            unreachable!()
        };
        assert_eq!(empty_close.code(), None);
        assert_eq!(empty_close.reason(), "");
        assert_eq!(
            control_reply(&empty_event).unwrap(),
            Frame::close(None, "").unwrap()
        );
    }

    #[test]
    fn valid_close_code_ranges_are_enforced() {
        for code in [1000, 1014, 3000, 4999] {
            assert!(Frame::close(Some(code), "").is_ok());
        }
        for code in [0, 999, 1004, 1005, 1006, 1015, 2000, 2999, 5000] {
            assert_eq!(
                Frame::close(Some(code), ""),
                Err(WebSocketError::InvalidCloseFrame)
            );
        }
    }

    #[test]
    fn diagnostics_are_stable_and_payload_free() {
        let errors = [
            WebSocketError::IncompleteHandshake,
            WebSocketError::HandshakeTooLarge,
            WebSocketError::InvalidHandshake,
            WebSocketError::HeaderInjection,
            WebSocketError::InvalidBase64,
            WebSocketError::ReservedBits,
            WebSocketError::InvalidOpcode,
            WebSocketError::MaskDirection,
            WebSocketError::NonCanonicalLength,
            WebSocketError::FrameTooLarge,
            WebSocketError::InvalidControlFrame,
            WebSocketError::InvalidCloseFrame,
            WebSocketError::InvalidUtf8,
            WebSocketError::InvalidFragmentation,
            WebSocketError::MessageTooLarge,
            WebSocketError::ClosedSession,
        ];
        for error in errors {
            let rendered = error.to_string();
            assert!(rendered.starts_with("websocket:"));
            assert!(!rendered.contains(RFC_KEY));
            assert!(!rendered.contains("payload"));
        }
    }
}
