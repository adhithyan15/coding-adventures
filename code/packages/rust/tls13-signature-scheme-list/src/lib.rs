//! Bounded TLS 1.3 signature-scheme list decoding.
//!
//! RFC 8446 section 4.2.3 gives `signature_algorithms` and
//! `signature_algorithms_cert` the same extension-data shape: a non-empty
//! two-byte-length-prefixed vector of two-byte `SignatureScheme` identifiers.
//! This crate owns only that framing. Unknown identifiers remain data for a
//! separate caller-owned policy.

#![forbid(unsafe_code)]

use core::{fmt, iter::FusedIterator};

/// The IANA extension type for `signature_algorithms`.
pub const SIGNATURE_ALGORITHMS_EXTENSION_TYPE: u16 = 13;

/// The IANA extension type for `signature_algorithms_cert`.
pub const SIGNATURE_ALGORITHMS_CERT_EXTENSION_TYPE: u16 = 50;

/// RFC 8446's largest possible number of two-byte scheme identifiers.
pub const MAX_SIGNATURE_SCHEME_COUNT: usize = 32_767;

/// Caller-selected resource limit for one decoded scheme list.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SignatureSchemeListLimits {
    max_scheme_count: usize,
}

impl SignatureSchemeListLimits {
    /// Validate a nonzero ceiling no larger than the RFC vector permits.
    pub const fn new(max_scheme_count: usize) -> Result<Self, SignatureSchemeListError> {
        if max_scheme_count == 0 || max_scheme_count > MAX_SIGNATURE_SCHEME_COUNT {
            return Err(SignatureSchemeListError::InvalidLimit);
        }
        Ok(Self { max_scheme_count })
    }

    /// Return the maximum accepted scheme count.
    #[must_use]
    pub const fn max_scheme_count(self) -> usize {
        self.max_scheme_count
    }
}

/// Stable, payload-free framing and resource failures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SignatureSchemeListError {
    /// The caller supplied a zero or out-of-protocol resource limit.
    InvalidLimit,
    /// Extension data omitted the required two-byte vector length.
    MissingListLength,
    /// The declared vector contained no scheme identifiers.
    EmptyList,
    /// The declared vector length could not contain complete two-byte values.
    OddListLength,
    /// The declared vector length did not consume the complete input.
    ListLengthMismatch,
    /// The declared scheme count exceeded the caller-selected ceiling.
    TooManySchemes,
}

impl fmt::Display for SignatureSchemeListError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimit => "invalid TLS signature-scheme list limit",
            Self::MissingListLength => "missing TLS signature-scheme list length",
            Self::EmptyList => "empty TLS signature-scheme list",
            Self::OddListLength => "odd TLS signature-scheme list length",
            Self::ListLengthMismatch => "TLS signature-scheme list length mismatch",
            Self::TooManySchemes => "TLS signature-scheme list exceeds the caller limit",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SignatureSchemeListError {}

/// One exact borrowed `SignatureSchemeList` extension-data value.
///
/// The value intentionally derives neither `Copy` nor `Clone`, keeping later
/// policy attached to the exact decode rather than a reconstructed list.
pub struct SignatureSchemeList<'input> {
    extension_data: &'input [u8],
    scheme_bytes: &'input [u8],
}

impl fmt::Debug for SignatureSchemeList<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignatureSchemeList")
            .field("scheme_count", &self.len())
            .field("extension_data", &"[OPAQUE]")
            .finish_non_exhaustive()
    }
}

impl<'input> SignatureSchemeList<'input> {
    /// Borrow the exact complete extension data supplied to the decoder.
    #[must_use]
    pub const fn extension_data(&self) -> &'input [u8] {
        self.extension_data
    }

    /// Return the number of scheme identifiers in preference order.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.scheme_bytes.len() / 2
    }

    /// Return whether the list is empty.
    ///
    /// A successfully decoded list is never empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.scheme_bytes.is_empty()
    }

    /// Return the identifier at one preference index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<u16> {
        let start = index.checked_mul(2)?;
        let end = start.checked_add(2)?;
        let bytes = self.scheme_bytes.get(start..end)?;
        Some(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    /// Iterate over every identifier in wire preference order.
    #[must_use]
    pub const fn iter(&self) -> SignatureSchemeIter<'input> {
        SignatureSchemeIter {
            remaining: self.scheme_bytes,
        }
    }

    /// Return whether the exact wire identifier occurs in the list.
    #[must_use]
    pub fn contains(&self, scheme: u16) -> bool {
        self.iter().any(|candidate| candidate == scheme)
    }
}

/// Exact-size iterator over borrowed two-byte scheme identifiers.
#[derive(Clone, Copy)]
pub struct SignatureSchemeIter<'input> {
    remaining: &'input [u8],
}

impl fmt::Debug for SignatureSchemeIter<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignatureSchemeIter")
            .field("remaining_scheme_count", &self.len())
            .field("remaining", &"[OPAQUE]")
            .finish_non_exhaustive()
    }
}

impl Iterator for SignatureSchemeIter<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        let [first, second, tail @ ..] = self.remaining else {
            return None;
        };
        let scheme = u16::from_be_bytes([*first, *second]);
        self.remaining = tail;
        Some(scheme)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining.len() / 2;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for SignatureSchemeIter<'_> {}
impl FusedIterator for SignatureSchemeIter<'_> {}

/// Decode one exact RFC 8446 section 4.2.3 `SignatureSchemeList`.
///
/// The caller-selected count ceiling is applied before the result exposes an
/// iterator. Unknown, private-use, and repeated identifiers remain ordered
/// wire data for a later policy layer.
pub fn decode_signature_scheme_list(
    extension_data: &[u8],
    limits: SignatureSchemeListLimits,
) -> Result<SignatureSchemeList<'_>, SignatureSchemeListError> {
    let [length_high, length_low, scheme_bytes @ ..] = extension_data else {
        return Err(SignatureSchemeListError::MissingListLength);
    };
    let declared_length = u16::from_be_bytes([*length_high, *length_low]) as usize;
    if declared_length == 0 {
        return Err(SignatureSchemeListError::EmptyList);
    }
    if declared_length & 1 != 0 {
        return Err(SignatureSchemeListError::OddListLength);
    }
    if scheme_bytes.len() != declared_length {
        return Err(SignatureSchemeListError::ListLengthMismatch);
    }
    if declared_length / 2 > limits.max_scheme_count {
        return Err(SignatureSchemeListError::TooManySchemes);
    }

    Ok(SignatureSchemeList {
        extension_data,
        scheme_bytes,
    })
}
