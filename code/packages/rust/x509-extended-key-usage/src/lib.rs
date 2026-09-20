//! # Bounded RFC 5280 Extended Key Usage values
//!
//! This crate recognizes and decodes only the Extended Key Usage extension.
//! It deliberately assigns no certificate purpose, certificate role,
//! certificate-path, trust-root, or cryptographic authority.

#![deny(unsafe_code)]

use der_asn1::{decode_object_identifier, Asn1Decoder, Asn1Error, Asn1ErrorKind, ObjectIdentifier};
use std::error::Error;
use std::fmt;
use x509_extension::X509Extension;

/// RFC 5280 `id-ce-extKeyUsage` (`2.5.29.37`).
pub const EXTENDED_KEY_USAGE_OID: &[u64] = &[2, 5, 29, 37];

/// Maximum number of purpose identifiers retained from one extension.
pub const MAX_EXTENDED_KEY_PURPOSES: usize = 64;

/// Stable failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtendedKeyUsageErrorKind {
    WrongExtensionId,
    Structure(Asn1ErrorKind),
    Empty,
    TooManyPurposes,
    InvalidPurpose(Asn1ErrorKind),
}

/// A redacted failure at an offset local to the encapsulated value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendedKeyUsageError {
    kind: ExtendedKeyUsageErrorKind,
    offset: usize,
}

impl ExtendedKeyUsageError {
    const fn new(kind: ExtendedKeyUsageErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            ExtendedKeyUsageErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(ExtendedKeyUsageErrorKind::Structure(error.kind()), offset)
    }

    fn purpose(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            ExtendedKeyUsageErrorKind::InvalidPurpose(error.kind()),
            child_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> ExtendedKeyUsageErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for ExtendedKeyUsageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 extended key usage error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for ExtendedKeyUsageError {}

/// One bounded, ordered collection of borrowed key-purpose identifiers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtendedKeyUsage<'a> {
    purposes: [Option<ObjectIdentifier<'a>>; MAX_EXTENDED_KEY_PURPOSES],
    len: usize,
}

impl<'a> ExtendedKeyUsage<'a> {
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: usize) -> Option<ObjectIdentifier<'a>> {
        self.purposes.get(index).copied().flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item = ObjectIdentifier<'a>> + '_ {
        self.purposes[..self.len].iter().copied().flatten()
    }

    pub fn contains(&self, purpose: &[u64]) -> bool {
        self.iter().any(|candidate| candidate.equals(purpose))
    }
}

/// Decode one generic extension as exact RFC 5280 Extended Key Usage.
pub fn decode_extended_key_usage<'a>(
    decoder: &mut Asn1Decoder,
    extension: X509Extension<'a>,
) -> Result<ExtendedKeyUsage<'a>, ExtendedKeyUsageError> {
    if !extension.extension_id().equals(EXTENDED_KEY_USAGE_OID) {
        return Err(ExtendedKeyUsageError::new(
            ExtendedKeyUsageErrorKind::WrongExtensionId,
            0,
        ));
    }

    let root = decoder
        .decode_exact(extension.extension_value())
        .map_err(ExtendedKeyUsageError::structure)?;
    let value_offset = root.header().len();
    let value_length = root.value().len();
    let mut children = decoder
        .sequence(root)
        .map_err(ExtendedKeyUsageError::structure)?;
    let mut purposes: [Option<ObjectIdentifier<'a>>; MAX_EXTENDED_KEY_PURPOSES] =
        [None; MAX_EXTENDED_KEY_PURPOSES];
    let mut len = 0usize;

    while !children.remaining().is_empty() {
        let child_offset = child_offset(value_offset, value_length, children.remaining().len());
        let child = children.read(decoder).map_err(|error| {
            ExtendedKeyUsageError::child_structure(error, value_offset, child_offset)
        })?;
        let child = child.expect("a non-empty DER cursor must yield one element");

        if len == MAX_EXTENDED_KEY_PURPOSES {
            return Err(ExtendedKeyUsageError::new(
                ExtendedKeyUsageErrorKind::TooManyPurposes,
                child_offset,
            ));
        }

        let purpose = decode_object_identifier(child, decoder.limits())
            .map_err(|error| ExtendedKeyUsageError::purpose(error, child_offset))?;
        purposes[len] = Some(purpose);
        len += 1;
    }

    if len == 0 {
        return Err(ExtendedKeyUsageError::new(
            ExtendedKeyUsageErrorKind::Empty,
            value_offset,
        ));
    }

    Ok(ExtendedKeyUsage { purposes, len })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
