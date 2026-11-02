//! Strict decoding of an empty HTTP/1 trailer section.
//!
//! The zero-size chunk line is caller-owned. This crate accepts only the exact
//! empty trailer-section terminator and rejects every trailer field.

#![forbid(unsafe_code)]

use core::fmt;

/// One validated borrowed empty trailer section and unconsumed input.
pub struct Http1EmptyTrailerSection<'input> {
    encoded_section: &'input [u8],
    remainder: &'input [u8],
}

impl<'input> Http1EmptyTrailerSection<'input> {
    #[must_use]
    pub const fn encoded_section(&self) -> &'input [u8] {
        self.encoded_section
    }

    #[must_use]
    pub const fn remainder(&self) -> &'input [u8] {
        self.remainder
    }
}

impl fmt::Debug for Http1EmptyTrailerSection<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Http1EmptyTrailerSection")
            .field("encoded_section", &"[REDACTED]")
            .field("remainder", &"[REDACTED]")
            .finish()
    }
}

/// Stable payload-free empty-section failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1EmptyTrailerSectionError {
    IncompleteSection,
    InvalidLineEnding,
    TrailerFieldsUnsupported,
}

impl fmt::Display for Http1EmptyTrailerSectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::IncompleteSection => "incomplete HTTP empty trailer section",
            Self::InvalidLineEnding => "invalid HTTP empty trailer-section line ending",
            Self::TrailerFieldsUnsupported => "HTTP trailer fields are unsupported",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1EmptyTrailerSectionError {}

/// Decode one exact CRLF representing an empty trailer section.
pub fn decode_http1_empty_trailer_section(
    input: &[u8],
) -> Result<Http1EmptyTrailerSection<'_>, Http1EmptyTrailerSectionError> {
    let Some(first) = input.first() else {
        return Err(Http1EmptyTrailerSectionError::IncompleteSection);
    };
    if *first == b'\n' {
        return Err(Http1EmptyTrailerSectionError::InvalidLineEnding);
    }
    if *first != b'\r' {
        return Err(Http1EmptyTrailerSectionError::TrailerFieldsUnsupported);
    }
    let Some(second) = input.get(1) else {
        return Err(Http1EmptyTrailerSectionError::IncompleteSection);
    };
    if *second != b'\n' {
        return Err(Http1EmptyTrailerSectionError::InvalidLineEnding);
    }

    Ok(Http1EmptyTrailerSection {
        encoded_section: &input[..2],
        remainder: &input[2..],
    })
}
