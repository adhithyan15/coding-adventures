//! # Bounded RFC 5280 Extensions sequences
//!
//! This crate composes a non-empty, unique-OID sequence of already generic
//! extensions. It deliberately owns no extension registry, certificate
//! policy, or cryptographic authority.

#![deny(unsafe_code)]

use der_asn1::{Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind};
use std::error::Error;
use std::fmt;
use x509_extension::{decode_x509_extension, X509Extension, X509ExtensionError};

/// Maximum number of extensions retained from one certificate.
pub const MAX_X509_EXTENSIONS: usize = 64;

/// Stable failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509ExtensionsErrorKind {
    Structure(Asn1ErrorKind),
    Empty,
    TooManyExtensions,
    InvalidExtension(x509_extension::X509ExtensionErrorKind),
    DuplicateExtensionId,
}

/// A redacted failure at a byte offset local to the Extensions sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509ExtensionsError {
    kind: X509ExtensionsErrorKind,
    offset: usize,
}

impl X509ExtensionsError {
    const fn new(kind: X509ExtensionsErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509ExtensionsErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(X509ExtensionsErrorKind::Structure(error.kind()), offset)
    }

    fn extension(error: X509ExtensionError, child_offset: usize) -> Self {
        Self::new(
            X509ExtensionsErrorKind::InvalidExtension(error.kind()),
            child_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> X509ExtensionsErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509ExtensionsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 extensions error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509ExtensionsError {}

/// One bounded, ordered collection of borrowed generic extensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X509Extensions<'a> {
    entries: [Option<X509Extension<'a>>; MAX_X509_EXTENSIONS],
    len: usize,
}

impl<'a> X509Extensions<'a> {
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: usize) -> Option<X509Extension<'a>> {
        self.entries.get(index).copied().flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item = X509Extension<'a>> + '_ {
        self.entries[..self.len].iter().copied().flatten()
    }
}

/// Decode one exact RFC 5280 `Extensions` sequence.
pub fn decode_x509_extensions<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509Extensions<'a>, X509ExtensionsError> {
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut children = decoder
        .sequence(element)
        .map_err(X509ExtensionsError::structure)?;
    let mut entries: [Option<X509Extension<'a>>; MAX_X509_EXTENSIONS] = [None; MAX_X509_EXTENSIONS];
    let mut len = 0usize;

    while !children.remaining().is_empty() {
        let child_offset = child_offset(value_offset, value_length, children.remaining().len());
        let child = children.read(decoder).map_err(|error| {
            X509ExtensionsError::child_structure(error, value_offset, child_offset)
        })?;
        let child = child.expect("a non-empty DER cursor must yield one element");

        if len == MAX_X509_EXTENSIONS {
            return Err(X509ExtensionsError::new(
                X509ExtensionsErrorKind::TooManyExtensions,
                child_offset,
            ));
        }

        let extension = decode_x509_extension(decoder, child)
            .map_err(|error| X509ExtensionsError::extension(error, child_offset))?;
        let duplicate = entries[..len].iter().flatten().any(|existing| {
            existing.extension_id().encoded() == extension.extension_id().encoded()
        });
        if duplicate {
            return Err(X509ExtensionsError::new(
                X509ExtensionsErrorKind::DuplicateExtensionId,
                child_offset,
            ));
        }

        entries[len] = Some(extension);
        len += 1;
    }

    if len == 0 {
        return Err(X509ExtensionsError::new(
            X509ExtensionsErrorKind::Empty,
            value_offset,
        ));
    }

    Ok(X509Extensions { entries, len })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
