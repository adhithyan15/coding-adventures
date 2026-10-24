//! # RFC 6960 OCSP response data
//!
//! This crate decodes the bounded ASN.1 `ResponseData` container: the omitted
//! default v1 version, one responder identifier, `producedAt`, a non-empty
//! ordered sequence of `SingleResponse` values, and optional generic response
//! extensions. It deliberately performs no response authentication, responder
//! authorization, certificate binding, freshness or revocation decision, or
//! transport.

#![deny(unsafe_code)]

use der_asn1::{
    decode_integer, decode_octet_string, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind,
};
use der_tlv::TagClass;
use std::error::Error;
use std::fmt;
use x509_extensions::{
    decode_x509_extensions, X509Extensions, X509ExtensionsError, X509ExtensionsErrorKind,
};
use x509_name::{decode_x509_name, X509Name, X509NameError, X509NameErrorKind};
use x509_ocsp_single_response::{
    decode_x509_ocsp_single_response, X509OcspSingleResponse, X509OcspSingleResponseError,
    X509OcspSingleResponseErrorKind,
};
use x509_time::{decode_x509_generalized_time, X509Time, X509TimeError, X509TimeErrorKind};

const VERSION_TAG: u32 = 0;
const RESPONDER_BY_NAME_TAG: u32 = 1;
const RESPONDER_BY_KEY_TAG: u32 = 2;
const RESPONSE_EXTENSIONS_TAG: u32 = 1;

/// Maximum number of `SingleResponse` values retained from one response.
///
/// Each response already owns bounded nested extension storage, so this outer
/// ceiling also bounds the composed value's stack footprint.
pub const MAX_X509_OCSP_SINGLE_RESPONSES: usize = 16;

/// The only version defined by RFC 6960.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum X509OcspResponseDataVersion {
    /// Version 1, represented in DER by omitting the defaulted version field.
    #[default]
    V1,
}

impl X509OcspResponseDataVersion {
    /// Return the ASN.1 INTEGER assigned by RFC 6960.
    pub const fn as_u8(self) -> u8 {
        0
    }
}

/// One structurally decoded RFC 6960 responder identifier.
// Retaining the fully validated allocation-free Name makes this variant much
// larger than the borrowed key-hash slice. Boxing would violate this crate's
// no-allocation boundary, so the size difference is deliberate and bounded.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, PartialEq, Eq)]
pub enum X509OcspResponderId<'a> {
    /// An explicitly tagged RFC 5280 Name. No name semantics are implied.
    ByName(X509Name<'a>),
    /// An explicitly tagged OCTET STRING. No hash semantics are implied.
    ByKey(&'a [u8]),
}

impl fmt::Debug for X509OcspResponderId<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ByName(name) => formatter
                .debug_struct("ByName")
                .field("rdn_count", &name.rdn_count())
                .field("attribute_count", &name.attribute_count())
                .finish(),
            Self::ByKey(value) => formatter
                .debug_struct("ByKey")
                .field("octet_count", &value.len())
                .finish(),
        }
    }
}

/// Stable failure categories. No category retains response bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509OcspResponseDataErrorKind {
    Structure(Asn1ErrorKind),
    InvalidVersionWrapper(Asn1ErrorKind),
    InvalidVersionInteger(Asn1ErrorKind),
    ExplicitDefaultVersion,
    UnsupportedVersion,
    MissingResponderId,
    InvalidResponderIdWrapper(Asn1ErrorKind),
    InvalidResponderName(X509NameErrorKind),
    InvalidResponderKeyHash(Asn1ErrorKind),
    UnexpectedResponderId,
    MissingProducedAt,
    InvalidProducedAt(X509TimeErrorKind),
    MissingResponses,
    InvalidResponses(Asn1ErrorKind),
    EmptyResponses,
    TooManyResponses,
    InvalidSingleResponse(X509OcspSingleResponseErrorKind),
    InvalidResponseExtensionsWrapper(Asn1ErrorKind),
    InvalidResponseExtensions(X509ExtensionsErrorKind),
    UnexpectedOptionalField,
    TrailingElement,
}

/// A redacted failure at a byte offset local to the `ResponseData` element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509OcspResponseDataError {
    kind: X509OcspResponseDataErrorKind,
    offset: usize,
}

impl X509OcspResponseDataError {
    const fn new(kind: X509OcspResponseDataErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509OcspResponseDataErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(
            X509OcspResponseDataErrorKind::Structure(error.kind()),
            offset,
        )
    }

    fn version_wrapper(error: Asn1Error, wrapper_offset: usize, value_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => wrapper_offset + error.offset(),
        };
        Self::new(
            X509OcspResponseDataErrorKind::InvalidVersionWrapper(error.kind()),
            offset,
        )
    }

    fn version_integer(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509OcspResponseDataErrorKind::InvalidVersionInteger(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn responder_wrapper(error: Asn1Error, wrapper_offset: usize, value_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => wrapper_offset + error.offset(),
        };
        Self::new(
            X509OcspResponseDataErrorKind::InvalidResponderIdWrapper(error.kind()),
            offset,
        )
    }

    fn responder_name(error: X509NameError, child_offset: usize) -> Self {
        Self::new(
            X509OcspResponseDataErrorKind::InvalidResponderName(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn responder_key_hash(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509OcspResponseDataErrorKind::InvalidResponderKeyHash(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn produced_at(error: X509TimeError, child_offset: usize) -> Self {
        Self::new(
            X509OcspResponseDataErrorKind::InvalidProducedAt(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn responses(error: Asn1Error, responses_offset: usize) -> Self {
        Self::new(
            X509OcspResponseDataErrorKind::InvalidResponses(error.kind()),
            responses_offset + error.offset(),
        )
    }

    fn single_response(error: X509OcspSingleResponseError, child_offset: usize) -> Self {
        Self::new(
            X509OcspResponseDataErrorKind::InvalidSingleResponse(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn response_extensions_wrapper(
        error: Asn1Error,
        wrapper_offset: usize,
        value_offset: usize,
    ) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => wrapper_offset + error.offset(),
        };
        Self::new(
            X509OcspResponseDataErrorKind::InvalidResponseExtensionsWrapper(error.kind()),
            offset,
        )
    }

    fn response_extensions(error: X509ExtensionsError, child_offset: usize) -> Self {
        Self::new(
            X509OcspResponseDataErrorKind::InvalidResponseExtensions(error.kind()),
            child_offset + error.offset(),
        )
    }

    /// Return the stable, payload-free failure category.
    pub const fn kind(self) -> X509OcspResponseDataErrorKind {
        self.kind
    }

    /// Return the failing byte offset relative to the `ResponseData` element.
    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509OcspResponseDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP response data error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509OcspResponseDataError {}

/// One structurally valid RFC 6960 `ResponseData` value.
#[derive(Clone, PartialEq, Eq)]
pub struct X509OcspResponseData<'a> {
    version: X509OcspResponseDataVersion,
    responder_id: X509OcspResponderId<'a>,
    produced_at: X509Time,
    responses: [Option<X509OcspSingleResponse<'a>>; MAX_X509_OCSP_SINGLE_RESPONSES],
    response_count: usize,
    response_extensions: Option<X509Extensions<'a>>,
}

impl<'a> X509OcspResponseData<'a> {
    /// Return the omitted-default RFC 6960 version.
    pub const fn version(&self) -> X509OcspResponseDataVersion {
        self.version
    }

    /// Return the structural responder identifier. This proves no authority.
    pub const fn responder_id(&self) -> &X509OcspResponderId<'a> {
        &self.responder_id
    }

    /// Return the clock-free `producedAt` value.
    pub const fn produced_at(&self) -> X509Time {
        self.produced_at
    }

    /// Return the number of retained `SingleResponse` values.
    pub const fn len(&self) -> usize {
        self.response_count
    }

    /// Return whether no responses were retained. Valid values are never empty.
    pub const fn is_empty(&self) -> bool {
        self.response_count == 0
    }

    /// Return one response by wire-order index.
    pub fn get(&self, index: usize) -> Option<&X509OcspSingleResponse<'a>> {
        self.responses.get(index).and_then(Option::as_ref)
    }

    /// Iterate over responses in wire order.
    pub fn iter(&self) -> impl Iterator<Item = &X509OcspSingleResponse<'a>> {
        self.responses[..self.response_count]
            .iter()
            .filter_map(Option::as_ref)
    }

    /// Return generic response extensions in wire order.
    pub const fn response_extensions(&self) -> Option<&X509Extensions<'a>> {
        self.response_extensions.as_ref()
    }
}

impl fmt::Debug for X509OcspResponseData<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("X509OcspResponseData")
            .field("version", &self.version)
            .field("responder_id", &self.responder_id)
            .field("produced_at", &self.produced_at)
            .field("response_count", &self.response_count)
            .field(
                "response_extensions_len",
                &self.response_extensions.as_ref().map(X509Extensions::len),
            )
            .finish()
    }
}

/// Decode one exact RFC 6960 `ResponseData` SEQUENCE.
///
/// The supplied decoder owns every shared DER budget. Success proves syntax
/// only; callers must separately authenticate the response, authorize its
/// responder, bind certificate identifiers, interpret extensions, and apply
/// freshness and revocation policy.
pub fn decode_x509_ocsp_response_data<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509OcspResponseData<'a>, X509OcspResponseDataError> {
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509OcspResponseDataError::structure)?;

    let first_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let first = read_required(
        decoder,
        &mut fields,
        value_offset,
        first_offset,
        X509OcspResponseDataErrorKind::MissingResponderId,
    )?;

    let (version, responder_element, responder_offset) = if has_context_tag(first, VERSION_TAG) {
        let version = decode_version(decoder, first, first_offset)?;
        let responder_offset = child_offset(value_offset, value_length, fields.remaining().len());
        let responder = read_required(
            decoder,
            &mut fields,
            value_offset,
            responder_offset,
            X509OcspResponseDataErrorKind::MissingResponderId,
        )?;
        (version, responder, responder_offset)
    } else {
        (X509OcspResponseDataVersion::V1, first, first_offset)
    };

    let responder_id = decode_responder_id(decoder, responder_element, responder_offset)?;

    let produced_at_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let produced_at_element = read_required(
        decoder,
        &mut fields,
        value_offset,
        produced_at_offset,
        X509OcspResponseDataErrorKind::MissingProducedAt,
    )?;
    let produced_at = decode_x509_generalized_time(produced_at_element)
        .map_err(|error| X509OcspResponseDataError::produced_at(error, produced_at_offset))?;

    let responses_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let responses_element = read_required(
        decoder,
        &mut fields,
        value_offset,
        responses_offset,
        X509OcspResponseDataErrorKind::MissingResponses,
    )?;
    let (responses, response_count) =
        decode_responses(decoder, responses_element, responses_offset)?;

    let optional_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let optional = fields.read(decoder).map_err(|error| {
        X509OcspResponseDataError::child_structure(error, value_offset, optional_offset)
    })?;
    let response_extensions = match optional {
        None => None,
        Some(wrapper) if has_exact_explicit_tag(wrapper, RESPONSE_EXTENSIONS_TAG) => {
            let extensions = decode_response_extensions(decoder, wrapper, optional_offset)?;
            let trailing_offset =
                child_offset(value_offset, value_length, fields.remaining().len());
            if fields
                .read(decoder)
                .map_err(|error| {
                    X509OcspResponseDataError::child_structure(error, value_offset, trailing_offset)
                })?
                .is_some()
            {
                return Err(X509OcspResponseDataError::new(
                    X509OcspResponseDataErrorKind::TrailingElement,
                    trailing_offset,
                ));
            }
            Some(extensions)
        }
        Some(_) => {
            return Err(X509OcspResponseDataError::new(
                X509OcspResponseDataErrorKind::UnexpectedOptionalField,
                optional_offset,
            ));
        }
    };

    Ok(X509OcspResponseData {
        version,
        responder_id,
        produced_at,
        responses,
        response_count,
        response_extensions,
    })
}

fn decode_version(
    decoder: &mut Asn1Decoder,
    wrapper: Asn1Element<'_>,
    wrapper_offset: usize,
) -> Result<X509OcspResponseDataVersion, X509OcspResponseDataError> {
    let child_offset = wrapper_offset + wrapper.header().len();
    let child = decoder.explicit(wrapper, VERSION_TAG).map_err(|error| {
        X509OcspResponseDataError::version_wrapper(error, wrapper_offset, child_offset)
    })?;
    let integer = decode_integer(child)
        .map_err(|error| X509OcspResponseDataError::version_integer(error, child_offset))?;
    let value_offset = child_offset + child.header().len();
    let value = integer
        .to_u64()
        .map_err(|error| X509OcspResponseDataError::version_integer(error, child_offset))?;
    match value {
        0 => Err(X509OcspResponseDataError::new(
            X509OcspResponseDataErrorKind::ExplicitDefaultVersion,
            value_offset,
        )),
        _ => Err(X509OcspResponseDataError::new(
            X509OcspResponseDataErrorKind::UnsupportedVersion,
            value_offset,
        )),
    }
}

fn decode_responder_id<'a>(
    decoder: &mut Asn1Decoder,
    wrapper: Asn1Element<'a>,
    wrapper_offset: usize,
) -> Result<X509OcspResponderId<'a>, X509OcspResponseDataError> {
    let tag = wrapper.tag();
    if tag.class != TagClass::ContextSpecific
        || (tag.number != RESPONDER_BY_NAME_TAG && tag.number != RESPONDER_BY_KEY_TAG)
    {
        return Err(X509OcspResponseDataError::new(
            X509OcspResponseDataErrorKind::UnexpectedResponderId,
            wrapper_offset,
        ));
    }

    let child_offset = wrapper_offset + wrapper.header().len();
    let child = decoder.explicit(wrapper, tag.number).map_err(|error| {
        X509OcspResponseDataError::responder_wrapper(error, wrapper_offset, child_offset)
    })?;
    match tag.number {
        RESPONDER_BY_NAME_TAG => decode_x509_name(decoder, child)
            .map(X509OcspResponderId::ByName)
            .map_err(|error| X509OcspResponseDataError::responder_name(error, child_offset)),
        RESPONDER_BY_KEY_TAG => decode_octet_string(child)
            .map(X509OcspResponderId::ByKey)
            .map_err(|error| X509OcspResponseDataError::responder_key_hash(error, child_offset)),
        _ => unreachable!("the responder tag was validated above"),
    }
}

fn decode_responses<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
    element_offset: usize,
) -> Result<
    (
        [Option<X509OcspSingleResponse<'a>>; MAX_X509_OCSP_SINGLE_RESPONSES],
        usize,
    ),
    X509OcspResponseDataError,
> {
    let value_offset = element_offset + element.header().len();
    let value_length = element.value().len();
    let mut children = decoder
        .sequence(element)
        .map_err(|error| X509OcspResponseDataError::responses(error, element_offset))?;
    let mut responses = std::array::from_fn(|_| None);
    let mut count = 0usize;

    while !children.remaining().is_empty() {
        let child_offset = child_offset(value_offset, value_length, children.remaining().len());
        let child = children.read(decoder).map_err(|error| {
            X509OcspResponseDataError::child_structure(error, value_offset, child_offset)
        })?;
        let child = child.expect("a non-empty DER cursor must yield one element");
        if count == MAX_X509_OCSP_SINGLE_RESPONSES {
            return Err(X509OcspResponseDataError::new(
                X509OcspResponseDataErrorKind::TooManyResponses,
                child_offset,
            ));
        }
        responses[count] = Some(
            decode_x509_ocsp_single_response(decoder, child)
                .map_err(|error| X509OcspResponseDataError::single_response(error, child_offset))?,
        );
        count += 1;
    }

    if count == 0 {
        return Err(X509OcspResponseDataError::new(
            X509OcspResponseDataErrorKind::EmptyResponses,
            value_offset,
        ));
    }

    Ok((responses, count))
}

fn decode_response_extensions<'a>(
    decoder: &mut Asn1Decoder,
    wrapper: Asn1Element<'a>,
    wrapper_offset: usize,
) -> Result<X509Extensions<'a>, X509OcspResponseDataError> {
    let child_offset = wrapper_offset + wrapper.header().len();
    let child = decoder
        .explicit(wrapper, RESPONSE_EXTENSIONS_TAG)
        .map_err(|error| {
            X509OcspResponseDataError::response_extensions_wrapper(
                error,
                wrapper_offset,
                child_offset,
            )
        })?;
    decode_x509_extensions(decoder, child)
        .map_err(|error| X509OcspResponseDataError::response_extensions(error, child_offset))
}

fn read_required<'a>(
    decoder: &mut Asn1Decoder,
    fields: &mut der_asn1::Asn1Cursor<'a>,
    value_offset: usize,
    field_offset: usize,
    missing: X509OcspResponseDataErrorKind,
) -> Result<Asn1Element<'a>, X509OcspResponseDataError> {
    fields
        .read(decoder)
        .map_err(|error| {
            X509OcspResponseDataError::child_structure(error, value_offset, field_offset)
        })?
        .ok_or_else(|| X509OcspResponseDataError::new(missing, field_offset))
}

fn has_context_tag(element: Asn1Element<'_>, number: u32) -> bool {
    let tag = element.tag();
    tag.class == TagClass::ContextSpecific && tag.number == number
}

fn has_exact_explicit_tag(element: Asn1Element<'_>, number: u32) -> bool {
    has_context_tag(element, number) && element.tag().constructed
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
