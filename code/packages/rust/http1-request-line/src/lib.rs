//! Bounded allocation-free HTTP/1 request-line validation and encoding.
//!
//! This crate accepts caller-borrowed method and origin-form target bytes,
//! validates them under explicit caller limits, and writes one exact HTTP/1.0
//! or HTTP/1.1 request line into caller-owned storage.

#![forbid(unsafe_code)]

use core::fmt;

/// Largest method accepted by this deliberately narrow encoder.
pub const MAX_HTTP1_REQUEST_METHOD_BYTES: usize = 32;

/// Largest origin-form target accepted by this deliberately narrow encoder.
pub const MAX_HTTP1_REQUEST_TARGET_BYTES: usize = 8 * 1024;

/// Largest complete request line, including spaces, version, and CRLF.
pub const MAX_HTTP1_REQUEST_LINE_BYTES: usize = 8 * 1024;

/// HTTP versions admitted by this request-line encoder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1RequestVersion {
    Http10,
    Http11,
}

impl Http1RequestVersion {
    const fn encoded(self) -> &'static [u8; 8] {
        match self {
            Self::Http10 => b"HTTP/1.0",
            Self::Http11 => b"HTTP/1.1",
        }
    }
}

/// Caller-owned resource limits for one request line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Http1RequestLineLimits {
    maximum_method_bytes: u8,
    maximum_target_bytes: u16,
    maximum_line_bytes: u16,
}

impl Http1RequestLineLimits {
    /// Validate method, target, and complete-line ceilings.
    pub fn new(
        maximum_method_bytes: usize,
        maximum_target_bytes: usize,
        maximum_line_bytes: usize,
    ) -> Result<Self, Http1RequestLineError> {
        if maximum_method_bytes == 0
            || maximum_method_bytes > MAX_HTTP1_REQUEST_METHOD_BYTES
            || maximum_target_bytes == 0
            || maximum_target_bytes > MAX_HTTP1_REQUEST_TARGET_BYTES
            || maximum_line_bytes == 0
            || maximum_line_bytes > MAX_HTTP1_REQUEST_LINE_BYTES
        {
            return Err(Http1RequestLineError::InvalidLimits);
        }
        Ok(Self {
            maximum_method_bytes: maximum_method_bytes as u8,
            maximum_target_bytes: maximum_target_bytes as u16,
            maximum_line_bytes: maximum_line_bytes as u16,
        })
    }

    #[must_use]
    pub const fn maximum_method_bytes(self) -> usize {
        self.maximum_method_bytes as usize
    }

    #[must_use]
    pub const fn maximum_target_bytes(self) -> usize {
        self.maximum_target_bytes as usize
    }

    #[must_use]
    pub const fn maximum_line_bytes(self) -> usize {
        self.maximum_line_bytes as usize
    }
}

/// One validated borrowed request line.
pub struct Http1RequestLine<'input> {
    method: &'input [u8],
    target: &'input [u8],
    version: Http1RequestVersion,
    encoded_length: u16,
}

impl<'input> Http1RequestLine<'input> {
    #[must_use]
    pub const fn method(&self) -> &'input [u8] {
        self.method
    }

    #[must_use]
    pub const fn target(&self) -> &'input [u8] {
        self.target
    }

    #[must_use]
    pub const fn version(&self) -> Http1RequestVersion {
        self.version
    }

    #[must_use]
    pub const fn encoded_length(&self) -> usize {
        self.encoded_length as usize
    }
}

impl fmt::Debug for Http1RequestLine<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Http1RequestLine")
            .field("method", &"[REDACTED]")
            .field("method_length", &self.method.len())
            .field("target", &"[REDACTED]")
            .field("target_length", &self.target.len())
            .field("version", &self.version)
            .field("encoded_length", &self.encoded_length)
            .finish()
    }
}

/// Stable payload-free request-line failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1RequestLineError {
    InvalidLimits,
    EmptyMethod,
    MethodTooLong,
    InvalidMethod,
    EmptyTarget,
    TargetTooLong,
    TargetNotOriginForm,
    InvalidTarget,
    LineTooLong,
    OutputTooSmall,
}

impl fmt::Display for Http1RequestLineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid HTTP/1 request-line limits",
            Self::EmptyMethod => "HTTP/1 request method is empty",
            Self::MethodTooLong => "HTTP/1 request method exceeds its byte limit",
            Self::InvalidMethod => "invalid HTTP/1 request method",
            Self::EmptyTarget => "HTTP/1 request target is empty",
            Self::TargetTooLong => "HTTP/1 request target exceeds its byte limit",
            Self::TargetNotOriginForm => "HTTP/1 request target is not origin-form",
            Self::InvalidTarget => "invalid HTTP/1 request target",
            Self::LineTooLong => "HTTP/1 request line exceeds its byte limit",
            Self::OutputTooSmall => "HTTP/1 request-line output is too small",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1RequestLineError {}

/// Validate one borrowed method, origin-form target, and version.
pub fn validate_http1_request_line<'input>(
    method: &'input [u8],
    target: &'input [u8],
    version: Http1RequestVersion,
    limits: Http1RequestLineLimits,
) -> Result<Http1RequestLine<'input>, Http1RequestLineError> {
    if method.is_empty() {
        return Err(Http1RequestLineError::EmptyMethod);
    }
    if method.len() > limits.maximum_method_bytes() {
        return Err(Http1RequestLineError::MethodTooLong);
    }
    if !method.iter().copied().all(is_token_byte) {
        return Err(Http1RequestLineError::InvalidMethod);
    }
    if target.is_empty() {
        return Err(Http1RequestLineError::EmptyTarget);
    }
    if target.len() > limits.maximum_target_bytes() {
        return Err(Http1RequestLineError::TargetTooLong);
    }
    if target[0] != b'/' {
        return Err(Http1RequestLineError::TargetNotOriginForm);
    }
    if !is_origin_form_target(target) {
        return Err(Http1RequestLineError::InvalidTarget);
    }

    let encoded_length = method
        .len()
        .checked_add(target.len())
        .and_then(|length| length.checked_add(12))
        .ok_or(Http1RequestLineError::LineTooLong)?;
    if encoded_length > limits.maximum_line_bytes() {
        return Err(Http1RequestLineError::LineTooLong);
    }

    Ok(Http1RequestLine {
        method,
        target,
        version,
        encoded_length: encoded_length as u16,
    })
}

/// Encode one previously validated request line into caller-owned storage.
///
/// The output remains unchanged when its capacity is insufficient.
pub fn encode_http1_request_line<'output>(
    request_line: &Http1RequestLine<'_>,
    output: &'output mut [u8],
) -> Result<&'output mut [u8], Http1RequestLineError> {
    let encoded_length = request_line.encoded_length();
    if output.len() < encoded_length {
        return Err(Http1RequestLineError::OutputTooSmall);
    }

    let mut offset = 0usize;
    write(
        &mut output[..encoded_length],
        &mut offset,
        request_line.method,
    );
    write(&mut output[..encoded_length], &mut offset, b" ");
    write(
        &mut output[..encoded_length],
        &mut offset,
        request_line.target,
    );
    write(&mut output[..encoded_length], &mut offset, b" ");
    write(
        &mut output[..encoded_length],
        &mut offset,
        request_line.version.encoded(),
    );
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

fn is_origin_form_target(target: &[u8]) -> bool {
    let mut index = 0usize;
    let mut in_query = false;
    while index < target.len() {
        let byte = target[index];
        if byte == b'%' {
            if index + 2 >= target.len()
                || !target[index + 1].is_ascii_hexdigit()
                || !target[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
            continue;
        }
        if !in_query && byte == b'?' {
            in_query = true;
        } else if !(is_pchar_byte(byte) || byte == b'/' || (in_query && byte == b'?')) {
            return false;
        }
        index += 1;
    }
    true
}

const fn is_pchar_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'-' | b'.'
                | b'_'
                | b'~'
                | b'!'
                | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'='
                | b':'
                | b'@'
        )
}
