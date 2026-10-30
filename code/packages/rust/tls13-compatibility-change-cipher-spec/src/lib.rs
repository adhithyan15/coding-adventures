//! Exact TLS 1.3 compatibility ChangeCipherSpec body framing.
//!
//! RFC 8446 permits a narrow, unprotected compatibility record whose complete
//! body is the single octet `0x01`. This crate owns only that canonical body.
//! A later handshake state machine must decide whether the record occurs in an
//! allowed compatibility window and silently discard it outside the transcript.
//!
//! ```
//! use tls13_compatibility_change_cipher_spec::{
//!     decode_compatibility_change_cipher_spec,
//!     encode_compatibility_change_cipher_spec,
//! };
//!
//! let message = decode_compatibility_change_cipher_spec(&[1])?;
//! let mut output = [0xa5; 2];
//! let used = encode_compatibility_change_cipher_spec(message, &mut output)?;
//! assert_eq!(&output[..used], &[1]);
//! assert_eq!(output[used], 0xa5);
//! # Ok::<(), tls13_compatibility_change_cipher_spec::CompatibilityChangeCipherSpecError>(())
//! ```

#![forbid(unsafe_code)]

use core::fmt;

/// The exact RFC 8446 compatibility ChangeCipherSpec body length.
pub const COMPATIBILITY_CHANGE_CIPHER_SPEC_BODY_LENGTH: usize = 1;

/// Typed evidence that one exact compatibility body was canonical.
///
/// The private field prevents callers from forging this value without passing
/// through [`decode_compatibility_change_cipher_spec`]. The value retains no
/// caller bytes and grants no handshake-state or transcript authority.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct CompatibilityChangeCipherSpec {
    _private: (),
}

impl CompatibilityChangeCipherSpec {
    /// Return the single canonical RFC wire octet.
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        1
    }
}

impl fmt::Debug for CompatibilityChangeCipherSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CompatibilityChangeCipherSpec")
    }
}

/// Stable, payload-free compatibility-body failures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompatibilityChangeCipherSpecErrorKind {
    /// The body was shorter or longer than exactly one byte.
    InvalidBodyLength,
    /// The single body octet was not the canonical value `0x01`.
    InvalidBody,
    /// Caller-owned output could not hold the complete body.
    OutputTooSmall,
}

/// One redacted compatibility-body failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CompatibilityChangeCipherSpecError {
    kind: CompatibilityChangeCipherSpecErrorKind,
}

impl CompatibilityChangeCipherSpecError {
    const fn new(kind: CompatibilityChangeCipherSpecErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(self) -> CompatibilityChangeCipherSpecErrorKind {
        self.kind
    }
}

impl fmt::Display for CompatibilityChangeCipherSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS 1.3 compatibility ChangeCipherSpec error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for CompatibilityChangeCipherSpecError {}

/// Decode one exact RFC 8446 compatibility ChangeCipherSpec body.
pub fn decode_compatibility_change_cipher_spec(
    body: &[u8],
) -> Result<CompatibilityChangeCipherSpec, CompatibilityChangeCipherSpecError> {
    let [value] = body else {
        return Err(CompatibilityChangeCipherSpecError::new(
            CompatibilityChangeCipherSpecErrorKind::InvalidBodyLength,
        ));
    };
    if *value != 1 {
        return Err(CompatibilityChangeCipherSpecError::new(
            CompatibilityChangeCipherSpecErrorKind::InvalidBody,
        ));
    }

    Ok(CompatibilityChangeCipherSpec { _private: () })
}

/// Encode one canonical compatibility body into caller-owned output.
///
/// Capacity is validated before the output byte changes. Extra caller capacity
/// remains untouched.
pub fn encode_compatibility_change_cipher_spec(
    message: CompatibilityChangeCipherSpec,
    output: &mut [u8],
) -> Result<usize, CompatibilityChangeCipherSpecError> {
    let Some(first) = output.first_mut() else {
        return Err(CompatibilityChangeCipherSpecError::new(
            CompatibilityChangeCipherSpecErrorKind::OutputTooSmall,
        ));
    };

    *first = message.wire_value();
    Ok(COMPATIBILITY_CHANGE_CIPHER_SPEC_BODY_LENGTH)
}
