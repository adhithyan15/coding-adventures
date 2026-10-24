//! # RFC 6960 basic OCSP responses
//!
//! This crate decodes the signed `BasicOCSPResponse` envelope above the
//! repository-owned `ResponseData`, algorithm-identifier, BIT STRING, and
//! certificate structure layers. The result is syntax only: it deliberately
//! grants no algorithm, signature, responder, path, revocation, or transport
//! authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_bit_string, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind, DerBitString,
};
use der_tlv::TagClass;
use std::error::Error;
use std::fmt;
use x509_algorithm_identifier::{
    decode_x509_algorithm_identifier, X509AlgorithmIdentifier, X509AlgorithmIdentifierError,
    X509AlgorithmIdentifierErrorKind,
};
use x509_certificate::{
    decode_x509_certificate_element, X509Certificate, X509CertificateError,
    X509CertificateErrorKind,
};
use x509_ocsp_response_data::{
    decode_x509_ocsp_response_data, X509OcspResponseData, X509OcspResponseDataError,
    X509OcspResponseDataErrorKind,
};

const CERTIFICATES_TAG: u32 = 0;

/// Maximum number of embedded responder-support certificates retained.
///
/// Certificates contain their own bounded Names and Extensions, so this small
/// outer ceiling also bounds the composed allocation-free stack footprint.
pub const MAX_X509_OCSP_CERTIFICATES: usize = 1;

/// Stable failure categories. No category retains response or certificate bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509OcspBasicResponseErrorKind {
    Structure(Asn1ErrorKind),
    MissingResponseData,
    InvalidResponseData(X509OcspResponseDataErrorKind),
    MissingSignatureAlgorithm,
    InvalidSignatureAlgorithm(X509AlgorithmIdentifierErrorKind),
    MissingSignature,
    InvalidSignature(Asn1ErrorKind),
    InvalidCertificatesWrapper(Asn1ErrorKind),
    InvalidCertificates(Asn1ErrorKind),
    TooManyCertificates,
    InvalidCertificate(X509CertificateErrorKind),
    UnexpectedOptionalField,
    TrailingElement,
}

/// A redacted failure at a byte offset local to the complete response element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509OcspBasicResponseError {
    kind: X509OcspBasicResponseErrorKind,
    offset: usize,
}

impl X509OcspBasicResponseError {
    const fn new(kind: X509OcspBasicResponseErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509OcspBasicResponseErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(
            X509OcspBasicResponseErrorKind::Structure(error.kind()),
            offset,
        )
    }

    fn response_data(error: X509OcspResponseDataError, child_offset: usize) -> Self {
        Self::new(
            X509OcspBasicResponseErrorKind::InvalidResponseData(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn signature_algorithm(error: X509AlgorithmIdentifierError, child_offset: usize) -> Self {
        Self::new(
            X509OcspBasicResponseErrorKind::InvalidSignatureAlgorithm(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn signature(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509OcspBasicResponseErrorKind::InvalidSignature(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn certificates_wrapper(error: Asn1Error, wrapper_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => child_offset + error.offset(),
            _ => wrapper_offset + error.offset(),
        };
        Self::new(
            X509OcspBasicResponseErrorKind::InvalidCertificatesWrapper(error.kind()),
            offset,
        )
    }

    fn certificates(error: Asn1Error, sequence_offset: usize) -> Self {
        Self::new(
            X509OcspBasicResponseErrorKind::InvalidCertificates(error.kind()),
            sequence_offset + error.offset(),
        )
    }

    fn certificate(error: X509CertificateError, child_offset: usize) -> Self {
        Self::new(
            X509OcspBasicResponseErrorKind::InvalidCertificate(error.kind()),
            child_offset + error.offset(),
        )
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspBasicResponseErrorKind {
        self.kind
    }

    /// Return the failing byte offset relative to the complete response.
    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509OcspBasicResponseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 basic OCSP response error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509OcspBasicResponseError {}

/// One structurally decoded RFC 6960 `BasicOCSPResponse`.
#[derive(Clone, PartialEq, Eq)]
pub struct X509OcspBasicResponse<'a> {
    encoded: &'a [u8],
    response_data: X509OcspResponseData<'a>,
    signature_algorithm: X509AlgorithmIdentifier<'a>,
    signature: DerBitString<'a>,
    certificates: [Option<X509Certificate<'a>>; MAX_X509_OCSP_CERTIFICATES],
    certificate_count: usize,
}

impl<'a> X509OcspBasicResponse<'a> {
    /// Return the exact canonical complete envelope encoding.
    pub const fn encoded(&self) -> &'a [u8] {
        self.encoded
    }

    /// Return the structural response data whose exact encoding is signed.
    pub const fn response_data(&self) -> &X509OcspResponseData<'a> {
        &self.response_data
    }

    /// Return the generic signature algorithm identifier.
    ///
    /// Structural decoding does not recognize or enable this algorithm.
    pub const fn signature_algorithm(&self) -> X509AlgorithmIdentifier<'a> {
        self.signature_algorithm
    }

    /// Return the canonical borrowed signature BIT STRING.
    ///
    /// Structural decoding does not verify these bits.
    pub const fn signature(&self) -> DerBitString<'a> {
        self.signature
    }

    /// Return the number of embedded certificates.
    pub const fn len(&self) -> usize {
        self.certificate_count
    }

    /// Return whether no embedded certificates were present.
    pub const fn is_empty(&self) -> bool {
        self.certificate_count == 0
    }

    /// Return one embedded certificate by wire-order index.
    pub fn get(&self, index: usize) -> Option<&X509Certificate<'a>> {
        self.certificates.get(index).and_then(Option::as_ref)
    }

    /// Iterate over embedded certificates in wire order.
    pub fn iter(&self) -> impl Iterator<Item = &X509Certificate<'a>> {
        self.certificates[..self.certificate_count]
            .iter()
            .filter_map(Option::as_ref)
    }
}

impl fmt::Debug for X509OcspBasicResponse<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("X509OcspBasicResponse")
            .field("encoded_octet_count", &self.encoded.len())
            .field("response_count", &self.response_data.len())
            .field("signature_octet_count", &self.signature.bytes().len())
            .field("signature_unused_bits", &self.signature.unused_bits())
            .field("certificate_count", &self.certificate_count)
            .finish()
    }
}

/// Decode one exact RFC 6960 `BasicOCSPResponse` SEQUENCE.
///
/// The supplied decoder owns every shared DER budget. Success proves syntax
/// only; signature verification, responder authorization, path construction,
/// freshness, and revocation policy remain caller-owned boundaries.
pub fn decode_x509_ocsp_basic_response<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509OcspBasicResponse<'a>, X509OcspBasicResponseError> {
    let encoded = element.encoded();
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509OcspBasicResponseError::structure)?;

    let response_data_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let response_data_element = read_required(
        decoder,
        &mut fields,
        value_offset,
        response_data_offset,
        X509OcspBasicResponseErrorKind::MissingResponseData,
    )?;
    let response_data = decode_x509_ocsp_response_data(decoder, response_data_element)
        .map_err(|error| X509OcspBasicResponseError::response_data(error, response_data_offset))?;

    let signature_algorithm_offset =
        child_offset(value_offset, value_length, fields.remaining().len());
    let signature_algorithm_element = read_required(
        decoder,
        &mut fields,
        value_offset,
        signature_algorithm_offset,
        X509OcspBasicResponseErrorKind::MissingSignatureAlgorithm,
    )?;
    let signature_algorithm =
        decode_x509_algorithm_identifier(decoder, signature_algorithm_element).map_err(
            |error| {
                X509OcspBasicResponseError::signature_algorithm(error, signature_algorithm_offset)
            },
        )?;

    let signature_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let signature_element = read_required(
        decoder,
        &mut fields,
        value_offset,
        signature_offset,
        X509OcspBasicResponseErrorKind::MissingSignature,
    )?;
    let signature = decode_bit_string(signature_element)
        .map_err(|error| X509OcspBasicResponseError::signature(error, signature_offset))?;

    let optional_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let optional = fields.read(decoder).map_err(|error| {
        X509OcspBasicResponseError::child_structure(error, value_offset, optional_offset)
    })?;
    let (certificates, certificate_count) = match optional {
        None => (std::array::from_fn(|_| None), 0),
        Some(wrapper) if has_exact_explicit_tag(wrapper, CERTIFICATES_TAG) => {
            let decoded = decode_certificates(decoder, wrapper, optional_offset)?;
            let trailing_offset =
                child_offset(value_offset, value_length, fields.remaining().len());
            if fields
                .read(decoder)
                .map_err(|error| {
                    X509OcspBasicResponseError::child_structure(
                        error,
                        value_offset,
                        trailing_offset,
                    )
                })?
                .is_some()
            {
                return Err(X509OcspBasicResponseError::new(
                    X509OcspBasicResponseErrorKind::TrailingElement,
                    trailing_offset,
                ));
            }
            decoded
        }
        Some(_) => {
            return Err(X509OcspBasicResponseError::new(
                X509OcspBasicResponseErrorKind::UnexpectedOptionalField,
                optional_offset,
            ));
        }
    };

    Ok(X509OcspBasicResponse {
        encoded,
        response_data,
        signature_algorithm,
        signature,
        certificates,
        certificate_count,
    })
}

fn decode_certificates<'a>(
    decoder: &mut Asn1Decoder,
    wrapper: Asn1Element<'a>,
    wrapper_offset: usize,
) -> Result<
    (
        [Option<X509Certificate<'a>>; MAX_X509_OCSP_CERTIFICATES],
        usize,
    ),
    X509OcspBasicResponseError,
> {
    let sequence_offset = wrapper_offset + wrapper.header().len();
    let sequence = decoder
        .explicit(wrapper, CERTIFICATES_TAG)
        .map_err(|error| {
            X509OcspBasicResponseError::certificates_wrapper(error, wrapper_offset, sequence_offset)
        })?;
    let value_offset = sequence_offset + sequence.header().len();
    let value_length = sequence.value().len();
    let mut children = decoder
        .sequence(sequence)
        .map_err(|error| X509OcspBasicResponseError::certificates(error, sequence_offset))?;
    let mut certificates = std::array::from_fn(|_| None);
    let mut count = 0usize;

    while !children.remaining().is_empty() {
        let child_offset = child_offset(value_offset, value_length, children.remaining().len());
        let child = children.read(decoder).map_err(|error| {
            X509OcspBasicResponseError::child_structure(error, value_offset, child_offset)
        })?;
        let child = child.expect("a non-empty DER cursor must yield one element");
        if count == MAX_X509_OCSP_CERTIFICATES {
            return Err(X509OcspBasicResponseError::new(
                X509OcspBasicResponseErrorKind::TooManyCertificates,
                child_offset,
            ));
        }
        certificates[count] = Some(
            decode_x509_certificate_element(decoder, child)
                .map_err(|error| X509OcspBasicResponseError::certificate(error, child_offset))?,
        );
        count += 1;
    }

    Ok((certificates, count))
}

fn read_required<'a>(
    decoder: &mut Asn1Decoder,
    fields: &mut der_asn1::Asn1Cursor<'a>,
    value_offset: usize,
    field_offset: usize,
    missing: X509OcspBasicResponseErrorKind,
) -> Result<Asn1Element<'a>, X509OcspBasicResponseError> {
    fields
        .read(decoder)
        .map_err(|error| {
            X509OcspBasicResponseError::child_structure(error, value_offset, field_offset)
        })?
        .ok_or_else(|| X509OcspBasicResponseError::new(missing, field_offset))
}

fn has_exact_explicit_tag(element: Asn1Element<'_>, number: u32) -> bool {
    let tag = element.tag();
    tag.class == TagClass::ContextSpecific && tag.constructed && tag.number == number
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
