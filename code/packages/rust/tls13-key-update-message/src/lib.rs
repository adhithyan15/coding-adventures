//! Exact TLS 1.3 `KeyUpdate` handshake-body framing.
//!
//! RFC 8446 section 4.6.3 gives `KeyUpdate` one byte whose two valid values
//! state whether the peer requests a response. This crate owns only that
//! syntax and returns a typed response requirement; it does not rotate traffic
//! secrets or sequence post-handshake messages.
//!
//! ```
//! use tls13_key_update_message::{
//!     decode_key_update, encode_key_update, KeyUpdateRequest,
//! };
//!
//! let request = decode_key_update(&[1])?;
//! assert_eq!(request, KeyUpdateRequest::UpdateRequested);
//! assert_eq!(
//!     request.required_response(),
//!     Some(KeyUpdateRequest::UpdateNotRequested),
//! );
//!
//! let mut output = [0xa5; 2];
//! let used = encode_key_update(request, &mut output)?;
//! assert_eq!(&output[..used], &[1]);
//! assert_eq!(output[used], 0xa5);
//! # Ok::<(), tls13_key_update_message::KeyUpdateError>(())
//! ```

#![forbid(unsafe_code)]

use core::fmt;

/// The exact RFC 8446 `KeyUpdate` body length.
pub const KEY_UPDATE_BODY_LENGTH: usize = 1;

/// The complete RFC 8446 `KeyUpdateRequest` enum.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum KeyUpdateRequest {
    /// Rotate the receive keys without sending a reciprocal update.
    UpdateNotRequested = 0,
    /// Rotate the receive keys and send one reciprocal non-requesting update.
    UpdateRequested = 1,
}

impl KeyUpdateRequest {
    /// Return the exact RFC wire value.
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Return whether this request requires a reciprocal `KeyUpdate`.
    #[must_use]
    pub const fn requires_response(self) -> bool {
        matches!(self, Self::UpdateRequested)
    }

    /// Return the exact reciprocal request required before later application data.
    ///
    /// The later state machine still owns response scheduling and coalescing.
    #[must_use]
    pub const fn required_response(self) -> Option<Self> {
        match self {
            Self::UpdateNotRequested => None,
            Self::UpdateRequested => Some(Self::UpdateNotRequested),
        }
    }
}

/// Stable, payload-free `KeyUpdate` framing failures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeyUpdateErrorKind {
    /// The body was shorter or longer than exactly one byte.
    InvalidBodyLength,
    /// The request byte was neither zero nor one.
    InvalidRequest,
    /// Caller-owned output could not hold the complete body.
    OutputTooSmall,
}

/// One redacted `KeyUpdate` body failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct KeyUpdateError {
    kind: KeyUpdateErrorKind,
}

impl KeyUpdateError {
    const fn new(kind: KeyUpdateErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(self) -> KeyUpdateErrorKind {
        self.kind
    }
}

impl fmt::Display for KeyUpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TLS 1.3 KeyUpdate error {:?}", self.kind)
    }
}

impl std::error::Error for KeyUpdateError {}

/// Decode exactly one complete RFC 8446 section 4.6.3 `KeyUpdate` body.
pub fn decode_key_update(body: &[u8]) -> Result<KeyUpdateRequest, KeyUpdateError> {
    let [request] = body else {
        return Err(KeyUpdateError::new(KeyUpdateErrorKind::InvalidBodyLength));
    };

    match request {
        0 => Ok(KeyUpdateRequest::UpdateNotRequested),
        1 => Ok(KeyUpdateRequest::UpdateRequested),
        _ => Err(KeyUpdateError::new(KeyUpdateErrorKind::InvalidRequest)),
    }
}

/// Encode one typed `KeyUpdate` body into caller-owned output.
///
/// Capacity is validated before the single output byte is changed. Extra
/// caller capacity remains untouched.
pub fn encode_key_update(
    request: KeyUpdateRequest,
    output: &mut [u8],
) -> Result<usize, KeyUpdateError> {
    let Some(first) = output.first_mut() else {
        return Err(KeyUpdateError::new(KeyUpdateErrorKind::OutputTooSmall));
    };

    *first = request.wire_value();
    Ok(KEY_UPDATE_BODY_LENGTH)
}
