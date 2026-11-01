//! Strict allocation-free HTTP/1 response body-framing policy.
//!
//! This crate consumes borrowed fields from a strict response-head decoder. It
//! does not authorize the existing permissive `http1` or `http1-client`
//! packages. It rejects transfer coding, framing ambiguity, and noncanonical
//! lengths before returning a bounded body-consumption plan.

#![forbid(unsafe_code)]

use core::fmt;

/// One borrowed response header produced by a strict response-head decoder.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Http1ResponseFramingHeader<'input> {
    name: &'input [u8],
    value: &'input [u8],
}

impl<'input> Http1ResponseFramingHeader<'input> {
    /// Retain one field from a strict HTTP/1 syntax decoder.
    ///
    /// Local field validation does not replace strict CRLF response-head
    /// parsing or authorize metadata from the existing permissive `http1`
    /// parser.
    #[must_use]
    pub const fn new(name: &'input [u8], value: &'input [u8]) -> Self {
        Self { name, value }
    }

    #[must_use]
    pub const fn name(self) -> &'input [u8] {
        self.name
    }

    #[must_use]
    pub const fn value(self) -> &'input [u8] {
        self.value
    }
}

impl fmt::Debug for Http1ResponseFramingHeader<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Http1ResponseFramingHeader")
            .field("name", &"[REDACTED]")
            .field("value", &"[REDACTED]")
            .field("name_length", &self.name.len())
            .field("value_length", &self.value.len())
            .finish()
    }
}

/// Semantics derived from the exact emitted request that caused this response.
///
/// A caller must not derive this value from response bytes or other untrusted
/// provider input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1ResponseRequestSemantics {
    Normal,
    Head,
    Connect,
}

/// Caller policy for consuming a response without an explicit length to EOF.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseDelimitedPolicy {
    Reject,
    Allow,
}

/// Caller-owned limits and close-delimited policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Http1ResponseBodyFramingPolicy {
    maximum_body_bytes: usize,
    close_delimited: CloseDelimitedPolicy,
}

impl Http1ResponseBodyFramingPolicy {
    #[must_use]
    pub const fn new(maximum_body_bytes: usize, close_delimited: CloseDelimitedPolicy) -> Self {
        Self {
            maximum_body_bytes,
            close_delimited,
        }
    }

    #[must_use]
    pub const fn maximum_body_bytes(self) -> usize {
        self.maximum_body_bytes
    }

    #[must_use]
    pub const fn close_delimited(self) -> CloseDelimitedPolicy {
        self.close_delimited
    }
}

/// Exact body-consumption plan produced by the framing policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1ResponseBodyFraming {
    NoBody,
    ContentLength(usize),
    UntilClose { maximum_body_bytes: usize },
}

/// Stable payload-free framing failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1ResponseBodyFramingError {
    InvalidStatusCode,
    InvalidHeaderName,
    InvalidHeaderValue,
    TransferEncodingForbidden,
    DuplicateContentLength,
    InvalidContentLength,
    NonCanonicalContentLength,
    ContentLengthTooLarge,
    ContentLengthForbidden,
    CloseDelimitedForbidden,
    SuccessfulConnectUnsupported,
}

impl fmt::Display for Http1ResponseBodyFramingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidStatusCode => "invalid HTTP response status code",
            Self::InvalidHeaderName => "invalid HTTP response header name",
            Self::InvalidHeaderValue => "invalid HTTP response header value",
            Self::TransferEncodingForbidden => "HTTP transfer encoding is not supported",
            Self::DuplicateContentLength => "duplicate HTTP Content-Length is forbidden",
            Self::InvalidContentLength => "invalid HTTP Content-Length",
            Self::NonCanonicalContentLength => "noncanonical HTTP Content-Length",
            Self::ContentLengthTooLarge => "HTTP Content-Length exceeds the body limit",
            Self::ContentLengthForbidden => "HTTP Content-Length is forbidden for this response",
            Self::CloseDelimitedForbidden => "close-delimited HTTP response body is forbidden",
            Self::SuccessfulConnectUnsupported => {
                "successful HTTP CONNECT tunnel framing is unsupported"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1ResponseBodyFramingError {}

/// Classify one final or informational response into a bounded body plan.
///
/// `request` must be bound to the exact emitted request. `policy` is trusted
/// caller configuration, not a choice derived from response bytes.
pub fn classify_http1_response_body(
    request: Http1ResponseRequestSemantics,
    status_code: u16,
    headers: &[Http1ResponseFramingHeader<'_>],
    policy: Http1ResponseBodyFramingPolicy,
) -> Result<Http1ResponseBodyFraming, Http1ResponseBodyFramingError> {
    if !(100..=599).contains(&status_code) {
        return Err(Http1ResponseBodyFramingError::InvalidStatusCode);
    }

    let mut content_length = None;
    for header in headers {
        validate_header(*header)?;
        if header.name.eq_ignore_ascii_case(b"Transfer-Encoding") {
            return Err(Http1ResponseBodyFramingError::TransferEncodingForbidden);
        }
        if header.name.eq_ignore_ascii_case(b"Content-Length") {
            if content_length.is_some() {
                return Err(Http1ResponseBodyFramingError::DuplicateContentLength);
            }
            content_length = Some(parse_content_length(header.value)?);
        }
    }

    if (100..200).contains(&status_code) || status_code == 204 {
        if content_length.is_some() {
            return Err(Http1ResponseBodyFramingError::ContentLengthForbidden);
        }
        return Ok(Http1ResponseBodyFraming::NoBody);
    }
    if request == Http1ResponseRequestSemantics::Head || status_code == 304 {
        return Ok(Http1ResponseBodyFraming::NoBody);
    }
    if request == Http1ResponseRequestSemantics::Connect && (200..300).contains(&status_code) {
        return Err(Http1ResponseBodyFramingError::SuccessfulConnectUnsupported);
    }

    match content_length {
        Some(length) if length > policy.maximum_body_bytes => {
            Err(Http1ResponseBodyFramingError::ContentLengthTooLarge)
        }
        Some(length) => Ok(Http1ResponseBodyFraming::ContentLength(length)),
        None if policy.close_delimited == CloseDelimitedPolicy::Allow => {
            Ok(Http1ResponseBodyFraming::UntilClose {
                maximum_body_bytes: policy.maximum_body_bytes,
            })
        }
        None => Err(Http1ResponseBodyFramingError::CloseDelimitedForbidden),
    }
}

fn validate_header(
    header: Http1ResponseFramingHeader<'_>,
) -> Result<(), Http1ResponseBodyFramingError> {
    if header.name.is_empty() || !header.name.iter().copied().all(is_token_octet) {
        return Err(Http1ResponseBodyFramingError::InvalidHeaderName);
    }
    if !header.value.iter().copied().all(is_field_value_octet) {
        return Err(Http1ResponseBodyFramingError::InvalidHeaderValue);
    }
    Ok(())
}

fn parse_content_length(value: &[u8]) -> Result<usize, Http1ResponseBodyFramingError> {
    let value = trim_optional_whitespace(value);
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return Err(Http1ResponseBodyFramingError::InvalidContentLength);
    }
    if value.len() > 1 && value[0] == b'0' {
        return Err(Http1ResponseBodyFramingError::NonCanonicalContentLength);
    }
    value.iter().try_fold(0usize, |length, byte| {
        length
            .checked_mul(10)
            .and_then(|length| length.checked_add(usize::from(*byte - b'0')))
            .ok_or(Http1ResponseBodyFramingError::InvalidContentLength)
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
