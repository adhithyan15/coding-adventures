//! # Allocation-free TLS 1.3 inner-plaintext framing
//!
//! Protected TLS 1.3 records hide their real content type inside authenticated
//! cleartext.  The cleartext contains content, one nonzero content-type byte,
//! and optional zero padding.  This crate owns that small syntax boundary; it
//! neither authenticates nor decrypts the bytes it receives.
//!
//! ```
//! use tls13_inner_plaintext::{
//!     decode_tls13_inner_plaintext, encode_tls13_inner_plaintext,
//!     Tls13InnerPlaintextPolicy,
//! };
//! use tls13_record::TlsContentType;
//!
//! let policy = Tls13InnerPlaintextPolicy::new(
//!     &[TlsContentType::ApplicationData],
//!     1024,
//! )?;
//! let mut encoded = [0xa5; 32];
//! let used = encode_tls13_inner_plaintext(
//!     &policy,
//!     TlsContentType::ApplicationData,
//!     b"hello",
//!     2,
//!     &mut encoded,
//! )?;
//! let decoded = decode_tls13_inner_plaintext(&policy, &encoded[..used])?;
//! assert_eq!(decoded.content(), b"hello");
//! assert_eq!(decoded.padding_length(), 2);
//! # Ok::<(), tls13_inner_plaintext::InnerPlaintextError>(())
//! ```

#![forbid(unsafe_code)]

use core::fmt;

pub use tls13_record::TlsContentType;

/// RFC 8446's absolute ceiling for the complete encoded `TLSInnerPlaintext`.
pub const TLS13_MAX_INNER_PLAINTEXT_LEN: usize = (1 << 14) + 1;

/// Stable, cleartext-free failure categories for policy and syntax validation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InnerPlaintextErrorKind {
    /// A policy allowed no protected content type.
    EmptyContentTypePolicy,
    /// A policy repeated a protected content type.
    DuplicateContentType,
    /// Protected ChangeCipherSpec is forbidden by TLS 1.3.
    ProtectedChangeCipherSpec,
    /// The caller's encoded-length ceiling was zero or exceeded the RFC limit.
    EncodedLengthLimitOutOfRange,
    /// No byte was available for content type or content.
    EmptyInnerPlaintext,
    /// The complete authenticated cleartext exceeded caller policy.
    InnerPlaintextTooLong,
    /// The authenticated cleartext contained only zero padding.
    MissingContentType,
    /// The final nonzero byte was not a recognized TLS content type.
    UnknownContentType,
    /// A recognized protected content type was not allowed by caller policy.
    DisallowedContentType,
    /// A protected handshake or alert carried no content.
    EmptyControlContent,
    /// Content, type, and padding lengths could not be added safely.
    EncodedLengthOverflow,
    /// Caller-owned encoding storage could not hold the complete value.
    OutputTooSmall,
}

/// One redacted TLS inner-plaintext failure.
///
/// The error stores only a stable category. It never retains authenticated
/// cleartext, content-type wire values, or lengths.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct InnerPlaintextError {
    kind: InnerPlaintextErrorKind,
}

impl InnerPlaintextError {
    const fn new(kind: InnerPlaintextErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(self) -> InnerPlaintextErrorKind {
        self.kind
    }
}

impl fmt::Display for InnerPlaintextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TLS 1.3 inner plaintext error {:?}", self.kind)
    }
}

impl std::error::Error for InnerPlaintextError {}

/// Caller-owned authorization and encoded-size policy for protected records.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Tls13InnerPlaintextPolicy {
    allowed_content_types: u8,
    maximum_encoded_length: usize,
}

impl Tls13InnerPlaintextPolicy {
    /// Validate a protected-content allowlist and complete encoded-length cap.
    ///
    /// The allowlist is data rather than state-machine logic. It must be
    /// non-empty and duplicate-free, and cannot contain ChangeCipherSpec.
    pub fn new(
        allowed_content_types: &[TlsContentType],
        maximum_encoded_length: usize,
    ) -> Result<Self, InnerPlaintextError> {
        if allowed_content_types.is_empty() {
            return Err(InnerPlaintextError::new(
                InnerPlaintextErrorKind::EmptyContentTypePolicy,
            ));
        }
        if maximum_encoded_length == 0 || maximum_encoded_length > TLS13_MAX_INNER_PLAINTEXT_LEN {
            return Err(InnerPlaintextError::new(
                InnerPlaintextErrorKind::EncodedLengthLimitOutOfRange,
            ));
        }

        let mut allowed = 0_u8;
        for content_type in allowed_content_types {
            if *content_type == TlsContentType::ChangeCipherSpec {
                return Err(InnerPlaintextError::new(
                    InnerPlaintextErrorKind::ProtectedChangeCipherSpec,
                ));
            }
            let bit = content_type_bit(*content_type);
            if allowed & bit != 0 {
                return Err(InnerPlaintextError::new(
                    InnerPlaintextErrorKind::DuplicateContentType,
                ));
            }
            allowed |= bit;
        }

        Ok(Self {
            allowed_content_types: allowed,
            maximum_encoded_length,
        })
    }

    /// Return the maximum complete encoded length accepted by this policy.
    #[must_use]
    pub const fn maximum_encoded_length(&self) -> usize {
        self.maximum_encoded_length
    }

    const fn allows(&self, content_type: TlsContentType) -> bool {
        self.allowed_content_types & content_type_bit(content_type) != 0
    }
}

const fn content_type_bit(content_type: TlsContentType) -> u8 {
    match content_type {
        TlsContentType::ChangeCipherSpec => 1 << 0,
        TlsContentType::Alert => 1 << 1,
        TlsContentType::Handshake => 1 << 2,
        TlsContentType::ApplicationData => 1 << 3,
    }
}

fn protected_content_type_from_wire(value: u8) -> Result<TlsContentType, InnerPlaintextError> {
    match value {
        20 => Err(InnerPlaintextError::new(
            InnerPlaintextErrorKind::ProtectedChangeCipherSpec,
        )),
        21 => Ok(TlsContentType::Alert),
        22 => Ok(TlsContentType::Handshake),
        23 => Ok(TlsContentType::ApplicationData),
        _ => Err(InnerPlaintextError::new(
            InnerPlaintextErrorKind::UnknownContentType,
        )),
    }
}

fn validate_content(
    policy: &Tls13InnerPlaintextPolicy,
    content_type: TlsContentType,
    content: &[u8],
) -> Result<(), InnerPlaintextError> {
    if content_type == TlsContentType::ChangeCipherSpec {
        return Err(InnerPlaintextError::new(
            InnerPlaintextErrorKind::ProtectedChangeCipherSpec,
        ));
    }
    if !policy.allows(content_type) {
        return Err(InnerPlaintextError::new(
            InnerPlaintextErrorKind::DisallowedContentType,
        ));
    }
    if content.is_empty()
        && matches!(
            content_type,
            TlsContentType::Handshake | TlsContentType::Alert
        )
    {
        return Err(InnerPlaintextError::new(
            InnerPlaintextErrorKind::EmptyControlContent,
        ));
    }
    Ok(())
}

/// Borrowed evidence for one validated `TLSInnerPlaintext` value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Tls13InnerPlaintext<'a> {
    content_type: TlsContentType,
    content: &'a [u8],
    padding_length: usize,
}

impl<'a> Tls13InnerPlaintext<'a> {
    /// Return the authenticated inner content type.
    #[must_use]
    pub const fn content_type(&self) -> TlsContentType {
        self.content_type
    }

    /// Borrow the authenticated content bytes.
    #[must_use]
    pub const fn content(&self) -> &'a [u8] {
        self.content
    }

    /// Return the number of verified trailing zero-padding bytes.
    #[must_use]
    pub const fn padding_length(&self) -> usize {
        self.padding_length
    }
}

/// Decode one complete, already-authenticated TLS 1.3 inner plaintext.
///
/// The complete slice is bounded before the reverse padding scan begins. This
/// function does not authenticate or decrypt its input; callers must invoke it
/// only after a future AEAD boundary has successfully authenticated the record.
pub fn decode_tls13_inner_plaintext<'a>(
    policy: &Tls13InnerPlaintextPolicy,
    authenticated_cleartext: &'a [u8],
) -> Result<Tls13InnerPlaintext<'a>, InnerPlaintextError> {
    if authenticated_cleartext.is_empty() {
        return Err(InnerPlaintextError::new(
            InnerPlaintextErrorKind::EmptyInnerPlaintext,
        ));
    }
    if authenticated_cleartext.len() > policy.maximum_encoded_length {
        return Err(InnerPlaintextError::new(
            InnerPlaintextErrorKind::InnerPlaintextTooLong,
        ));
    }

    let content_type_index = authenticated_cleartext
        .iter()
        .rposition(|byte| *byte != 0)
        .ok_or_else(|| InnerPlaintextError::new(InnerPlaintextErrorKind::MissingContentType))?;
    let content_type =
        protected_content_type_from_wire(authenticated_cleartext[content_type_index])?;
    let content = &authenticated_cleartext[..content_type_index];
    validate_content(policy, content_type, content)?;

    Ok(Tls13InnerPlaintext {
        content_type,
        content,
        padding_length: authenticated_cleartext.len() - content_type_index - 1,
    })
}

/// Encode one TLS 1.3 inner plaintext into caller-owned storage.
///
/// Output remains unchanged unless type policy, content emptiness, checked
/// length arithmetic, the complete encoded-length ceiling, and output capacity
/// have all passed validation.
pub fn encode_tls13_inner_plaintext(
    policy: &Tls13InnerPlaintextPolicy,
    content_type: TlsContentType,
    content: &[u8],
    padding_length: usize,
    output: &mut [u8],
) -> Result<usize, InnerPlaintextError> {
    validate_content(policy, content_type, content)?;
    let encoded_length = content
        .len()
        .checked_add(1)
        .and_then(|length| length.checked_add(padding_length))
        .ok_or_else(|| InnerPlaintextError::new(InnerPlaintextErrorKind::EncodedLengthOverflow))?;
    if encoded_length > policy.maximum_encoded_length {
        return Err(InnerPlaintextError::new(
            InnerPlaintextErrorKind::InnerPlaintextTooLong,
        ));
    }
    if output.len() < encoded_length {
        return Err(InnerPlaintextError::new(
            InnerPlaintextErrorKind::OutputTooSmall,
        ));
    }

    output[..content.len()].copy_from_slice(content);
    output[content.len()] = content_type.wire_value();
    output[content.len() + 1..encoded_length].fill(0);
    Ok(encoded_length)
}
