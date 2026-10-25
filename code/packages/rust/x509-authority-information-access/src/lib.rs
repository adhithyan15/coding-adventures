//! # Bounded RFC 5280 Authority Information Access values
//!
//! This crate decodes one deliberately narrow AIA profile: arbitrary access
//! method OIDs paired with URI `GeneralName` locations. It grants no URI,
//! revocation, certificate-path, trust-root, HTTP, TLS, or network authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_implicit_ia5_string, decode_object_identifier, Asn1Decoder, Asn1Element, Asn1Error,
    Asn1ErrorKind, ObjectIdentifier,
};
use std::error::Error;
use std::fmt;
use x509_extension::X509Extension;

/// RFC 5280 `id-pe-authorityInfoAccess` (`1.3.6.1.5.5.7.1.1`).
pub const AUTHORITY_INFORMATION_ACCESS_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 1, 1];

/// RFC 5280 `id-ad-ocsp` (`1.3.6.1.5.5.7.48.1`).
pub const ID_AD_OCSP_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 48, 1];

/// RFC 5280 `id-ad-caIssuers` (`1.3.6.1.5.5.7.48.2`).
pub const ID_AD_CA_ISSUERS_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 48, 2];

/// Fixed stack bound for one extension's retained access descriptions.
pub const MAX_ACCESS_DESCRIPTIONS: usize = 32;

const URI_GENERAL_NAME_TAG: u32 = 6;

/// Stable failure categories. No category retains OID arcs, URI text, or bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityInformationAccessErrorKind {
    WrongExtensionId,
    CriticalExtension,
    Structure(Asn1ErrorKind),
    Empty,
    TooManyAccessDescriptions,
    MissingAccessMethod,
    InvalidAccessMethod(Asn1ErrorKind),
    MissingAccessLocation,
    UnsupportedAccessLocation,
    InvalidUri(Asn1ErrorKind),
    EmptyUri,
    TrailingAccessDescriptionField,
}

/// A redacted failure at an offset local to the encapsulated AIA value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthorityInformationAccessError {
    kind: AuthorityInformationAccessErrorKind,
    offset: usize,
}

impl AuthorityInformationAccessError {
    const fn new(kind: AuthorityInformationAccessErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            AuthorityInformationAccessErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn nested_structure(error: Asn1Error, element_offset: usize) -> Self {
        Self::new(
            AuthorityInformationAccessErrorKind::Structure(error.kind()),
            element_offset + error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(
            AuthorityInformationAccessErrorKind::Structure(error.kind()),
            offset,
        )
    }

    fn access_method(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            AuthorityInformationAccessErrorKind::InvalidAccessMethod(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn uri(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            AuthorityInformationAccessErrorKind::InvalidUri(error.kind()),
            child_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> AuthorityInformationAccessErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for AuthorityInformationAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 authority information access error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for AuthorityInformationAccessError {}

/// One borrowed and structurally validated RFC 5280 `AccessDescription`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessDescription<'a> {
    access_method: ObjectIdentifier<'a>,
    uri: &'a str,
}

impl<'a> AccessDescription<'a> {
    pub const fn access_method(self) -> ObjectIdentifier<'a> {
        self.access_method
    }

    pub const fn uri(self) -> &'a str {
        self.uri
    }
}

/// One bounded, ordered AIA collection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityInformationAccess<'a> {
    descriptions: [Option<AccessDescription<'a>>; MAX_ACCESS_DESCRIPTIONS],
    len: usize,
}

impl<'a> AuthorityInformationAccess<'a> {
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: usize) -> Option<AccessDescription<'a>> {
        self.descriptions.get(index).copied().flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item = AccessDescription<'a>> + '_ {
        self.descriptions[..self.len].iter().copied().flatten()
    }

    /// Iterate URI locations whose method equals one caller-selected OID.
    pub fn locations_for<'method>(
        &'method self,
        method: &'method [u64],
    ) -> impl Iterator<Item = &'a str> + 'method {
        self.iter()
            .filter(move |description| description.access_method().equals(method))
            .map(AccessDescription::uri)
    }
}

/// Decode one generic extension as the supported RFC 5280 AIA profile.
///
/// RFC 5280 requires conforming CAs to mark AIA non-critical. This decoder
/// enforces that outer policy and accepts only URI `GeneralName` locations;
/// other location forms remain unsupported and fail closed.
pub fn decode_authority_information_access<'a>(
    decoder: &mut Asn1Decoder,
    extension: X509Extension<'a>,
) -> Result<AuthorityInformationAccess<'a>, AuthorityInformationAccessError> {
    if !extension
        .extension_id()
        .equals(AUTHORITY_INFORMATION_ACCESS_OID)
    {
        return Err(AuthorityInformationAccessError::new(
            AuthorityInformationAccessErrorKind::WrongExtensionId,
            0,
        ));
    }
    if extension.critical() {
        return Err(AuthorityInformationAccessError::new(
            AuthorityInformationAccessErrorKind::CriticalExtension,
            0,
        ));
    }

    let root = decoder
        .decode_exact_embedded_context(
            extension.extension_value_context(),
            extension.extension_value(),
        )
        .map_err(AuthorityInformationAccessError::structure)?;
    let value_offset = root.header().len();
    let value_length = root.value().len();
    let mut children = decoder
        .sequence(root)
        .map_err(AuthorityInformationAccessError::structure)?;
    let mut descriptions = [None; MAX_ACCESS_DESCRIPTIONS];
    let mut len = 0usize;

    while !children.remaining().is_empty() {
        let description_offset =
            child_offset(value_offset, value_length, children.remaining().len());
        if len == MAX_ACCESS_DESCRIPTIONS {
            return Err(AuthorityInformationAccessError::new(
                AuthorityInformationAccessErrorKind::TooManyAccessDescriptions,
                description_offset,
            ));
        }

        let element = children
            .read(decoder)
            .map_err(|error| {
                AuthorityInformationAccessError::child_structure(
                    error,
                    value_offset,
                    description_offset,
                )
            })?
            .expect("a non-empty DER cursor must yield one element");
        descriptions[len] = Some(decode_access_description(
            decoder,
            element,
            description_offset,
        )?);
        len += 1;
    }

    if len == 0 {
        return Err(AuthorityInformationAccessError::new(
            AuthorityInformationAccessErrorKind::Empty,
            value_offset,
        ));
    }
    children
        .finish()
        .map_err(AuthorityInformationAccessError::structure)?;
    Ok(AuthorityInformationAccess { descriptions, len })
}

fn decode_access_description<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
    element_offset: usize,
) -> Result<AccessDescription<'a>, AuthorityInformationAccessError> {
    let relative_value_offset = element.header().len();
    let value_offset = element_offset + relative_value_offset;
    let value_length = element.value().len();
    let mut fields = decoder.sequence(element).map_err(|error| {
        AuthorityInformationAccessError::nested_structure(error, element_offset)
    })?;

    let method_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let method_element = fields
        .read(decoder)
        .map_err(|error| {
            AuthorityInformationAccessError::child_structure(error, value_offset, method_offset)
        })?
        .ok_or_else(|| {
            AuthorityInformationAccessError::new(
                AuthorityInformationAccessErrorKind::MissingAccessMethod,
                method_offset,
            )
        })?;
    let access_method = decode_object_identifier(method_element, decoder.limits())
        .map_err(|error| AuthorityInformationAccessError::access_method(error, method_offset))?;

    let location_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let location_element = fields
        .read(decoder)
        .map_err(|error| {
            AuthorityInformationAccessError::child_structure(error, value_offset, location_offset)
        })?
        .ok_or_else(|| {
            AuthorityInformationAccessError::new(
                AuthorityInformationAccessErrorKind::MissingAccessLocation,
                location_offset,
            )
        })?;
    if location_element.tag().number != URI_GENERAL_NAME_TAG {
        return Err(AuthorityInformationAccessError::new(
            AuthorityInformationAccessErrorKind::UnsupportedAccessLocation,
            location_offset,
        ));
    }
    let uri = decode_implicit_ia5_string(location_element, URI_GENERAL_NAME_TAG)
        .map_err(|error| AuthorityInformationAccessError::uri(error, location_offset))?;
    if uri.is_empty() {
        return Err(AuthorityInformationAccessError::new(
            AuthorityInformationAccessErrorKind::EmptyUri,
            location_offset + location_element.header().len(),
        ));
    }

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            AuthorityInformationAccessError::child_structure(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(AuthorityInformationAccessError::new(
            AuthorityInformationAccessErrorKind::TrailingAccessDescriptionField,
            trailing_offset,
        ));
    }
    Ok(AccessDescription { access_method, uri })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
