//! Bounded allocation-free HTTP/1 response-head decoding.
//!
//! This crate parses strict HTTP/1.0 and HTTP/1.1 response-line and header
//! syntax from caller-borrowed bytes. It retains the exact encoded head and the
//! unconsumed body tail without allocating or classifying body framing.

#![forbid(unsafe_code)]

use core::fmt;

/// Fixed maximum number of headers this package can retain.
pub const MAX_HTTP1_RESPONSE_HEADERS: usize = 64;

/// Largest complete response head accepted by this package.
pub const MAX_HTTP1_RESPONSE_HEAD_BYTES: usize = u16::MAX as usize;

/// Largest individual response-line or field-line accepted by this package.
pub const MAX_HTTP1_RESPONSE_LINE_BYTES: usize = 8 * 1024;

/// HTTP versions admitted by this deliberately narrow response parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1ResponseVersion {
    Http10,
    Http11,
}

/// Caller-owned resource limits for one response head.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Http1ResponseHeadLimits {
    maximum_head_bytes: u16,
    maximum_line_bytes: u16,
    maximum_headers: u8,
}

impl Http1ResponseHeadLimits {
    /// Validate complete-head, individual-line, and retained-header ceilings.
    pub fn new(
        maximum_head_bytes: usize,
        maximum_line_bytes: usize,
        maximum_headers: usize,
    ) -> Result<Self, Http1ResponseHeadError> {
        if maximum_head_bytes == 0
            || maximum_head_bytes > MAX_HTTP1_RESPONSE_HEAD_BYTES
            || maximum_line_bytes == 0
            || maximum_line_bytes > MAX_HTTP1_RESPONSE_LINE_BYTES
            || maximum_line_bytes > maximum_head_bytes
            || maximum_headers > MAX_HTTP1_RESPONSE_HEADERS
        {
            return Err(Http1ResponseHeadError::InvalidLimits);
        }
        Ok(Self {
            maximum_head_bytes: maximum_head_bytes as u16,
            maximum_line_bytes: maximum_line_bytes as u16,
            maximum_headers: maximum_headers as u8,
        })
    }

    #[must_use]
    pub const fn maximum_head_bytes(self) -> usize {
        self.maximum_head_bytes as usize
    }

    #[must_use]
    pub const fn maximum_line_bytes(self) -> usize {
        self.maximum_line_bytes as usize
    }

    #[must_use]
    pub const fn maximum_headers(self) -> usize {
        self.maximum_headers as usize
    }
}

/// One borrowed response header field in original wire order.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Http1ResponseHeader<'input> {
    name: &'input [u8],
    value: &'input [u8],
}

impl<'input> Http1ResponseHeader<'input> {
    #[must_use]
    pub const fn name(self) -> &'input [u8] {
        self.name
    }

    #[must_use]
    pub const fn value(self) -> &'input [u8] {
        self.value
    }

    /// Compare an ASCII caller name without normalizing or allocating.
    #[must_use]
    pub fn name_eq_ignore_ascii_case(self, expected: &[u8]) -> bool {
        self.name.eq_ignore_ascii_case(expected)
    }
}

impl fmt::Debug for Http1ResponseHeader<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Http1ResponseHeader")
            .field("name", &"[REDACTED]")
            .field("value", &"[REDACTED]")
            .field("name_length", &self.name.len())
            .field("value_length", &self.value.len())
            .finish()
    }
}

const EMPTY_HEADER: Http1ResponseHeader<'static> = Http1ResponseHeader {
    name: &[],
    value: &[],
};

/// One exact borrowed response head plus its unconsumed stream tail.
pub struct Http1ResponseHead<'input> {
    version: Http1ResponseVersion,
    status_code: u16,
    reason_phrase: &'input [u8],
    headers: [Http1ResponseHeader<'input>; MAX_HTTP1_RESPONSE_HEADERS],
    header_count: u8,
    encoded_head: &'input [u8],
    body_tail: &'input [u8],
}

impl<'input> Http1ResponseHead<'input> {
    #[must_use]
    pub const fn version(&self) -> Http1ResponseVersion {
        self.version
    }

    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_code
    }

    #[must_use]
    pub const fn reason_phrase(&self) -> &'input [u8] {
        self.reason_phrase
    }

    #[must_use]
    pub fn headers(&self) -> &[Http1ResponseHeader<'input>] {
        &self.headers[..usize::from(self.header_count)]
    }

    #[must_use]
    pub const fn encoded_head(&self) -> &'input [u8] {
        self.encoded_head
    }

    #[must_use]
    pub const fn body_tail(&self) -> &'input [u8] {
        self.body_tail
    }
}

impl fmt::Debug for Http1ResponseHead<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Http1ResponseHead")
            .field("version", &self.version)
            .field("status_code", &self.status_code)
            .field("reason_phrase", &"[REDACTED]")
            .field("header_count", &self.header_count)
            .field("encoded_head", &"[REDACTED]")
            .field("body_tail", &"[REDACTED]")
            .finish()
    }
}

/// Stable payload-free failures for limits and response-head syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1ResponseHeadError {
    InvalidLimits,
    IncompleteHead,
    HeadTooLong,
    InvalidLineEnding,
    LineTooLong,
    InvalidVersion,
    InvalidStatusLine,
    InvalidStatusCode,
    InvalidReasonPhrase,
    TooManyHeaders,
    ObsoleteLineFolding,
    MissingHeaderColon,
    EmptyHeaderName,
    InvalidHeaderName,
    InvalidHeaderValue,
}

impl fmt::Display for Http1ResponseHeadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid HTTP/1 response-head limits",
            Self::IncompleteHead => "incomplete HTTP/1 response head",
            Self::HeadTooLong => "HTTP/1 response head exceeds its byte limit",
            Self::InvalidLineEnding => "invalid HTTP/1 response line ending",
            Self::LineTooLong => "HTTP/1 response line exceeds its byte limit",
            Self::InvalidVersion => "unsupported HTTP response version",
            Self::InvalidStatusLine => "invalid HTTP response status line",
            Self::InvalidStatusCode => "invalid HTTP response status code",
            Self::InvalidReasonPhrase => "invalid HTTP response reason phrase",
            Self::TooManyHeaders => "HTTP response exceeds its header-count limit",
            Self::ObsoleteLineFolding => "obsolete HTTP response line folding is forbidden",
            Self::MissingHeaderColon => "HTTP response header is missing a colon",
            Self::EmptyHeaderName => "HTTP response header name is empty",
            Self::InvalidHeaderName => "invalid HTTP response header name",
            Self::InvalidHeaderValue => "invalid HTTP response header value",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1ResponseHeadError {}

/// Decode one response head and retain the unconsumed body bytes.
pub fn decode_http1_response_head(
    input: &[u8],
    limits: Http1ResponseHeadLimits,
) -> Result<Http1ResponseHead<'_>, Http1ResponseHeadError> {
    let (status_line, mut offset) = next_line(input, 0, limits)?;
    let (version, status_code, reason_phrase) = parse_status_line(status_line)?;
    let mut headers = [EMPTY_HEADER; MAX_HTTP1_RESPONSE_HEADERS];
    let mut header_count = 0usize;

    loop {
        let (line, next_offset) = next_line(input, offset, limits)?;
        offset = next_offset;
        if line.is_empty() {
            break;
        }
        if header_count >= limits.maximum_headers() {
            return Err(Http1ResponseHeadError::TooManyHeaders);
        }
        headers[header_count] = parse_header(line)?;
        header_count += 1;
    }

    Ok(Http1ResponseHead {
        version,
        status_code,
        reason_phrase,
        headers,
        header_count: header_count as u8,
        encoded_head: &input[..offset],
        body_tail: &input[offset..],
    })
}

fn next_line(
    input: &[u8],
    start: usize,
    limits: Http1ResponseHeadLimits,
) -> Result<(&[u8], usize), Http1ResponseHeadError> {
    let mut index = start;
    while index < input.len() && index < limits.maximum_head_bytes() {
        match input[index] {
            b'\r' => {
                if index + 1 >= input.len() {
                    return if input.len() >= limits.maximum_head_bytes() {
                        Err(Http1ResponseHeadError::HeadTooLong)
                    } else {
                        Err(Http1ResponseHeadError::IncompleteHead)
                    };
                }
                if input[index + 1] != b'\n' {
                    return Err(Http1ResponseHeadError::InvalidLineEnding);
                }
                let next = index + 2;
                if next > limits.maximum_head_bytes() {
                    return Err(Http1ResponseHeadError::HeadTooLong);
                }
                if index - start > limits.maximum_line_bytes() {
                    return Err(Http1ResponseHeadError::LineTooLong);
                }
                return Ok((&input[start..index], next));
            }
            b'\n' => return Err(Http1ResponseHeadError::InvalidLineEnding),
            _ => index += 1,
        }
    }

    if index >= limits.maximum_head_bytes() {
        Err(Http1ResponseHeadError::HeadTooLong)
    } else {
        Err(Http1ResponseHeadError::IncompleteHead)
    }
}

fn parse_status_line(
    line: &[u8],
) -> Result<(Http1ResponseVersion, u16, &[u8]), Http1ResponseHeadError> {
    if line.len() < 13 || line[8] != b' ' || line[12] != b' ' {
        return Err(Http1ResponseHeadError::InvalidStatusLine);
    }
    let version = match &line[..8] {
        b"HTTP/1.0" => Http1ResponseVersion::Http10,
        b"HTTP/1.1" => Http1ResponseVersion::Http11,
        _ => return Err(Http1ResponseHeadError::InvalidVersion),
    };
    let digits = &line[9..12];
    if !digits.iter().all(u8::is_ascii_digit) {
        return Err(Http1ResponseHeadError::InvalidStatusCode);
    }
    let status_code = u16::from(digits[0] - b'0') * 100
        + u16::from(digits[1] - b'0') * 10
        + u16::from(digits[2] - b'0');
    if !(100..=599).contains(&status_code) {
        return Err(Http1ResponseHeadError::InvalidStatusCode);
    }
    let reason_phrase = &line[13..];
    if !reason_phrase.iter().copied().all(is_field_value_octet) {
        return Err(Http1ResponseHeadError::InvalidReasonPhrase);
    }
    Ok((version, status_code, reason_phrase))
}

fn parse_header(line: &[u8]) -> Result<Http1ResponseHeader<'_>, Http1ResponseHeadError> {
    if matches!(line.first(), Some(b' ' | b'\t')) {
        return Err(Http1ResponseHeadError::ObsoleteLineFolding);
    }
    let Some(colon) = line.iter().position(|byte| *byte == b':') else {
        return Err(Http1ResponseHeadError::MissingHeaderColon);
    };
    let name = &line[..colon];
    if name.is_empty() {
        return Err(Http1ResponseHeadError::EmptyHeaderName);
    }
    if !name.iter().copied().all(is_token_octet) {
        return Err(Http1ResponseHeadError::InvalidHeaderName);
    }
    let raw_value = &line[colon + 1..];
    if !raw_value.iter().copied().all(is_field_value_octet) {
        return Err(Http1ResponseHeadError::InvalidHeaderValue);
    }
    Ok(Http1ResponseHeader {
        name,
        value: trim_optional_whitespace(raw_value),
    })
}

fn is_token_octet(byte: u8) -> bool {
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

fn is_field_value_octet(byte: u8) -> bool {
    byte == b'\t' || (b' '..=b'~').contains(&byte) || byte >= 0x80
}

fn trim_optional_whitespace(mut value: &[u8]) -> &[u8] {
    while matches!(value.first(), Some(b' ' | b'\t')) {
        value = &value[1..];
    }
    while matches!(value.last(), Some(b' ' | b'\t')) {
        value = &value[..value.len() - 1];
    }
    value
}
