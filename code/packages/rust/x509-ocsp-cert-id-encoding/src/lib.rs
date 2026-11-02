//! # Allocation-free RFC 6960 OCSP `CertID` encoding
//!
//! This crate validates exact caller-owned algorithm and serial-number DER,
//! then frames those bytes with caller-owned issuer hashes as one `CertID`.
//! It owns no hash selection, hashing, certificate binding, issuer authority,
//! revocation, request, transport, or network authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_algorithm_identifier::{
    decode_x509_algorithm_identifier, X509AlgorithmIdentifier, X509AlgorithmIdentifierErrorKind,
};
use x509_serial_number::{decode_x509_serial_number, X509SerialNumber, X509SerialNumberErrorKind};

/// Stable identifier-encoding failures retaining no caller bytes or lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspCertIdEncodingErrorKind {
    /// The algorithm input is not one complete canonical DER element.
    AlgorithmIdentifierFraming(Asn1ErrorKind),
    /// The element is not a structurally valid `AlgorithmIdentifier`.
    InvalidAlgorithmIdentifier(X509AlgorithmIdentifierErrorKind),
    /// The serial input is not one complete canonical DER element.
    SerialNumberFraming(Asn1ErrorKind),
    /// The element is not a positive bounded RFC 5280 serial number.
    InvalidSerialNumber(X509SerialNumberErrorKind),
    /// A complete encoded length cannot be represented by this platform.
    LengthOverflow,
    /// Caller-owned output cannot hold the complete identifier.
    OutputTooSmall,
}

/// A redacted OCSP identifier-encoding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspCertIdEncodingError {
    kind: X509OcspCertIdEncodingErrorKind,
}

impl X509OcspCertIdEncodingError {
    const fn new(kind: X509OcspCertIdEncodingErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509OcspCertIdEncodingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspCertIdEncodingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP CertID encoding error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspCertIdEncodingError {}

/// Exact canonical inputs admitted for one OCSP `CertID` encoding.
///
/// This evidence is deliberately non-copyable. It retains the structural
/// decodes and every exact caller input, but proves no algorithm policy, hash
/// computation, certificate binding, or issuer relationship.
pub struct ValidatedX509OcspCertIdFields<'input> {
    algorithm_identifier_der: &'input [u8],
    algorithm_identifier: X509AlgorithmIdentifier<'input>,
    issuer_name_hash: &'input [u8],
    issuer_key_hash: &'input [u8],
    serial_number_der: &'input [u8],
    serial_number: X509SerialNumber<'input>,
    contents_len: usize,
    encoded_len: usize,
}

impl<'input> ValidatedX509OcspCertIdFields<'input> {
    /// Return the exact canonical `AlgorithmIdentifier` DER.
    #[must_use]
    pub const fn algorithm_identifier_der(&self) -> &'input [u8] {
        self.algorithm_identifier_der
    }

    /// Return the structurally decoded algorithm identifier.
    #[must_use]
    pub const fn algorithm_identifier(&self) -> X509AlgorithmIdentifier<'input> {
        self.algorithm_identifier
    }

    /// Return the exact caller-owned issuer-name hash bytes.
    #[must_use]
    pub const fn issuer_name_hash(&self) -> &'input [u8] {
        self.issuer_name_hash
    }

    /// Return the exact caller-owned issuer-key hash bytes.
    #[must_use]
    pub const fn issuer_key_hash(&self) -> &'input [u8] {
        self.issuer_key_hash
    }

    /// Return the exact canonical serial-number INTEGER DER.
    #[must_use]
    pub const fn serial_number_der(&self) -> &'input [u8] {
        self.serial_number_der
    }

    /// Return the validated positive serial number.
    #[must_use]
    pub const fn serial_number(&self) -> X509SerialNumber<'input> {
        self.serial_number
    }

    /// Return the exact output length required for this identifier.
    #[must_use]
    pub const fn encoded_len(&self) -> usize {
        self.encoded_len
    }
}

impl fmt::Debug for ValidatedX509OcspCertIdFields<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509OcspCertIdFields")
    }
}

/// Validate exact caller inputs and preflight one complete `CertID` length.
///
/// One decoder shares the caller-owned depth, element, length, and OID-arc
/// budgets across both DER inputs. Hash bytes remain uninterpreted because a
/// later data-driven algorithm policy owns their required lengths.
pub fn validate_x509_ocsp_cert_id_fields<'input>(
    algorithm_identifier_der: &'input [u8],
    issuer_name_hash: &'input [u8],
    issuer_key_hash: &'input [u8],
    serial_number_der: &'input [u8],
    limits: Asn1Limits,
) -> Result<ValidatedX509OcspCertIdFields<'input>, X509OcspCertIdEncodingError> {
    let mut decoder = Asn1Decoder::new(limits);
    let algorithm_element = decoder
        .decode_exact(algorithm_identifier_der)
        .map_err(|error| {
            X509OcspCertIdEncodingError::new(
                X509OcspCertIdEncodingErrorKind::AlgorithmIdentifierFraming(error.kind()),
            )
        })?;
    let algorithm_identifier = decode_x509_algorithm_identifier(&mut decoder, algorithm_element)
        .map_err(|error| {
            X509OcspCertIdEncodingError::new(
                X509OcspCertIdEncodingErrorKind::InvalidAlgorithmIdentifier(error.kind()),
            )
        })?;

    let serial_element = decoder.decode_exact(serial_number_der).map_err(|error| {
        X509OcspCertIdEncodingError::new(X509OcspCertIdEncodingErrorKind::SerialNumberFraming(
            error.kind(),
        ))
    })?;
    let serial_number = decode_x509_serial_number(serial_element).map_err(|error| {
        X509OcspCertIdEncodingError::new(X509OcspCertIdEncodingErrorKind::InvalidSerialNumber(
            error.kind(),
        ))
    })?;

    let name_hash_len = tlv_encoded_len(issuer_name_hash.len()).ok_or_else(|| {
        X509OcspCertIdEncodingError::new(X509OcspCertIdEncodingErrorKind::LengthOverflow)
    })?;
    let key_hash_len = tlv_encoded_len(issuer_key_hash.len()).ok_or_else(|| {
        X509OcspCertIdEncodingError::new(X509OcspCertIdEncodingErrorKind::LengthOverflow)
    })?;
    let contents_len = algorithm_identifier_der
        .len()
        .checked_add(name_hash_len)
        .and_then(|length| length.checked_add(key_hash_len))
        .and_then(|length| length.checked_add(serial_number_der.len()))
        .ok_or_else(|| {
            X509OcspCertIdEncodingError::new(X509OcspCertIdEncodingErrorKind::LengthOverflow)
        })?;
    let encoded_len = tlv_encoded_len(contents_len).ok_or_else(|| {
        X509OcspCertIdEncodingError::new(X509OcspCertIdEncodingErrorKind::LengthOverflow)
    })?;

    Ok(ValidatedX509OcspCertIdFields {
        algorithm_identifier_der,
        algorithm_identifier,
        issuer_name_hash,
        issuer_key_hash,
        serial_number_der,
        serial_number,
        contents_len,
        encoded_len,
    })
}

/// One encoded `CertID` bound to its exact validated caller inputs.
pub struct EncodedX509OcspCertId<'fields, 'input, 'output> {
    fields: &'fields ValidatedX509OcspCertIdFields<'input>,
    encoded: &'output [u8],
}

impl<'fields, 'input, 'output> EncodedX509OcspCertId<'fields, 'input, 'output> {
    /// Return the exact validated fields encoded by this value.
    #[must_use]
    pub const fn fields(&self) -> &'fields ValidatedX509OcspCertIdFields<'input> {
        self.fields
    }

    /// Return the complete canonical DER `CertID`.
    #[must_use]
    pub const fn encoded(&self) -> &'output [u8] {
        self.encoded
    }
}

impl fmt::Debug for EncodedX509OcspCertId<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EncodedX509OcspCertId")
    }
}

/// Encode one exact RFC 6960 `CertID` into caller-owned output.
///
/// Every nested length and complete output capacity was preflighted during
/// validation. An undersized output is rejected before any byte changes, and
/// extra capacity remains untouched.
pub fn encode_x509_ocsp_cert_id<'fields, 'input, 'output>(
    fields: &'fields ValidatedX509OcspCertIdFields<'input>,
    output: &'output mut [u8],
) -> Result<EncodedX509OcspCertId<'fields, 'input, 'output>, X509OcspCertIdEncodingError> {
    if output.len() < fields.encoded_len {
        return Err(X509OcspCertIdEncodingError::new(
            X509OcspCertIdEncodingErrorKind::OutputTooSmall,
        ));
    }

    let mut offset = 0;
    write_header(output, &mut offset, 0x30, fields.contents_len);
    copy_bytes(output, &mut offset, fields.algorithm_identifier_der);
    write_header(output, &mut offset, 0x04, fields.issuer_name_hash.len());
    copy_bytes(output, &mut offset, fields.issuer_name_hash);
    write_header(output, &mut offset, 0x04, fields.issuer_key_hash.len());
    copy_bytes(output, &mut offset, fields.issuer_key_hash);
    copy_bytes(output, &mut offset, fields.serial_number_der);
    debug_assert_eq!(offset, fields.encoded_len);

    Ok(EncodedX509OcspCertId {
        fields,
        encoded: &output[..offset],
    })
}

fn tlv_encoded_len(contents_len: usize) -> Option<usize> {
    1usize
        .checked_add(der_length_encoded_len(contents_len))?
        .checked_add(contents_len)
}

fn der_length_encoded_len(length: usize) -> usize {
    if length <= 127 {
        1
    } else {
        1 + ((usize::BITS - length.leading_zeros()).div_ceil(8) as usize)
    }
}

fn write_header(output: &mut [u8], offset: &mut usize, tag: u8, contents_len: usize) {
    output[*offset] = tag;
    *offset += 1;
    write_der_length(output, offset, contents_len);
}

fn write_der_length(output: &mut [u8], offset: &mut usize, length: usize) {
    if length <= 127 {
        output[*offset] = length as u8;
        *offset += 1;
        return;
    }

    let bytes = length.to_be_bytes();
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .expect("long-form DER length is nonzero");
    let significant = &bytes[first..];
    output[*offset] = 0x80 | significant.len() as u8;
    *offset += 1;
    copy_bytes(output, offset, significant);
}

fn copy_bytes(output: &mut [u8], offset: &mut usize, bytes: &[u8]) {
    let end = *offset + bytes.len();
    output[*offset..end].copy_from_slice(bytes);
    *offset = end;
}
