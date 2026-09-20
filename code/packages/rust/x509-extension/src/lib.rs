//! # Generic RFC 5280 Extension values
//!
//! This crate composes only the generic OID, omitted-default critical flag,
//! and opaque OCTET STRING container. It deliberately owns no extension
//! registry, certificate policy, or cryptographic authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_boolean, decode_object_identifier, decode_octet_string, Asn1Decoder, Asn1Element,
    Asn1Error, Asn1ErrorKind, ObjectIdentifier,
};
use std::error::Error;
use std::fmt;

const BOOLEAN_TAG: u32 = 1;

/// Stable failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509ExtensionErrorKind {
    Structure(Asn1ErrorKind),
    MissingExtensionId,
    InvalidExtensionId(Asn1ErrorKind),
    InvalidCritical(Asn1ErrorKind),
    EncodedDefaultCritical,
    MissingExtensionValue,
    InvalidExtensionValue(Asn1ErrorKind),
    TrailingElement,
}

/// A redacted failure at a byte offset local to the Extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509ExtensionError {
    kind: X509ExtensionErrorKind,
    offset: usize,
}

impl X509ExtensionError {
    const fn new(kind: X509ExtensionErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509ExtensionErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(X509ExtensionErrorKind::Structure(error.kind()), offset)
    }

    fn extension_id(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509ExtensionErrorKind::InvalidExtensionId(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn critical(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509ExtensionErrorKind::InvalidCritical(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn extension_value(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509ExtensionErrorKind::InvalidExtensionValue(error.kind()),
            child_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> X509ExtensionErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509ExtensionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 extension error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509ExtensionError {}

/// One borrowed generic RFC 5280 extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509Extension<'a> {
    extension_id: ObjectIdentifier<'a>,
    critical: bool,
    extension_value: &'a [u8],
}

impl<'a> X509Extension<'a> {
    pub const fn extension_id(self) -> ObjectIdentifier<'a> {
        self.extension_id
    }

    pub const fn critical(self) -> bool {
        self.critical
    }

    pub const fn extension_value(self) -> &'a [u8] {
        self.extension_value
    }
}

/// Decode one generic RFC 5280 `Extension` sequence.
pub fn decode_x509_extension<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509Extension<'a>, X509ExtensionError> {
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509ExtensionError::structure)?;

    let extension_id_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let extension_id_element = fields
        .read(decoder)
        .map_err(|error| {
            X509ExtensionError::child_structure(error, value_offset, extension_id_offset)
        })?
        .ok_or_else(|| {
            X509ExtensionError::new(
                X509ExtensionErrorKind::MissingExtensionId,
                extension_id_offset,
            )
        })?;
    let extension_id = decode_object_identifier(extension_id_element, decoder.limits())
        .map_err(|error| X509ExtensionError::extension_id(error, extension_id_offset))?;

    let second_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let second = fields
        .read(decoder)
        .map_err(|error| X509ExtensionError::child_structure(error, value_offset, second_offset))?
        .ok_or_else(|| {
            X509ExtensionError::new(X509ExtensionErrorKind::MissingExtensionValue, second_offset)
        })?;

    let (critical, extension_value_element, extension_value_offset) =
        if second.tag().number == BOOLEAN_TAG {
            let critical = decode_boolean(second)
                .map_err(|error| X509ExtensionError::critical(error, second_offset))?;
            if !critical {
                return Err(X509ExtensionError::new(
                    X509ExtensionErrorKind::EncodedDefaultCritical,
                    second_offset,
                ));
            }
            let extension_value_offset =
                child_offset(value_offset, value_length, fields.remaining().len());
            let extension_value_element = fields
                .read(decoder)
                .map_err(|error| {
                    X509ExtensionError::child_structure(error, value_offset, extension_value_offset)
                })?
                .ok_or_else(|| {
                    X509ExtensionError::new(
                        X509ExtensionErrorKind::MissingExtensionValue,
                        extension_value_offset,
                    )
                })?;
            (true, extension_value_element, extension_value_offset)
        } else {
            (false, second, second_offset)
        };

    let extension_value = decode_octet_string(extension_value_element)
        .map_err(|error| X509ExtensionError::extension_value(error, extension_value_offset))?;

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| X509ExtensionError::child_structure(error, value_offset, trailing_offset))?
        .is_some()
    {
        return Err(X509ExtensionError::new(
            X509ExtensionErrorKind::TrailingElement,
            trailing_offset,
        ));
    }

    Ok(X509Extension {
        extension_id,
        critical,
        extension_value,
    })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
