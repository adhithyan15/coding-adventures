//! # RFC 6960 OCSP certificate identifiers
//!
//! An OCSP response names a certificate through four values: a hash algorithm,
//! hashes of the issuer's name and public key, and the certificate serial
//! number. This crate decodes only that `CertID` container. It deliberately
//! owns no hash registry, certificate binding, response signature, responder
//! authorization, status, freshness, or transport authority.

#![deny(unsafe_code)]

use der_asn1::{decode_octet_string, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind};
use std::error::Error;
use std::fmt;
use x509_algorithm_identifier::{
    decode_x509_algorithm_identifier, X509AlgorithmIdentifier, X509AlgorithmIdentifierError,
    X509AlgorithmIdentifierErrorKind,
};
use x509_serial_number::{
    decode_x509_serial_number, X509SerialNumber, X509SerialNumberError, X509SerialNumberErrorKind,
};

/// Stable failure categories. No category retains identifier bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509OcspCertIdErrorKind {
    Structure(Asn1ErrorKind),
    MissingHashAlgorithm,
    InvalidHashAlgorithm(X509AlgorithmIdentifierErrorKind),
    MissingIssuerNameHash,
    InvalidIssuerNameHash(Asn1ErrorKind),
    MissingIssuerKeyHash,
    InvalidIssuerKeyHash(Asn1ErrorKind),
    MissingSerialNumber,
    InvalidSerialNumber(X509SerialNumberErrorKind),
    TrailingElement,
}

/// A redacted failure at a byte offset local to the `CertID` element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509OcspCertIdError {
    kind: X509OcspCertIdErrorKind,
    offset: usize,
}

impl X509OcspCertIdError {
    const fn new(kind: X509OcspCertIdErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509OcspCertIdErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(X509OcspCertIdErrorKind::Structure(error.kind()), offset)
    }

    fn hash_algorithm(error: X509AlgorithmIdentifierError, child_offset: usize) -> Self {
        Self::new(
            X509OcspCertIdErrorKind::InvalidHashAlgorithm(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn issuer_name_hash(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509OcspCertIdErrorKind::InvalidIssuerNameHash(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn issuer_key_hash(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509OcspCertIdErrorKind::InvalidIssuerKeyHash(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn serial_number(error: X509SerialNumberError, child_offset: usize) -> Self {
        Self::new(
            X509OcspCertIdErrorKind::InvalidSerialNumber(error.kind()),
            child_offset + error.offset(),
        )
    }

    /// Return the stable, payload-free failure category.
    pub const fn kind(self) -> X509OcspCertIdErrorKind {
        self.kind
    }

    /// Return the failing byte offset relative to the `CertID` element.
    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509OcspCertIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP certificate identifier error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509OcspCertIdError {}

/// One borrowed, structurally valid RFC 6960 `CertID`.
///
/// The OCTET STRING lengths remain uninterpreted because their required sizes
/// depend on the later data-driven hash-algorithm policy. Retaining bytes here
/// is identity data, not proof that they match any issuer or certificate.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct X509OcspCertId<'a> {
    contents: &'a [u8],
    hash_algorithm: X509AlgorithmIdentifier<'a>,
    issuer_name_hash: &'a [u8],
    issuer_key_hash: &'a [u8],
    serial_number: X509SerialNumber<'a>,
}

impl<'a> X509OcspCertId<'a> {
    /// Return the canonical DER contents without the outer SEQUENCE framing.
    pub const fn contents(self) -> &'a [u8] {
        self.contents
    }

    /// Return the generic, structurally validated hash algorithm identifier.
    pub const fn hash_algorithm(self) -> X509AlgorithmIdentifier<'a> {
        self.hash_algorithm
    }

    /// Return the exact issuer-name hash OCTET STRING contents.
    pub const fn issuer_name_hash(self) -> &'a [u8] {
        self.issuer_name_hash
    }

    /// Return the exact issuer-key hash OCTET STRING contents.
    pub const fn issuer_key_hash(self) -> &'a [u8] {
        self.issuer_key_hash
    }

    /// Return the validated positive RFC 5280 certificate serial number.
    pub const fn serial_number(self) -> X509SerialNumber<'a> {
        self.serial_number
    }
}

impl fmt::Debug for X509OcspCertId<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("X509OcspCertId")
            .field("hash_algorithm", &"validated AlgorithmIdentifier")
            .field("issuer_name_hash_len", &self.issuer_name_hash.len())
            .field("issuer_key_hash_len", &self.issuer_key_hash.len())
            .field("serial_number_len", &self.serial_number.encoded_len())
            .finish()
    }
}

/// Decode one exact RFC 6960 `CertID` SEQUENCE.
///
/// The supplied decoder owns the shared DER depth, element, length, and
/// OID-arc budgets. Every nested decoder consumes those same budgets.
pub fn decode_x509_ocsp_cert_id<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509OcspCertId<'a>, X509OcspCertIdError> {
    let contents = element.value();
    let value_offset = element.header().len();
    let value_length = contents.len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509OcspCertIdError::structure)?;

    let hash_algorithm_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let hash_algorithm_element = fields
        .read(decoder)
        .map_err(|error| {
            X509OcspCertIdError::child_structure(error, value_offset, hash_algorithm_offset)
        })?
        .ok_or_else(|| {
            X509OcspCertIdError::new(
                X509OcspCertIdErrorKind::MissingHashAlgorithm,
                hash_algorithm_offset,
            )
        })?;
    let hash_algorithm = decode_x509_algorithm_identifier(decoder, hash_algorithm_element)
        .map_err(|error| X509OcspCertIdError::hash_algorithm(error, hash_algorithm_offset))?;

    let issuer_name_hash_offset =
        child_offset(value_offset, value_length, fields.remaining().len());
    let issuer_name_hash_element = fields
        .read(decoder)
        .map_err(|error| {
            X509OcspCertIdError::child_structure(error, value_offset, issuer_name_hash_offset)
        })?
        .ok_or_else(|| {
            X509OcspCertIdError::new(
                X509OcspCertIdErrorKind::MissingIssuerNameHash,
                issuer_name_hash_offset,
            )
        })?;
    let issuer_name_hash = decode_octet_string(issuer_name_hash_element)
        .map_err(|error| X509OcspCertIdError::issuer_name_hash(error, issuer_name_hash_offset))?;

    let issuer_key_hash_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let issuer_key_hash_element = fields
        .read(decoder)
        .map_err(|error| {
            X509OcspCertIdError::child_structure(error, value_offset, issuer_key_hash_offset)
        })?
        .ok_or_else(|| {
            X509OcspCertIdError::new(
                X509OcspCertIdErrorKind::MissingIssuerKeyHash,
                issuer_key_hash_offset,
            )
        })?;
    let issuer_key_hash = decode_octet_string(issuer_key_hash_element)
        .map_err(|error| X509OcspCertIdError::issuer_key_hash(error, issuer_key_hash_offset))?;

    let serial_number_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let serial_number_element = fields
        .read(decoder)
        .map_err(|error| {
            X509OcspCertIdError::child_structure(error, value_offset, serial_number_offset)
        })?
        .ok_or_else(|| {
            X509OcspCertIdError::new(
                X509OcspCertIdErrorKind::MissingSerialNumber,
                serial_number_offset,
            )
        })?;
    let serial_number = decode_x509_serial_number(serial_number_element)
        .map_err(|error| X509OcspCertIdError::serial_number(error, serial_number_offset))?;

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            X509OcspCertIdError::child_structure(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(X509OcspCertIdError::new(
            X509OcspCertIdErrorKind::TrailingElement,
            trailing_offset,
        ));
    }

    Ok(X509OcspCertId {
        contents,
        hash_algorithm,
        issuer_name_hash,
        issuer_key_hash,
        serial_number,
    })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
