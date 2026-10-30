//! # Bounded RFC 5280 Authority Key Identifier values
//!
//! This crate recognizes only the non-critical, keyIdentifier-only Authority
//! Key Identifier profile. Unsupported issuer/serial forms fail closed. The
//! borrowed result grants no issuer, path, trust, or cryptographic authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{decode_implicit_octet_string, Asn1Decoder, Asn1Error, Asn1ErrorKind};
use x509_extension::X509Extension;

/// RFC 5280 `id-ce-authorityKeyIdentifier` (`2.5.29.35`).
pub const AUTHORITY_KEY_IDENTIFIER_OID: &[u64] = &[2, 5, 29, 35];

/// Stable policy-construction failures retaining no caller values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityKeyIdentifierPolicyErrorKind {
    ZeroMaximumIdentifierBytes,
}

/// A redacted Authority Key Identifier policy failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AuthorityKeyIdentifierPolicyError {
    kind: AuthorityKeyIdentifierPolicyErrorKind,
}

impl AuthorityKeyIdentifierPolicyError {
    #[must_use]
    pub const fn kind(self) -> AuthorityKeyIdentifierPolicyErrorKind {
        self.kind
    }
}
impl fmt::Debug for AuthorityKeyIdentifierPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorityKeyIdentifierPolicyError")
            .field("kind", &self.kind)
            .finish()
    }
}
impl fmt::Display for AuthorityKeyIdentifierPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 Authority Key Identifier policy error {:?}",
            self.kind
        )
    }
}
impl std::error::Error for AuthorityKeyIdentifierPolicyError {}

/// One validated caller-owned key-identifier byte ceiling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorityKeyIdentifierPolicy {
    maximum_identifier_bytes: usize,
}

impl AuthorityKeyIdentifierPolicy {
    pub const fn new(
        maximum_identifier_bytes: usize,
    ) -> Result<Self, AuthorityKeyIdentifierPolicyError> {
        if maximum_identifier_bytes == 0 {
            return Err(AuthorityKeyIdentifierPolicyError {
                kind: AuthorityKeyIdentifierPolicyErrorKind::ZeroMaximumIdentifierBytes,
            });
        }
        Ok(Self {
            maximum_identifier_bytes,
        })
    }
    #[must_use]
    pub const fn maximum_identifier_bytes(self) -> usize {
        self.maximum_identifier_bytes
    }
}

/// Stable decoding failures retaining no identifier bytes or lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityKeyIdentifierErrorKind {
    WrongExtensionId,
    CriticalExtension,
    Structure(Asn1ErrorKind),
    UnsupportedProfile,
    EmptyKeyIdentifier,
    KeyIdentifierTooLong,
}

/// A redacted failure at an offset local to the encapsulated value.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AuthorityKeyIdentifierError {
    kind: AuthorityKeyIdentifierErrorKind,
    offset: usize,
}

impl AuthorityKeyIdentifierError {
    const fn new(kind: AuthorityKeyIdentifierErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }
    fn structure(error: Asn1Error) -> Self {
        Self::new(
            AuthorityKeyIdentifierErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }
    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(
            AuthorityKeyIdentifierErrorKind::Structure(error.kind()),
            offset,
        )
    }
    #[must_use]
    pub const fn kind(self) -> AuthorityKeyIdentifierErrorKind {
        self.kind
    }
    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset
    }
}
impl fmt::Debug for AuthorityKeyIdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorityKeyIdentifierError")
            .field("kind", &self.kind)
            .field("offset", &self.offset)
            .finish()
    }
}
impl fmt::Display for AuthorityKeyIdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 Authority Key Identifier error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}
impl std::error::Error for AuthorityKeyIdentifierError {}

/// One borrowed, validated keyIdentifier-only Authority Key Identifier value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorityKeyIdentifier<'a> {
    key_identifier: &'a [u8],
}
impl<'a> AuthorityKeyIdentifier<'a> {
    #[must_use]
    pub const fn key_identifier(self) -> &'a [u8] {
        self.key_identifier
    }
}

/// Decode the supported exact non-critical keyIdentifier-only profile.
pub fn decode_authority_key_identifier<'a>(
    decoder: &mut Asn1Decoder,
    extension: X509Extension<'a>,
    policy: AuthorityKeyIdentifierPolicy,
) -> Result<AuthorityKeyIdentifier<'a>, AuthorityKeyIdentifierError> {
    if !extension
        .extension_id()
        .equals(AUTHORITY_KEY_IDENTIFIER_OID)
    {
        return Err(AuthorityKeyIdentifierError::new(
            AuthorityKeyIdentifierErrorKind::WrongExtensionId,
            0,
        ));
    }
    if extension.critical() {
        return Err(AuthorityKeyIdentifierError::new(
            AuthorityKeyIdentifierErrorKind::CriticalExtension,
            0,
        ));
    }

    let root = decoder
        .decode_exact_embedded_context(
            extension.extension_value_context(),
            extension.extension_value(),
        )
        .map_err(AuthorityKeyIdentifierError::structure)?;
    let value_offset = root.header().len();
    let value_length = root.value().len();
    let mut fields = decoder
        .sequence(root)
        .map_err(AuthorityKeyIdentifierError::structure)?;
    let key_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let key_element = fields
        .read(decoder)
        .map_err(|error| {
            AuthorityKeyIdentifierError::child_structure(error, value_offset, key_offset)
        })?
        .ok_or_else(|| {
            AuthorityKeyIdentifierError::new(
                AuthorityKeyIdentifierErrorKind::UnsupportedProfile,
                key_offset,
            )
        })?;
    let key_identifier = decode_implicit_octet_string(key_element, 0).map_err(|_| {
        AuthorityKeyIdentifierError::new(
            AuthorityKeyIdentifierErrorKind::UnsupportedProfile,
            key_offset,
        )
    })?;
    if key_identifier.is_empty() {
        return Err(AuthorityKeyIdentifierError::new(
            AuthorityKeyIdentifierErrorKind::EmptyKeyIdentifier,
            key_offset + key_element.header().len(),
        ));
    }
    if key_identifier.len() > policy.maximum_identifier_bytes {
        return Err(AuthorityKeyIdentifierError::new(
            AuthorityKeyIdentifierErrorKind::KeyIdentifierTooLong,
            key_offset + key_element.header().len(),
        ));
    }
    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            AuthorityKeyIdentifierError::child_structure(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(AuthorityKeyIdentifierError::new(
            AuthorityKeyIdentifierErrorKind::UnsupportedProfile,
            trailing_offset,
        ));
    }

    Ok(AuthorityKeyIdentifier { key_identifier })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
