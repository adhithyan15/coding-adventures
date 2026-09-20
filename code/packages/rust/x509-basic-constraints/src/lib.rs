//! # Bounded RFC 5280 Basic Constraints values
//!
//! This crate recognizes and decodes only the Basic Constraints extension. It
//! deliberately grants no CA, certificate-path, trust-root, or cryptographic
//! authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_boolean, decode_integer, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind, DerInteger,
};
use std::error::Error;
use std::fmt;
use x509_extension::X509Extension;

const BOOLEAN_TAG: u32 = 1;

/// RFC 5280 `id-ce-basicConstraints` (`2.5.29.19`).
pub const BASIC_CONSTRAINTS_OID: &[u64] = &[2, 5, 29, 19];

/// Stable failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BasicConstraintsErrorKind {
    WrongExtensionId,
    Structure(Asn1ErrorKind),
    InvalidCa(Asn1ErrorKind),
    EncodedDefaultCa,
    InvalidPathLen(Asn1ErrorKind),
    NegativePathLen,
    PathLenWithoutCa,
    TrailingElement,
}

/// A redacted failure at an offset local to the encapsulated value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BasicConstraintsError {
    kind: BasicConstraintsErrorKind,
    offset: usize,
}

impl BasicConstraintsError {
    const fn new(kind: BasicConstraintsErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            BasicConstraintsErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(BasicConstraintsErrorKind::Structure(error.kind()), offset)
    }

    fn ca(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            BasicConstraintsErrorKind::InvalidCa(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn path_len(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            BasicConstraintsErrorKind::InvalidPathLen(error.kind()),
            child_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> BasicConstraintsErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for BasicConstraintsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 basic constraints error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for BasicConstraintsError {}

/// One borrowed, validated RFC 5280 Basic Constraints value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BasicConstraints<'a> {
    is_ca: bool,
    path_len_constraint: Option<DerInteger<'a>>,
}

impl<'a> BasicConstraints<'a> {
    pub const fn is_ca(self) -> bool {
        self.is_ca
    }

    pub const fn path_len_constraint(self) -> Option<DerInteger<'a>> {
        self.path_len_constraint
    }
}

/// Decode one generic extension as exact RFC 5280 Basic Constraints.
pub fn decode_basic_constraints<'a>(
    decoder: &mut Asn1Decoder,
    extension: X509Extension<'a>,
) -> Result<BasicConstraints<'a>, BasicConstraintsError> {
    if !extension.extension_id().equals(BASIC_CONSTRAINTS_OID) {
        return Err(BasicConstraintsError::new(
            BasicConstraintsErrorKind::WrongExtensionId,
            0,
        ));
    }

    let root = decoder
        .decode_exact(extension.extension_value())
        .map_err(BasicConstraintsError::structure)?;
    decode_value(decoder, root)
}

fn decode_value<'a>(
    decoder: &mut Asn1Decoder,
    root: Asn1Element<'a>,
) -> Result<BasicConstraints<'a>, BasicConstraintsError> {
    let value_offset = root.header().len();
    let value_length = root.value().len();
    let mut fields = decoder
        .sequence(root)
        .map_err(BasicConstraintsError::structure)?;

    let first_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields.remaining().is_empty() {
        return Ok(BasicConstraints {
            is_ca: false,
            path_len_constraint: None,
        });
    }
    let first = fields
        .read(decoder)
        .map_err(|error| BasicConstraintsError::child_structure(error, value_offset, first_offset))?
        .expect("a non-empty DER cursor must yield one element");

    if first.tag().number != BOOLEAN_TAG {
        validate_path_len(first, first_offset)?;
        return Err(BasicConstraintsError::new(
            BasicConstraintsErrorKind::PathLenWithoutCa,
            first_offset,
        ));
    }

    let is_ca =
        decode_boolean(first).map_err(|error| BasicConstraintsError::ca(error, first_offset))?;
    if !is_ca {
        return Err(BasicConstraintsError::new(
            BasicConstraintsErrorKind::EncodedDefaultCa,
            first_offset,
        ));
    }

    let path_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let path_len_constraint = match fields
        .read(decoder)
        .map_err(|error| BasicConstraintsError::child_structure(error, value_offset, path_offset))?
    {
        Some(element) => Some(validate_path_len(element, path_offset)?),
        None => None,
    };

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            BasicConstraintsError::child_structure(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(BasicConstraintsError::new(
            BasicConstraintsErrorKind::TrailingElement,
            trailing_offset,
        ));
    }

    Ok(BasicConstraints {
        is_ca,
        path_len_constraint,
    })
}

fn validate_path_len<'a>(
    element: Asn1Element<'a>,
    offset: usize,
) -> Result<DerInteger<'a>, BasicConstraintsError> {
    let integer =
        decode_integer(element).map_err(|error| BasicConstraintsError::path_len(error, offset))?;
    if integer.is_negative() {
        return Err(BasicConstraintsError::new(
            BasicConstraintsErrorKind::NegativePathLen,
            offset + element.header().len(),
        ));
    }
    Ok(integer)
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
