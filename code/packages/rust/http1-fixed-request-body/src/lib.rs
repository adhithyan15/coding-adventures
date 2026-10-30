//! Bounded allocation-free HTTP/1 fixed request-body length binding.
//!
//! This crate retains one caller-borrowed, potentially sensitive request body
//! under an explicit ceiling and encodes its exact byte length as canonical
//! shortest decimal into caller-owned storage.

#![forbid(unsafe_code)]

use core::fmt;

/// Largest request body admitted by this deliberately narrow boundary.
pub const MAX_HTTP1_FIXED_REQUEST_BODY_BYTES: usize = 1024 * 1024;

/// Largest decimal length value emitted for the package body ceiling.
pub const MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES: usize = 7;

/// Caller-owned resource limit for one fixed request body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Http1FixedRequestBodyLimits {
    maximum_body_bytes: u32,
}

impl Http1FixedRequestBodyLimits {
    /// Validate the maximum body ceiling. Zero admits only an empty body.
    pub fn new(maximum_body_bytes: usize) -> Result<Self, Http1FixedRequestBodyError> {
        if maximum_body_bytes > MAX_HTTP1_FIXED_REQUEST_BODY_BYTES {
            return Err(Http1FixedRequestBodyError::InvalidLimits);
        }
        Ok(Self {
            maximum_body_bytes: maximum_body_bytes as u32,
        })
    }

    #[must_use]
    pub const fn maximum_body_bytes(self) -> usize {
        self.maximum_body_bytes as usize
    }
}

/// Non-copyable evidence binding one borrowed body to its exact length.
pub struct Http1FixedRequestBody<'input> {
    body: &'input [u8],
    content_length_value_length: u8,
}

impl<'input> Http1FixedRequestBody<'input> {
    /// Return the exact body whose length was validated.
    #[must_use]
    pub const fn body(&self) -> &'input [u8] {
        self.body
    }

    /// Return the exact request body length in bytes.
    #[must_use]
    pub const fn content_length(&self) -> usize {
        self.body.len()
    }

    /// Return the canonical decimal value's encoded byte length.
    #[must_use]
    pub const fn content_length_value_length(&self) -> usize {
        self.content_length_value_length as usize
    }
}

impl fmt::Debug for Http1FixedRequestBody<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Http1FixedRequestBody")
            .field("body", &"[REDACTED]")
            .field("body_length", &self.body.len())
            .field(
                "content_length_value_length",
                &self.content_length_value_length,
            )
            .finish()
    }
}

/// Stable payload-free fixed request-body failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1FixedRequestBodyError {
    InvalidLimits,
    BodyTooLong,
    OutputTooSmall,
}

impl fmt::Display for Http1FixedRequestBodyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid HTTP/1 fixed request-body limits",
            Self::BodyTooLong => "HTTP/1 fixed request body exceeds its byte limit",
            Self::OutputTooSmall => "HTTP/1 fixed request content-length output is too small",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1FixedRequestBodyError {}

/// Bind one borrowed body to its exact byte length under caller limits.
pub fn validate_http1_fixed_request_body<'input>(
    body: &'input [u8],
    limits: Http1FixedRequestBodyLimits,
) -> Result<Http1FixedRequestBody<'input>, Http1FixedRequestBodyError> {
    if body.len() > limits.maximum_body_bytes() {
        return Err(Http1FixedRequestBodyError::BodyTooLong);
    }
    Ok(Http1FixedRequestBody {
        body,
        content_length_value_length: decimal_length(body.len()) as u8,
    })
}

/// Encode the exact body length as canonical shortest decimal.
///
/// The output remains unchanged when its capacity is insufficient.
pub fn encode_http1_fixed_request_content_length<'output>(
    body: &Http1FixedRequestBody<'_>,
    output: &'output mut [u8],
) -> Result<&'output mut [u8], Http1FixedRequestBodyError> {
    let encoded_length = body.content_length_value_length();
    if output.len() < encoded_length {
        return Err(Http1FixedRequestBodyError::OutputTooSmall);
    }
    write_decimal(body.content_length(), &mut output[..encoded_length]);
    Ok(&mut output[..encoded_length])
}

const fn decimal_length(value: usize) -> usize {
    match value {
        0..=9 => 1,
        10..=99 => 2,
        100..=999 => 3,
        1_000..=9_999 => 4,
        10_000..=99_999 => 5,
        100_000..=999_999 => 6,
        _ => 7,
    }
}

fn write_decimal(mut value: usize, output: &mut [u8]) {
    for digit in output.iter_mut().rev() {
        *digit = b'0' + (value % 10) as u8;
        value /= 10;
    }
    debug_assert_eq!(value, 0);
}
