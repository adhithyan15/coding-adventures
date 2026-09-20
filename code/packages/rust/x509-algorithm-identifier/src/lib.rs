//! # Generic RFC 5280 AlgorithmIdentifier values
//!
//! This crate decodes only the generic OID-plus-optional-parameters container.
//! It deliberately owns no algorithm registry or cryptographic authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_object_identifier, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind, ObjectIdentifier,
};
use std::error::Error;
use std::fmt;

/// Stable failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509AlgorithmIdentifierErrorKind {
    Structure(Asn1ErrorKind),
    MissingAlgorithm,
    InvalidAlgorithm(Asn1ErrorKind),
    TrailingElement,
}

/// A redacted failure at a byte offset local to the AlgorithmIdentifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509AlgorithmIdentifierError {
    kind: X509AlgorithmIdentifierErrorKind,
    offset: usize,
}

impl X509AlgorithmIdentifierError {
    const fn new(kind: X509AlgorithmIdentifierErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509AlgorithmIdentifierErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(
            X509AlgorithmIdentifierErrorKind::Structure(error.kind()),
            offset,
        )
    }

    fn algorithm(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509AlgorithmIdentifierErrorKind::InvalidAlgorithm(error.kind()),
            child_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> X509AlgorithmIdentifierErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509AlgorithmIdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 algorithm identifier error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509AlgorithmIdentifierError {}

/// One borrowed generic RFC 5280 algorithm identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509AlgorithmIdentifier<'a> {
    algorithm: ObjectIdentifier<'a>,
    parameters: Option<Asn1Element<'a>>,
}

impl<'a> X509AlgorithmIdentifier<'a> {
    pub const fn algorithm(self) -> ObjectIdentifier<'a> {
        self.algorithm
    }

    pub const fn parameters(self) -> Option<Asn1Element<'a>> {
        self.parameters
    }
}

/// Decode one generic RFC 5280 `AlgorithmIdentifier` sequence.
pub fn decode_x509_algorithm_identifier<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509AlgorithmIdentifier<'a>, X509AlgorithmIdentifierError> {
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509AlgorithmIdentifierError::structure)?;

    let algorithm_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let algorithm_element = fields
        .read(decoder)
        .map_err(|error| {
            X509AlgorithmIdentifierError::child_structure(error, value_offset, algorithm_offset)
        })?
        .ok_or_else(|| {
            X509AlgorithmIdentifierError::new(
                X509AlgorithmIdentifierErrorKind::MissingAlgorithm,
                algorithm_offset,
            )
        })?;
    let algorithm = decode_object_identifier(algorithm_element, decoder.limits())
        .map_err(|error| X509AlgorithmIdentifierError::algorithm(error, algorithm_offset))?;

    let parameters_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let parameters = fields.read(decoder).map_err(|error| {
        X509AlgorithmIdentifierError::child_structure(error, value_offset, parameters_offset)
    })?;

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            X509AlgorithmIdentifierError::child_structure(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(X509AlgorithmIdentifierError::new(
            X509AlgorithmIdentifierErrorKind::TrailingElement,
            trailing_offset,
        ));
    }
    Ok(X509AlgorithmIdentifier {
        algorithm,
        parameters,
    })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
