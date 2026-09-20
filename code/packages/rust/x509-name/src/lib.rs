//! # Bounded RFC 5280 distinguished names
//!
//! This crate decodes only the structural `Name`, RDN, and attribute layers.
//! It deliberately owns no DirectoryString normalization, semantic name
//! comparison, certificate-path policy, or cryptographic authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_object_identifier, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind, ObjectIdentifier,
};
use std::error::Error;
use std::fmt;

/// Maximum number of relative distinguished names retained from one Name.
pub const MAX_X509_NAME_RDNS: usize = 32;
/// Maximum total number of attributes retained across all RDNs in one Name.
pub const MAX_X509_NAME_ATTRIBUTES: usize = 64;

/// Stable attribute failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509AttributeTypeAndValueErrorKind {
    Structure(Asn1ErrorKind),
    MissingType,
    InvalidType(Asn1ErrorKind),
    MissingValue,
    TrailingElement,
}

/// Stable Name failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509NameErrorKind {
    Structure(Asn1ErrorKind),
    TooManyRelativeDistinguishedNames,
    EmptyRelativeDistinguishedName,
    TooManyAttributes,
    UnsortedRelativeDistinguishedName,
    InvalidAttribute(X509AttributeTypeAndValueErrorKind),
}

/// A redacted failure at a byte offset local to the Name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509NameError {
    kind: X509NameErrorKind,
    offset: usize,
}

impl X509NameError {
    const fn new(kind: X509NameErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(X509NameErrorKind::Structure(error.kind()), error.offset())
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(X509NameErrorKind::Structure(error.kind()), offset)
    }

    fn rdn_child_structure(
        error: Asn1Error,
        rdn_offset: usize,
        value_offset: usize,
        child_offset: usize,
    ) -> Self {
        let local_offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(
            X509NameErrorKind::Structure(error.kind()),
            rdn_offset + local_offset,
        )
    }

    fn attribute(error: X509AttributeTypeAndValueError, attribute_offset: usize) -> Self {
        Self::new(
            X509NameErrorKind::InvalidAttribute(error.kind),
            attribute_offset + error.offset,
        )
    }

    pub const fn kind(self) -> X509NameErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509NameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 Name error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509NameError {}

/// One borrowed generic RFC 5280 `AttributeTypeAndValue`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509AttributeTypeAndValue<'a> {
    attribute_type: ObjectIdentifier<'a>,
    value: Asn1Element<'a>,
}

impl<'a> X509AttributeTypeAndValue<'a> {
    pub const fn attribute_type(self) -> ObjectIdentifier<'a> {
        self.attribute_type
    }

    /// Return the exact, still-typed DER attribute value.
    pub const fn value(self) -> Asn1Element<'a> {
        self.value
    }
}

/// A borrowed view of one non-empty RDN in a decoded Name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509RelativeDistinguishedName<'name, 'a> {
    attributes: &'name [Option<X509AttributeTypeAndValue<'a>>],
}

impl<'name, 'a> X509RelativeDistinguishedName<'name, 'a> {
    pub const fn len(self) -> usize {
        self.attributes.len()
    }

    pub const fn is_empty(self) -> bool {
        self.attributes.is_empty()
    }

    pub fn get(self, index: usize) -> Option<X509AttributeTypeAndValue<'a>> {
        self.attributes.get(index).copied().flatten()
    }

    pub fn iter(self) -> impl Iterator<Item = X509AttributeTypeAndValue<'a>> + 'name {
        self.attributes.iter().copied().flatten()
    }
}

/// One bounded, ordered, borrowed RFC 5280 Name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X509Name<'a> {
    encoded: &'a [u8],
    attributes: [Option<X509AttributeTypeAndValue<'a>>; MAX_X509_NAME_ATTRIBUTES],
    rdn_ends: [usize; MAX_X509_NAME_RDNS],
    rdn_count: usize,
    attribute_count: usize,
}

impl<'a> X509Name<'a> {
    /// Return the exact canonical DER encoding of the Name.
    pub const fn encoded(&self) -> &'a [u8] {
        self.encoded
    }

    pub const fn rdn_count(&self) -> usize {
        self.rdn_count
    }

    pub const fn attribute_count(&self) -> usize {
        self.attribute_count
    }

    pub const fn is_empty(&self) -> bool {
        self.rdn_count == 0
    }

    pub fn get_rdn(&self, index: usize) -> Option<X509RelativeDistinguishedName<'_, 'a>> {
        if index >= self.rdn_count {
            return None;
        }
        let start = if index == 0 {
            0
        } else {
            self.rdn_ends[index - 1]
        };
        let end = self.rdn_ends[index];
        Some(X509RelativeDistinguishedName {
            attributes: &self.attributes[start..end],
        })
    }

    pub fn rdns(&self) -> impl Iterator<Item = X509RelativeDistinguishedName<'_, 'a>> + '_ {
        (0..self.rdn_count).filter_map(|index| self.get_rdn(index))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct X509AttributeTypeAndValueError {
    kind: X509AttributeTypeAndValueErrorKind,
    offset: usize,
}

impl X509AttributeTypeAndValueError {
    const fn new(kind: X509AttributeTypeAndValueErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509AttributeTypeAndValueErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(
            X509AttributeTypeAndValueErrorKind::Structure(error.kind()),
            offset,
        )
    }
}

/// Decode one exact RFC 5280 `Name` RDN sequence.
pub fn decode_x509_name<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509Name<'a>, X509NameError> {
    let encoded = element.encoded();
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut rdns = decoder
        .sequence(element)
        .map_err(X509NameError::structure)?;
    let mut attributes: [Option<X509AttributeTypeAndValue<'a>>; MAX_X509_NAME_ATTRIBUTES] =
        [None; MAX_X509_NAME_ATTRIBUTES];
    let mut rdn_ends = [0usize; MAX_X509_NAME_RDNS];
    let mut rdn_count = 0usize;
    let mut attribute_count = 0usize;

    while !rdns.remaining().is_empty() {
        let rdn_offset = child_offset(value_offset, value_length, rdns.remaining().len());
        let rdn = rdns
            .read(decoder)
            .map_err(|error| X509NameError::child_structure(error, value_offset, rdn_offset))?
            .expect("a non-empty DER cursor must yield one element");

        if rdn_count == MAX_X509_NAME_RDNS {
            return Err(X509NameError::new(
                X509NameErrorKind::TooManyRelativeDistinguishedNames,
                rdn_offset,
            ));
        }

        let rdn_value_offset = rdn.header().len();
        let rdn_value_length = rdn.value().len();
        let mut members = decoder.set(rdn).map_err(|error| {
            X509NameError::new(
                X509NameErrorKind::Structure(error.kind()),
                rdn_offset + error.offset(),
            )
        })?;
        let first_attribute = attribute_count;
        let mut previous_encoding: Option<&[u8]> = None;

        while !members.remaining().is_empty() {
            let attribute_local_offset = child_offset(
                rdn_value_offset,
                rdn_value_length,
                members.remaining().len(),
            );
            let attribute_offset = rdn_offset + attribute_local_offset;
            let attribute_element = members
                .read(decoder)
                .map_err(|error| {
                    X509NameError::rdn_child_structure(
                        error,
                        rdn_offset,
                        rdn_value_offset,
                        attribute_local_offset,
                    )
                })?
                .expect("a non-empty DER cursor must yield one element");

            if attribute_count == MAX_X509_NAME_ATTRIBUTES {
                return Err(X509NameError::new(
                    X509NameErrorKind::TooManyAttributes,
                    attribute_offset,
                ));
            }
            if previous_encoding.is_some_and(|previous| previous > attribute_element.encoded()) {
                return Err(X509NameError::new(
                    X509NameErrorKind::UnsortedRelativeDistinguishedName,
                    attribute_offset,
                ));
            }

            let attribute = decode_attribute_type_and_value(decoder, attribute_element)
                .map_err(|error| X509NameError::attribute(error, attribute_offset))?;
            previous_encoding = Some(attribute_element.encoded());
            attributes[attribute_count] = Some(attribute);
            attribute_count += 1;
        }

        if attribute_count == first_attribute {
            return Err(X509NameError::new(
                X509NameErrorKind::EmptyRelativeDistinguishedName,
                rdn_offset + rdn_value_offset,
            ));
        }

        rdn_ends[rdn_count] = attribute_count;
        rdn_count += 1;
    }

    Ok(X509Name {
        encoded,
        attributes,
        rdn_ends,
        rdn_count,
        attribute_count,
    })
}

fn decode_attribute_type_and_value<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509AttributeTypeAndValue<'a>, X509AttributeTypeAndValueError> {
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509AttributeTypeAndValueError::structure)?;

    let type_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let type_element = fields
        .read(decoder)
        .map_err(|error| {
            X509AttributeTypeAndValueError::child_structure(error, value_offset, type_offset)
        })?
        .ok_or_else(|| {
            X509AttributeTypeAndValueError::new(
                X509AttributeTypeAndValueErrorKind::MissingType,
                type_offset,
            )
        })?;
    let attribute_type =
        decode_object_identifier(type_element, decoder.limits()).map_err(|error| {
            X509AttributeTypeAndValueError::new(
                X509AttributeTypeAndValueErrorKind::InvalidType(error.kind()),
                type_offset + error.offset(),
            )
        })?;

    let value_field_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let value = fields
        .read(decoder)
        .map_err(|error| {
            X509AttributeTypeAndValueError::child_structure(error, value_offset, value_field_offset)
        })?
        .ok_or_else(|| {
            X509AttributeTypeAndValueError::new(
                X509AttributeTypeAndValueErrorKind::MissingValue,
                value_field_offset,
            )
        })?;

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            X509AttributeTypeAndValueError::child_structure(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(X509AttributeTypeAndValueError::new(
            X509AttributeTypeAndValueErrorKind::TrailingElement,
            trailing_offset,
        ));
    }

    Ok(X509AttributeTypeAndValue {
        attribute_type,
        value,
    })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
