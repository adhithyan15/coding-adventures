//! # RFC 6960 OCSP response envelopes
//!
//! An OCSP transport returns one outer `OCSPResponse` value. That value first
//! reports whether the responder processed the request and, only for success,
//! carries an explicitly tagged `ResponseBytes` value:
//!
//! ```text
//! OCSPResponse ::= SEQUENCE {
//!     responseStatus  OCSPResponseStatus,
//!     responseBytes   [0] EXPLICIT ResponseBytes OPTIONAL }
//!
//! ResponseBytes ::= SEQUENCE {
//!     responseType    OBJECT IDENTIFIER,
//!     response        OCTET STRING }
//! ```
//!
//! This crate validates that envelope without assigning meaning to the
//! response-type OID or decoding the OCTET STRING. Keeping dispatch outside
//! this layer prevents a syntactically valid response type from silently
//! acquiring signature, responder, revocation, or transport authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_enumerated, decode_object_identifier, decode_octet_string, Asn1Decoder, Asn1Element,
    Asn1Error, Asn1ErrorKind, ObjectIdentifier,
};
use der_tlv::TagClass;
use std::error::Error;
use std::fmt;

const RESPONSE_BYTES_TAG: u32 = 0;

/// One assigned RFC 6960 `OCSPResponseStatus` value.
///
/// Four is deliberately absent from the registry. Numeric access avoids
/// making Rust's enum representation part of the wire contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509OcspResponseStatus {
    Successful,
    MalformedRequest,
    InternalError,
    TryLater,
    SigRequired,
    Unauthorized,
}

impl X509OcspResponseStatus {
    /// Return the standards-assigned ENUMERATED value.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Successful => 0,
            Self::MalformedRequest => 1,
            Self::InternalError => 2,
            Self::TryLater => 3,
            Self::SigRequired => 5,
            Self::Unauthorized => 6,
        }
    }

    const fn from_u64(value: u64) -> Option<Self> {
        match value {
            0 => Some(Self::Successful),
            1 => Some(Self::MalformedRequest),
            2 => Some(Self::InternalError),
            3 => Some(Self::TryLater),
            5 => Some(Self::SigRequired),
            6 => Some(Self::Unauthorized),
            _ => None,
        }
    }
}

/// One generic, structurally validated RFC 6960 `ResponseBytes` value.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct X509OcspResponseBytes<'a> {
    encoded: &'a [u8],
    response_type: ObjectIdentifier<'a>,
    response: &'a [u8],
}

impl<'a> X509OcspResponseBytes<'a> {
    /// Return the exact canonical `ResponseBytes` SEQUENCE encoding.
    pub const fn encoded(self) -> &'a [u8] {
        self.encoded
    }

    /// Return the generic response-type object identifier.
    ///
    /// Structural decoding does not recognize or authorize this identifier.
    pub const fn response_type(self) -> ObjectIdentifier<'a> {
        self.response_type
    }

    /// Return the borrowed contents of the response OCTET STRING.
    ///
    /// Structural decoding does not parse or authenticate these bytes.
    pub const fn response(self) -> &'a [u8] {
        self.response
    }
}

impl fmt::Debug for X509OcspResponseBytes<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("X509OcspResponseBytes")
            .field("response_type_arc_count", &self.response_type.arc_count())
            .field("response_octet_count", &self.response.len())
            .finish()
    }
}

/// One structurally valid RFC 6960 `OCSPResponse` envelope.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct X509OcspResponse<'a> {
    encoded: &'a [u8],
    status: X509OcspResponseStatus,
    response_bytes: Option<X509OcspResponseBytes<'a>>,
}

impl<'a> X509OcspResponse<'a> {
    /// Return the exact canonical complete response encoding.
    pub const fn encoded(self) -> &'a [u8] {
        self.encoded
    }

    /// Return the assigned response status.
    pub const fn status(self) -> X509OcspResponseStatus {
        self.status
    }

    /// Return the generic response bytes carried by a successful response.
    pub const fn response_bytes(self) -> Option<X509OcspResponseBytes<'a>> {
        self.response_bytes
    }
}

impl fmt::Debug for X509OcspResponse<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("X509OcspResponse")
            .field("encoded_octet_count", &self.encoded.len())
            .field("status", &self.status)
            .field(
                "response_type_arc_count",
                &self
                    .response_bytes
                    .map(|response| response.response_type.arc_count()),
            )
            .field(
                "response_octet_count",
                &self.response_bytes.map(|response| response.response.len()),
            )
            .finish()
    }
}

/// Stable failure categories. No category retains response payload bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509OcspResponseErrorKind {
    Structure(Asn1ErrorKind),
    MissingStatus,
    InvalidStatus(Asn1ErrorKind),
    UnsupportedStatus,
    MissingResponseBytes,
    UnexpectedResponseBytes,
    UnexpectedOptionalField,
    InvalidResponseBytesWrapper(Asn1ErrorKind),
    InvalidResponseBytes(Asn1ErrorKind),
    MissingResponseType,
    InvalidResponseType(Asn1ErrorKind),
    MissingResponse,
    InvalidResponse(Asn1ErrorKind),
    TrailingResponseBytesElement,
    TrailingElement,
}

/// A redacted failure at a byte offset local to the complete response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509OcspResponseError {
    kind: X509OcspResponseErrorKind,
    offset: usize,
}

impl X509OcspResponseError {
    const fn new(kind: X509OcspResponseErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509OcspResponseErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(X509OcspResponseErrorKind::Structure(error.kind()), offset)
    }

    fn status(error: Asn1Error, status_offset: usize) -> Self {
        Self::new(
            X509OcspResponseErrorKind::InvalidStatus(error.kind()),
            status_offset + error.offset(),
        )
    }

    fn response_bytes_wrapper(
        error: Asn1Error,
        wrapper_offset: usize,
        wrapper_value_offset: usize,
    ) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => wrapper_value_offset + error.offset(),
            _ => wrapper_offset + error.offset(),
        };
        Self::new(
            X509OcspResponseErrorKind::InvalidResponseBytesWrapper(error.kind()),
            offset,
        )
    }

    fn response_bytes(error: Asn1Error, sequence_offset: usize) -> Self {
        Self::new(
            X509OcspResponseErrorKind::InvalidResponseBytes(error.kind()),
            sequence_offset + error.offset(),
        )
    }

    fn response_bytes_child(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(
            X509OcspResponseErrorKind::InvalidResponseBytes(error.kind()),
            offset,
        )
    }

    fn response_type(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509OcspResponseErrorKind::InvalidResponseType(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn response(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509OcspResponseErrorKind::InvalidResponse(error.kind()),
            child_offset + error.offset(),
        )
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspResponseErrorKind {
        self.kind
    }

    /// Return the failing byte offset relative to the complete response.
    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509OcspResponseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP response error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509OcspResponseError {}

/// Decode one exact RFC 6960 `OCSPResponse` SEQUENCE.
///
/// The supplied decoder owns every shared DER budget. A successful decode
/// proves only the outer syntax and status/body relationship; response-type
/// dispatch, inner decoding, authentication, responder authorization,
/// freshness, and revocation policy remain caller-owned boundaries.
pub fn decode_x509_ocsp_response<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509OcspResponse<'a>, X509OcspResponseError> {
    let encoded = element.encoded();
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509OcspResponseError::structure)?;

    let status_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let status_element = read_required(
        decoder,
        &mut fields,
        value_offset,
        status_offset,
        X509OcspResponseErrorKind::MissingStatus,
    )?;
    let enumerated = decode_enumerated(status_element)
        .map_err(|error| X509OcspResponseError::status(error, status_offset))?;
    let status_value = enumerated
        .to_u64()
        .map_err(|error| X509OcspResponseError::status(error, status_offset))?;
    let status = X509OcspResponseStatus::from_u64(status_value).ok_or_else(|| {
        X509OcspResponseError::new(
            X509OcspResponseErrorKind::UnsupportedStatus,
            status_offset + status_element.header().len(),
        )
    })?;

    let optional_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let optional = fields.read(decoder).map_err(|error| {
        X509OcspResponseError::child_structure(error, value_offset, optional_offset)
    })?;

    let response_bytes = match (status, optional) {
        (X509OcspResponseStatus::Successful, None) => {
            return Err(X509OcspResponseError::new(
                X509OcspResponseErrorKind::MissingResponseBytes,
                optional_offset,
            ));
        }
        (X509OcspResponseStatus::Successful, Some(wrapper))
            if has_exact_explicit_tag(wrapper, RESPONSE_BYTES_TAG) =>
        {
            Some(decode_response_bytes(decoder, wrapper, optional_offset)?)
        }
        (X509OcspResponseStatus::Successful, Some(_)) => {
            return Err(X509OcspResponseError::new(
                X509OcspResponseErrorKind::UnexpectedOptionalField,
                optional_offset,
            ));
        }
        (_, Some(wrapper)) if has_exact_explicit_tag(wrapper, RESPONSE_BYTES_TAG) => {
            return Err(X509OcspResponseError::new(
                X509OcspResponseErrorKind::UnexpectedResponseBytes,
                optional_offset,
            ));
        }
        (_, Some(_)) => {
            return Err(X509OcspResponseError::new(
                X509OcspResponseErrorKind::UnexpectedOptionalField,
                optional_offset,
            ));
        }
        (_, None) => None,
    };

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            X509OcspResponseError::child_structure(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(X509OcspResponseError::new(
            X509OcspResponseErrorKind::TrailingElement,
            trailing_offset,
        ));
    }

    Ok(X509OcspResponse {
        encoded,
        status,
        response_bytes,
    })
}

fn decode_response_bytes<'a>(
    decoder: &mut Asn1Decoder,
    wrapper: Asn1Element<'a>,
    wrapper_offset: usize,
) -> Result<X509OcspResponseBytes<'a>, X509OcspResponseError> {
    let sequence_offset = wrapper_offset + wrapper.header().len();
    let sequence = decoder
        .explicit(wrapper, RESPONSE_BYTES_TAG)
        .map_err(|error| {
            X509OcspResponseError::response_bytes_wrapper(error, wrapper_offset, sequence_offset)
        })?;
    let encoded = sequence.encoded();
    let value_offset = sequence_offset + sequence.header().len();
    let value_length = sequence.value().len();
    let mut fields = decoder
        .sequence(sequence)
        .map_err(|error| X509OcspResponseError::response_bytes(error, sequence_offset))?;

    let response_type_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let response_type_element = read_response_bytes_required(
        decoder,
        &mut fields,
        value_offset,
        response_type_offset,
        X509OcspResponseErrorKind::MissingResponseType,
    )?;
    let response_type = decode_object_identifier(response_type_element, decoder.limits())
        .map_err(|error| X509OcspResponseError::response_type(error, response_type_offset))?;

    let response_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let response_element = read_response_bytes_required(
        decoder,
        &mut fields,
        value_offset,
        response_offset,
        X509OcspResponseErrorKind::MissingResponse,
    )?;
    let response = decode_octet_string(response_element)
        .map_err(|error| X509OcspResponseError::response(error, response_offset))?;

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            X509OcspResponseError::response_bytes_child(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(X509OcspResponseError::new(
            X509OcspResponseErrorKind::TrailingResponseBytesElement,
            trailing_offset,
        ));
    }

    Ok(X509OcspResponseBytes {
        encoded,
        response_type,
        response,
    })
}

fn read_response_bytes_required<'a>(
    decoder: &mut Asn1Decoder,
    fields: &mut der_asn1::Asn1Cursor<'a>,
    value_offset: usize,
    field_offset: usize,
    missing: X509OcspResponseErrorKind,
) -> Result<Asn1Element<'a>, X509OcspResponseError> {
    fields
        .read(decoder)
        .map_err(|error| {
            X509OcspResponseError::response_bytes_child(error, value_offset, field_offset)
        })?
        .ok_or_else(|| X509OcspResponseError::new(missing, field_offset))
}

fn read_required<'a>(
    decoder: &mut Asn1Decoder,
    fields: &mut der_asn1::Asn1Cursor<'a>,
    value_offset: usize,
    field_offset: usize,
    missing: X509OcspResponseErrorKind,
) -> Result<Asn1Element<'a>, X509OcspResponseError> {
    fields
        .read(decoder)
        .map_err(|error| X509OcspResponseError::child_structure(error, value_offset, field_offset))?
        .ok_or_else(|| X509OcspResponseError::new(missing, field_offset))
}

fn has_exact_explicit_tag(element: Asn1Element<'_>, number: u32) -> bool {
    let tag = element.tag();
    tag.class == TagClass::ContextSpecific && tag.constructed && tag.number == number
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
