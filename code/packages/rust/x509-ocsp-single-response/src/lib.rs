//! # RFC 6960 OCSP single responses
//!
//! This crate decodes the bounded ASN.1 `SingleResponse` container: one
//! `CertID`, one `CertStatus`, one `thisUpdate`, and the ordered optional
//! `nextUpdate` and `singleExtensions` fields. It deliberately assigns no hash
//! policy or extension semantics and performs no certificate binding,
//! authentication, responder authorization, freshness decision, or transport.

#![deny(unsafe_code)]

use der_asn1::{Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind};
use der_tlv::TagClass;
use std::error::Error;
use std::fmt;
use x509_extensions::{
    decode_x509_extensions, X509Extensions, X509ExtensionsError, X509ExtensionsErrorKind,
};
use x509_ocsp_cert_id::{
    decode_x509_ocsp_cert_id, X509OcspCertId, X509OcspCertIdError, X509OcspCertIdErrorKind,
};
use x509_ocsp_cert_status::{
    decode_x509_ocsp_cert_status, X509OcspCertStatus, X509OcspCertStatusError,
    X509OcspCertStatusErrorKind,
};
use x509_time::{decode_x509_generalized_time, X509Time, X509TimeError, X509TimeErrorKind};

const NEXT_UPDATE_TAG: u32 = 0;
const SINGLE_EXTENSIONS_TAG: u32 = 1;

/// Stable failure categories. No category retains response bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509OcspSingleResponseErrorKind {
    Structure(Asn1ErrorKind),
    MissingCertId,
    InvalidCertId(X509OcspCertIdErrorKind),
    MissingCertStatus,
    InvalidCertStatus(X509OcspCertStatusErrorKind),
    MissingThisUpdate,
    InvalidThisUpdate(X509TimeErrorKind),
    InvalidNextUpdateWrapper(Asn1ErrorKind),
    InvalidNextUpdate(X509TimeErrorKind),
    InvalidSingleExtensionsWrapper(Asn1ErrorKind),
    InvalidSingleExtensions(X509ExtensionsErrorKind),
    UnexpectedOptionalField,
    TrailingElement,
}

/// A redacted failure at a byte offset local to the `SingleResponse` element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509OcspSingleResponseError {
    kind: X509OcspSingleResponseErrorKind,
    offset: usize,
}

impl X509OcspSingleResponseError {
    const fn new(kind: X509OcspSingleResponseErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509OcspSingleResponseErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(
            X509OcspSingleResponseErrorKind::Structure(error.kind()),
            offset,
        )
    }

    fn cert_id(error: X509OcspCertIdError, child_offset: usize) -> Self {
        Self::new(
            X509OcspSingleResponseErrorKind::InvalidCertId(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn cert_status(error: X509OcspCertStatusError, child_offset: usize) -> Self {
        Self::new(
            X509OcspSingleResponseErrorKind::InvalidCertStatus(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn this_update(error: X509TimeError, child_offset: usize) -> Self {
        Self::new(
            X509OcspSingleResponseErrorKind::InvalidThisUpdate(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn explicit_wrapper(
        error: Asn1Error,
        wrapper_offset: usize,
        wrapper_value_offset: usize,
        next_update: bool,
    ) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => wrapper_value_offset + error.offset(),
            _ => wrapper_offset + error.offset(),
        };
        let kind = if next_update {
            X509OcspSingleResponseErrorKind::InvalidNextUpdateWrapper(error.kind())
        } else {
            X509OcspSingleResponseErrorKind::InvalidSingleExtensionsWrapper(error.kind())
        };
        Self::new(kind, offset)
    }

    fn next_update(error: X509TimeError, child_offset: usize) -> Self {
        Self::new(
            X509OcspSingleResponseErrorKind::InvalidNextUpdate(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn single_extensions(error: X509ExtensionsError, child_offset: usize) -> Self {
        Self::new(
            X509OcspSingleResponseErrorKind::InvalidSingleExtensions(error.kind()),
            child_offset + error.offset(),
        )
    }

    /// Return the stable, payload-free failure category.
    pub const fn kind(self) -> X509OcspSingleResponseErrorKind {
        self.kind
    }

    /// Return the failing byte offset relative to the `SingleResponse` element.
    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509OcspSingleResponseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP single response error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509OcspSingleResponseError {}

/// One structurally valid RFC 6960 `SingleResponse`.
#[derive(Clone, PartialEq, Eq)]
pub struct X509OcspSingleResponse<'a> {
    cert_id: X509OcspCertId<'a>,
    cert_status: X509OcspCertStatus,
    this_update: X509Time,
    next_update: Option<X509Time>,
    single_extensions: Option<X509Extensions<'a>>,
}

impl<'a> X509OcspSingleResponse<'a> {
    /// Return the generic certificate identifier. This does not prove binding.
    pub const fn cert_id(&self) -> X509OcspCertId<'a> {
        self.cert_id
    }

    /// Return the structurally decoded status. This does not prove authority.
    pub const fn cert_status(&self) -> X509OcspCertStatus {
        self.cert_status
    }

    /// Return the clock-free `thisUpdate` value.
    pub const fn this_update(&self) -> X509Time {
        self.this_update
    }

    /// Return the optional clock-free `nextUpdate` value.
    pub const fn next_update(&self) -> Option<X509Time> {
        self.next_update
    }

    /// Return generic single-response extensions in wire order.
    pub const fn single_extensions(&self) -> Option<&X509Extensions<'a>> {
        self.single_extensions.as_ref()
    }
}

impl fmt::Debug for X509OcspSingleResponse<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("X509OcspSingleResponse")
            .field("cert_id", &"validated CertID")
            .field("cert_status", &self.cert_status)
            .field("this_update", &self.this_update)
            .field("next_update", &self.next_update)
            .field(
                "single_extensions_len",
                &self.single_extensions.as_ref().map(X509Extensions::len),
            )
            .finish()
    }
}

/// Decode one exact RFC 6960 `SingleResponse` SEQUENCE.
///
/// The supplied decoder owns all shared DER budgets. The result proves syntax
/// only; callers must separately authenticate and authorize the OCSP response,
/// bind the `CertID`, interpret extensions, and apply freshness and revocation
/// policy.
pub fn decode_x509_ocsp_single_response<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509OcspSingleResponse<'a>, X509OcspSingleResponseError> {
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509OcspSingleResponseError::structure)?;

    let cert_id_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let cert_id_element = read_required(
        decoder,
        &mut fields,
        value_offset,
        cert_id_offset,
        X509OcspSingleResponseErrorKind::MissingCertId,
    )?;
    let cert_id = decode_x509_ocsp_cert_id(decoder, cert_id_element)
        .map_err(|error| X509OcspSingleResponseError::cert_id(error, cert_id_offset))?;

    let cert_status_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let cert_status_element = read_required(
        decoder,
        &mut fields,
        value_offset,
        cert_status_offset,
        X509OcspSingleResponseErrorKind::MissingCertStatus,
    )?;
    let cert_status = decode_x509_ocsp_cert_status(decoder, cert_status_element)
        .map_err(|error| X509OcspSingleResponseError::cert_status(error, cert_status_offset))?;

    let this_update_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let this_update_element = read_required(
        decoder,
        &mut fields,
        value_offset,
        this_update_offset,
        X509OcspSingleResponseErrorKind::MissingThisUpdate,
    )?;
    let this_update = decode_x509_generalized_time(this_update_element)
        .map_err(|error| X509OcspSingleResponseError::this_update(error, this_update_offset))?;

    let first_optional_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let first_optional = fields.read(decoder).map_err(|error| {
        X509OcspSingleResponseError::child_structure(error, value_offset, first_optional_offset)
    })?;

    let (next_update, single_extensions) = match first_optional {
        None => (None, None),
        Some(wrapper) if has_explicit_tag(wrapper, NEXT_UPDATE_TAG) => {
            let next_update = decode_next_update(decoder, wrapper, first_optional_offset)?;
            let second_offset = child_offset(value_offset, value_length, fields.remaining().len());
            let second = fields.read(decoder).map_err(|error| {
                X509OcspSingleResponseError::child_structure(error, value_offset, second_offset)
            })?;
            match second {
                None => (Some(next_update), None),
                Some(wrapper) if has_explicit_tag(wrapper, SINGLE_EXTENSIONS_TAG) => {
                    let extensions = decode_single_extensions(decoder, wrapper, second_offset)?;
                    let trailing_offset =
                        child_offset(value_offset, value_length, fields.remaining().len());
                    if fields
                        .read(decoder)
                        .map_err(|error| {
                            X509OcspSingleResponseError::child_structure(
                                error,
                                value_offset,
                                trailing_offset,
                            )
                        })?
                        .is_some()
                    {
                        return Err(X509OcspSingleResponseError::new(
                            X509OcspSingleResponseErrorKind::TrailingElement,
                            trailing_offset,
                        ));
                    }
                    (Some(next_update), Some(extensions))
                }
                Some(_) => {
                    return Err(X509OcspSingleResponseError::new(
                        X509OcspSingleResponseErrorKind::UnexpectedOptionalField,
                        second_offset,
                    ));
                }
            }
        }
        Some(wrapper) if has_explicit_tag(wrapper, SINGLE_EXTENSIONS_TAG) => {
            let extensions = decode_single_extensions(decoder, wrapper, first_optional_offset)?;
            let second_offset = child_offset(value_offset, value_length, fields.remaining().len());
            if fields
                .read(decoder)
                .map_err(|error| {
                    X509OcspSingleResponseError::child_structure(error, value_offset, second_offset)
                })?
                .is_some()
            {
                return Err(X509OcspSingleResponseError::new(
                    X509OcspSingleResponseErrorKind::UnexpectedOptionalField,
                    second_offset,
                ));
            }
            (None, Some(extensions))
        }
        Some(_) => {
            return Err(X509OcspSingleResponseError::new(
                X509OcspSingleResponseErrorKind::UnexpectedOptionalField,
                first_optional_offset,
            ));
        }
    };

    Ok(X509OcspSingleResponse {
        cert_id,
        cert_status,
        this_update,
        next_update,
        single_extensions,
    })
}

fn read_required<'a>(
    decoder: &mut Asn1Decoder,
    fields: &mut der_asn1::Asn1Cursor<'a>,
    value_offset: usize,
    field_offset: usize,
    missing: X509OcspSingleResponseErrorKind,
) -> Result<Asn1Element<'a>, X509OcspSingleResponseError> {
    fields
        .read(decoder)
        .map_err(|error| {
            X509OcspSingleResponseError::child_structure(error, value_offset, field_offset)
        })?
        .ok_or_else(|| X509OcspSingleResponseError::new(missing, field_offset))
}

fn decode_next_update(
    decoder: &mut Asn1Decoder,
    wrapper: Asn1Element<'_>,
    wrapper_offset: usize,
) -> Result<X509Time, X509OcspSingleResponseError> {
    let child_offset = wrapper_offset + wrapper.header().len();
    let child = decoder
        .explicit(wrapper, NEXT_UPDATE_TAG)
        .map_err(|error| {
            X509OcspSingleResponseError::explicit_wrapper(error, wrapper_offset, child_offset, true)
        })?;
    decode_x509_generalized_time(child)
        .map_err(|error| X509OcspSingleResponseError::next_update(error, child_offset))
}

fn decode_single_extensions<'a>(
    decoder: &mut Asn1Decoder,
    wrapper: Asn1Element<'a>,
    wrapper_offset: usize,
) -> Result<X509Extensions<'a>, X509OcspSingleResponseError> {
    let child_offset = wrapper_offset + wrapper.header().len();
    let child = decoder
        .explicit(wrapper, SINGLE_EXTENSIONS_TAG)
        .map_err(|error| {
            X509OcspSingleResponseError::explicit_wrapper(
                error,
                wrapper_offset,
                child_offset,
                false,
            )
        })?;
    decode_x509_extensions(decoder, child)
        .map_err(|error| X509OcspSingleResponseError::single_extensions(error, child_offset))
}

fn has_explicit_tag(element: Asn1Element<'_>, number: u32) -> bool {
    let tag = element.tag();
    tag.class == TagClass::ContextSpecific && tag.constructed && tag.number == number
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
