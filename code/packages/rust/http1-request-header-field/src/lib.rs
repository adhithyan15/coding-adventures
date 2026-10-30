//! Bounded allocation-free HTTP/1 request-header field encoding.
//!
//! This crate validates one caller-borrowed token field name and a deliberately
//! narrow ASCII field value under explicit limits, then writes one exact field
//! line into caller-owned storage. Values are treated as potentially sensitive.

#![forbid(unsafe_code)]

use core::fmt;

/// Largest field name accepted by this deliberately narrow encoder.
pub const MAX_HTTP1_REQUEST_HEADER_NAME_BYTES: usize = 256;

/// Largest field value accepted by this deliberately narrow encoder.
pub const MAX_HTTP1_REQUEST_HEADER_VALUE_BYTES: usize = 8 * 1024;

/// Largest complete field line, including separator and CRLF.
pub const MAX_HTTP1_REQUEST_HEADER_LINE_BYTES: usize = 8 * 1024;

/// Caller-owned resource limits for one request-header field line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Http1RequestHeaderFieldLimits {
    maximum_name_bytes: u16,
    maximum_value_bytes: u16,
    maximum_line_bytes: u16,
}

impl Http1RequestHeaderFieldLimits {
    /// Validate field-name, field-value, and complete-line ceilings.
    pub fn new(
        maximum_name_bytes: usize,
        maximum_value_bytes: usize,
        maximum_line_bytes: usize,
    ) -> Result<Self, Http1RequestHeaderFieldError> {
        if maximum_name_bytes == 0
            || maximum_name_bytes > MAX_HTTP1_REQUEST_HEADER_NAME_BYTES
            || maximum_value_bytes > MAX_HTTP1_REQUEST_HEADER_VALUE_BYTES
            || maximum_line_bytes == 0
            || maximum_line_bytes > MAX_HTTP1_REQUEST_HEADER_LINE_BYTES
        {
            return Err(Http1RequestHeaderFieldError::InvalidLimits);
        }
        Ok(Self {
            maximum_name_bytes: maximum_name_bytes as u16,
            maximum_value_bytes: maximum_value_bytes as u16,
            maximum_line_bytes: maximum_line_bytes as u16,
        })
    }

    #[must_use]
    pub const fn maximum_name_bytes(self) -> usize {
        self.maximum_name_bytes as usize
    }

    #[must_use]
    pub const fn maximum_value_bytes(self) -> usize {
        self.maximum_value_bytes as usize
    }

    #[must_use]
    pub const fn maximum_line_bytes(self) -> usize {
        self.maximum_line_bytes as usize
    }
}

/// One validated borrowed request-header field.
pub struct Http1RequestHeaderField<'input> {
    name: &'input [u8],
    value: &'input [u8],
    encoded_length: u16,
}

impl<'input> Http1RequestHeaderField<'input> {
    #[must_use]
    pub const fn name(&self) -> &'input [u8] {
        self.name
    }

    #[must_use]
    pub const fn value(&self) -> &'input [u8] {
        self.value
    }

    #[must_use]
    pub const fn encoded_length(&self) -> usize {
        self.encoded_length as usize
    }
}

impl fmt::Debug for Http1RequestHeaderField<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Http1RequestHeaderField")
            .field("name", &"[REDACTED]")
            .field("name_length", &self.name.len())
            .field("value", &"[REDACTED]")
            .field("value_length", &self.value.len())
            .field("encoded_length", &self.encoded_length)
            .finish()
    }
}

/// Stable payload-free request-header field failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1RequestHeaderFieldError {
    InvalidLimits,
    EmptyName,
    NameTooLong,
    InvalidName,
    ValueTooLong,
    InvalidValue,
    ValueHasOuterWhitespace,
    LineTooLong,
    OutputTooSmall,
}

impl fmt::Display for Http1RequestHeaderFieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid HTTP/1 request-header field limits",
            Self::EmptyName => "HTTP/1 request-header field name is empty",
            Self::NameTooLong => "HTTP/1 request-header field name exceeds its byte limit",
            Self::InvalidName => "invalid HTTP/1 request-header field name",
            Self::ValueTooLong => "HTTP/1 request-header field value exceeds its byte limit",
            Self::InvalidValue => "invalid HTTP/1 request-header field value",
            Self::ValueHasOuterWhitespace => {
                "HTTP/1 request-header field value has outer whitespace"
            }
            Self::LineTooLong => "HTTP/1 request-header field line exceeds its byte limit",
            Self::OutputTooSmall => "HTTP/1 request-header field output is too small",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1RequestHeaderFieldError {}

/// Validate one borrowed request-header field name and value.
pub fn validate_http1_request_header_field<'input>(
    name: &'input [u8],
    value: &'input [u8],
    limits: Http1RequestHeaderFieldLimits,
) -> Result<Http1RequestHeaderField<'input>, Http1RequestHeaderFieldError> {
    if name.is_empty() {
        return Err(Http1RequestHeaderFieldError::EmptyName);
    }
    if name.len() > limits.maximum_name_bytes() {
        return Err(Http1RequestHeaderFieldError::NameTooLong);
    }
    if !name.iter().copied().all(is_token_byte) {
        return Err(Http1RequestHeaderFieldError::InvalidName);
    }
    if value.len() > limits.maximum_value_bytes() {
        return Err(Http1RequestHeaderFieldError::ValueTooLong);
    }
    if !value.iter().copied().all(is_field_value_byte) {
        return Err(Http1RequestHeaderFieldError::InvalidValue);
    }
    if matches!(value.first(), Some(b' ' | b'\t')) || matches!(value.last(), Some(b' ' | b'\t')) {
        return Err(Http1RequestHeaderFieldError::ValueHasOuterWhitespace);
    }

    let encoded_length = name
        .len()
        .checked_add(value.len())
        .and_then(|length| length.checked_add(4))
        .ok_or(Http1RequestHeaderFieldError::LineTooLong)?;
    if encoded_length > limits.maximum_line_bytes() {
        return Err(Http1RequestHeaderFieldError::LineTooLong);
    }

    Ok(Http1RequestHeaderField {
        name,
        value,
        encoded_length: encoded_length as u16,
    })
}

/// Encode one previously validated request-header field into caller storage.
///
/// The output remains unchanged when its capacity is insufficient.
pub fn encode_http1_request_header_field<'output>(
    field: &Http1RequestHeaderField<'_>,
    output: &'output mut [u8],
) -> Result<&'output mut [u8], Http1RequestHeaderFieldError> {
    let encoded_length = field.encoded_length();
    if output.len() < encoded_length {
        return Err(Http1RequestHeaderFieldError::OutputTooSmall);
    }

    let mut offset = 0usize;
    write(&mut output[..encoded_length], &mut offset, field.name);
    write(&mut output[..encoded_length], &mut offset, b": ");
    write(&mut output[..encoded_length], &mut offset, field.value);
    write(&mut output[..encoded_length], &mut offset, b"\r\n");
    debug_assert_eq!(offset, encoded_length);
    Ok(&mut output[..encoded_length])
}

fn write(output: &mut [u8], offset: &mut usize, bytes: &[u8]) {
    let end = *offset + bytes.len();
    output[*offset..end].copy_from_slice(bytes);
    *offset = end;
}

const fn is_token_byte(byte: u8) -> bool {
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

const fn is_field_value_byte(byte: u8) -> bool {
    byte == b'\t' || (byte >= b' ' && byte <= b'~')
}
