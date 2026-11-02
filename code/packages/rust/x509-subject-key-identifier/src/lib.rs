//! # Bounded RFC 5280 Subject Key Identifier values
//!
//! This crate recognizes and decodes only the Subject Key Identifier
//! extension. It borrows one exact non-empty identifier under a caller-owned
//! byte ceiling and grants no issuer, path, trust, or cryptographic authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{decode_octet_string, Asn1Decoder, Asn1Error, Asn1ErrorKind};
use x509_extension::X509Extension;

/// RFC 5280 `id-ce-subjectKeyIdentifier` (`2.5.29.14`).
pub const SUBJECT_KEY_IDENTIFIER_OID: &[u64] = &[2, 5, 29, 14];

/// Stable policy-construction failures retaining no caller values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubjectKeyIdentifierPolicyErrorKind {
    ZeroMaximumIdentifierBytes,
}

/// A redacted Subject Key Identifier policy-construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SubjectKeyIdentifierPolicyError {
    kind: SubjectKeyIdentifierPolicyErrorKind,
}

impl SubjectKeyIdentifierPolicyError {
    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> SubjectKeyIdentifierPolicyErrorKind {
        self.kind
    }
}

impl fmt::Debug for SubjectKeyIdentifierPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubjectKeyIdentifierPolicyError")
            .field("kind", &self.kind)
            .finish()
    }
}
impl fmt::Display for SubjectKeyIdentifierPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 Subject Key Identifier policy error {:?}",
            self.kind
        )
    }
}
impl std::error::Error for SubjectKeyIdentifierPolicyError {}

/// One validated caller-owned identifier-length ceiling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubjectKeyIdentifierPolicy {
    maximum_identifier_bytes: usize,
}

impl SubjectKeyIdentifierPolicy {
    /// Construct a nonzero maximum accepted identifier length.
    pub const fn new(
        maximum_identifier_bytes: usize,
    ) -> Result<Self, SubjectKeyIdentifierPolicyError> {
        if maximum_identifier_bytes == 0 {
            return Err(SubjectKeyIdentifierPolicyError {
                kind: SubjectKeyIdentifierPolicyErrorKind::ZeroMaximumIdentifierBytes,
            });
        }
        Ok(Self {
            maximum_identifier_bytes,
        })
    }

    /// Return the caller-selected maximum identifier length.
    #[must_use]
    pub const fn maximum_identifier_bytes(self) -> usize {
        self.maximum_identifier_bytes
    }
}

/// Stable decoding failures retaining no identifier bytes or lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubjectKeyIdentifierErrorKind {
    WrongExtensionId,
    Structure(Asn1ErrorKind),
    InvalidIdentifier(Asn1ErrorKind),
    EmptyIdentifier,
    IdentifierTooLong,
}

/// A redacted failure at an offset local to the encapsulated value.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SubjectKeyIdentifierError {
    kind: SubjectKeyIdentifierErrorKind,
    offset: usize,
}

impl SubjectKeyIdentifierError {
    const fn new(kind: SubjectKeyIdentifierErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }
    fn structure(error: Asn1Error) -> Self {
        Self::new(
            SubjectKeyIdentifierErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }
    fn identifier(error: Asn1Error) -> Self {
        Self::new(
            SubjectKeyIdentifierErrorKind::InvalidIdentifier(error.kind()),
            error.offset(),
        )
    }
    #[must_use]
    pub const fn kind(self) -> SubjectKeyIdentifierErrorKind {
        self.kind
    }
    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Debug for SubjectKeyIdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubjectKeyIdentifierError")
            .field("kind", &self.kind)
            .field("offset", &self.offset)
            .finish()
    }
}
impl fmt::Display for SubjectKeyIdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 Subject Key Identifier error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}
impl std::error::Error for SubjectKeyIdentifierError {}

/// One borrowed, validated RFC 5280 Subject Key Identifier value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubjectKeyIdentifier<'a> {
    identifier: &'a [u8],
}

impl<'a> SubjectKeyIdentifier<'a> {
    /// Return the exact non-empty identifier octets.
    #[must_use]
    pub const fn identifier(self) -> &'a [u8] {
        self.identifier
    }
}

/// Decode one generic extension as exact RFC 5280 Subject Key Identifier.
pub fn decode_subject_key_identifier<'a>(
    decoder: &mut Asn1Decoder,
    extension: X509Extension<'a>,
    policy: SubjectKeyIdentifierPolicy,
) -> Result<SubjectKeyIdentifier<'a>, SubjectKeyIdentifierError> {
    if !extension.extension_id().equals(SUBJECT_KEY_IDENTIFIER_OID) {
        return Err(SubjectKeyIdentifierError::new(
            SubjectKeyIdentifierErrorKind::WrongExtensionId,
            0,
        ));
    }

    let element = decoder
        .decode_exact(extension.extension_value())
        .map_err(SubjectKeyIdentifierError::structure)?;
    let value_offset = element.header().len();
    let identifier = decode_octet_string(element).map_err(SubjectKeyIdentifierError::identifier)?;
    if identifier.is_empty() {
        return Err(SubjectKeyIdentifierError::new(
            SubjectKeyIdentifierErrorKind::EmptyIdentifier,
            value_offset,
        ));
    }
    if identifier.len() > policy.maximum_identifier_bytes {
        return Err(SubjectKeyIdentifierError::new(
            SubjectKeyIdentifierErrorKind::IdentifierTooLong,
            value_offset,
        ));
    }

    Ok(SubjectKeyIdentifier { identifier })
}
