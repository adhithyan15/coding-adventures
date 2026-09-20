//! # Generic RFC 5280 SubjectPublicKeyInfo values
//!
//! This crate composes only the generic algorithm-plus-key-bit-string
//! container. It deliberately owns no algorithm registry or cryptographic
//! authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_bit_string, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind, DerBitString,
};
use std::error::Error;
use std::fmt;
use x509_algorithm_identifier::{
    decode_x509_algorithm_identifier, X509AlgorithmIdentifier, X509AlgorithmIdentifierError,
    X509AlgorithmIdentifierErrorKind,
};

/// Stable failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509SubjectPublicKeyInfoErrorKind {
    Structure(Asn1ErrorKind),
    MissingAlgorithm,
    InvalidAlgorithm(X509AlgorithmIdentifierErrorKind),
    MissingSubjectPublicKey,
    InvalidSubjectPublicKey(Asn1ErrorKind),
    TrailingElement,
}

/// A redacted failure at a byte offset local to the SubjectPublicKeyInfo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509SubjectPublicKeyInfoError {
    kind: X509SubjectPublicKeyInfoErrorKind,
    offset: usize,
}

impl X509SubjectPublicKeyInfoError {
    const fn new(kind: X509SubjectPublicKeyInfoErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509SubjectPublicKeyInfoErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(
            X509SubjectPublicKeyInfoErrorKind::Structure(error.kind()),
            offset,
        )
    }

    fn algorithm(error: X509AlgorithmIdentifierError, child_offset: usize) -> Self {
        Self::new(
            X509SubjectPublicKeyInfoErrorKind::InvalidAlgorithm(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn subject_public_key(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509SubjectPublicKeyInfoErrorKind::InvalidSubjectPublicKey(error.kind()),
            child_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> X509SubjectPublicKeyInfoErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509SubjectPublicKeyInfoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 subject public key info error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509SubjectPublicKeyInfoError {}

/// One borrowed generic RFC 5280 subject public key information value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509SubjectPublicKeyInfo<'a> {
    algorithm: X509AlgorithmIdentifier<'a>,
    subject_public_key: DerBitString<'a>,
}

impl<'a> X509SubjectPublicKeyInfo<'a> {
    pub const fn algorithm(self) -> X509AlgorithmIdentifier<'a> {
        self.algorithm
    }

    pub const fn subject_public_key(self) -> DerBitString<'a> {
        self.subject_public_key
    }
}

/// Decode one generic RFC 5280 `SubjectPublicKeyInfo` sequence.
pub fn decode_x509_subject_public_key_info<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509SubjectPublicKeyInfo<'a>, X509SubjectPublicKeyInfoError> {
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509SubjectPublicKeyInfoError::structure)?;

    let algorithm_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let algorithm_element = fields
        .read(decoder)
        .map_err(|error| {
            X509SubjectPublicKeyInfoError::child_structure(error, value_offset, algorithm_offset)
        })?
        .ok_or_else(|| {
            X509SubjectPublicKeyInfoError::new(
                X509SubjectPublicKeyInfoErrorKind::MissingAlgorithm,
                algorithm_offset,
            )
        })?;
    let algorithm = decode_x509_algorithm_identifier(decoder, algorithm_element)
        .map_err(|error| X509SubjectPublicKeyInfoError::algorithm(error, algorithm_offset))?;

    let subject_public_key_offset =
        child_offset(value_offset, value_length, fields.remaining().len());
    let subject_public_key_element = fields
        .read(decoder)
        .map_err(|error| {
            X509SubjectPublicKeyInfoError::child_structure(
                error,
                value_offset,
                subject_public_key_offset,
            )
        })?
        .ok_or_else(|| {
            X509SubjectPublicKeyInfoError::new(
                X509SubjectPublicKeyInfoErrorKind::MissingSubjectPublicKey,
                subject_public_key_offset,
            )
        })?;
    let subject_public_key = decode_bit_string(subject_public_key_element).map_err(|error| {
        X509SubjectPublicKeyInfoError::subject_public_key(error, subject_public_key_offset)
    })?;

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            X509SubjectPublicKeyInfoError::child_structure(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(X509SubjectPublicKeyInfoError::new(
            X509SubjectPublicKeyInfoErrorKind::TrailingElement,
            trailing_offset,
        ));
    }

    Ok(X509SubjectPublicKeyInfo {
        algorithm,
        subject_public_key,
    })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
