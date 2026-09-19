//! # Typed, bounded ASN.1 DER values
//!
//! `der-tlv` answers where one canonical DER element ends. This crate answers
//! what a small set of universal values means, while preserving borrowed input
//! and one shared depth/element budget for a caller's schema walk. The split is
//! deliberate: framing, value canonicality, X.509 schema, and trust are
//! different security boundaries and should be reviewable independently.

#![deny(unsafe_code)]

use der_tlv::{
    decode_exact, DerCursor, DerElement, DerError, DerErrorKind, DerLimits, DerTag, TagClass,
};
use std::error::Error;
use std::fmt;

const BOOLEAN_TAG: u32 = 1;
const INTEGER_TAG: u32 = 2;
const BIT_STRING_TAG: u32 = 3;
const OCTET_STRING_TAG: u32 = 4;
const NULL_TAG: u32 = 5;
const OBJECT_IDENTIFIER_TAG: u32 = 6;
const SEQUENCE_TAG: u32 = 16;
const SET_TAG: u32 = 17;

/// Default number of levels in one document, including its root.
pub const DEFAULT_MAX_DEPTH: usize = 32;
/// Default number of elements decoded through one shared decoder.
pub const DEFAULT_MAX_TOTAL_ELEMENTS: usize = 16_384;
/// Default number of arcs accepted in one object identifier.
pub const DEFAULT_MAX_OID_ARCS: usize = 128;

/// Explicit framing, nesting, whole-document work, and OID limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Asn1Limits {
    pub der: DerLimits,
    pub max_depth: usize,
    pub max_total_elements: usize,
    pub max_oid_arcs: usize,
}

impl Default for Asn1Limits {
    fn default() -> Self {
        Self {
            der: DerLimits::default(),
            max_depth: DEFAULT_MAX_DEPTH,
            max_total_elements: DEFAULT_MAX_TOTAL_ELEMENTS,
            max_oid_arcs: DEFAULT_MAX_OID_ARCS,
        }
    }
}

/// Stable failure categories. No category contains input bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asn1ErrorKind {
    Framing(DerErrorKind),
    UnexpectedTag,
    DecoderLimitMismatch,
    DepthLimitExceeded,
    ElementLimitExceeded,
    InvalidBooleanLength,
    InvalidBooleanValue,
    EmptyInteger,
    NonMinimalInteger,
    NegativeInteger,
    IntegerOverflow,
    MissingUnusedBitCount,
    InvalidUnusedBitCount,
    NonZeroBitPadding,
    BitLengthOverflow,
    NonEmptyNull,
    EmptyObjectIdentifier,
    UnterminatedObjectIdentifier,
    NonMinimalObjectIdentifier,
    ObjectIdentifierOverflow,
    OidArcLimitExceeded,
}

/// A redacted typed-DER failure at a local byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Asn1Error {
    kind: Asn1ErrorKind,
    offset: usize,
}

impl Asn1Error {
    const fn new(kind: Asn1ErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn framing(error: DerError) -> Self {
        Self::new(Asn1ErrorKind::Framing(error.kind()), error.offset())
    }

    pub const fn kind(self) -> Asn1ErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for Asn1Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "ASN.1 DER value error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for Asn1Error {}

/// One immutable framed element plus its decoder-derived document depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Asn1Element<'a> {
    element: DerElement<'a>,
    depth: usize,
}

impl<'a> Asn1Element<'a> {
    pub const fn tag(&self) -> DerTag {
        self.element.tag()
    }

    pub fn header(&self) -> &'a [u8] {
        self.element.header()
    }

    pub fn value(&self) -> &'a [u8] {
        self.element.value()
    }

    pub const fn encoded(&self) -> &'a [u8] {
        self.element.encoded()
    }

    pub const fn depth(&self) -> usize {
        self.depth
    }

    fn value_offset(&self) -> usize {
        self.header().len()
    }
}

/// Shared depth and total-element authority for one schema walk.
#[derive(Debug)]
pub struct Asn1Decoder {
    limits: Asn1Limits,
    elements_read: usize,
}

impl Asn1Decoder {
    pub const fn new(limits: Asn1Limits) -> Self {
        Self {
            limits,
            elements_read: 0,
        }
    }

    /// Decode one exact depth-zero document root.
    ///
    /// Both framing and work accounting are transactional: failure changes no
    /// counter, including when valid input arrives after the budget is spent.
    pub fn decode_exact<'a>(&mut self, input: &'a [u8]) -> Result<Asn1Element<'a>, Asn1Error> {
        if self.limits.max_depth == 0 {
            return Err(Asn1Error::new(Asn1ErrorKind::DepthLimitExceeded, 0));
        }
        self.require_element_capacity(0)?;
        let element = decode_exact(input, self.limits.der).map_err(Asn1Error::framing)?;
        self.elements_read += 1;
        Ok(Asn1Element { element, depth: 0 })
    }

    /// Open an exact universal SEQUENCE as a bounded child cursor.
    pub fn sequence<'a>(&self, element: Asn1Element<'a>) -> Result<Asn1Cursor<'a>, Asn1Error> {
        self.constructed(element, TagClass::Universal, SEQUENCE_TAG)
    }

    /// Open an exact universal SET as a bounded child cursor.
    pub fn set<'a>(&self, element: Asn1Element<'a>) -> Result<Asn1Cursor<'a>, Asn1Error> {
        self.constructed(element, TagClass::Universal, SET_TAG)
    }

    /// Decode the single child of an exact context-specific explicit wrapper.
    pub fn explicit<'a>(
        &mut self,
        element: Asn1Element<'a>,
        tag_number: u32,
    ) -> Result<Asn1Element<'a>, Asn1Error> {
        expect_tag(element, TagClass::ContextSpecific, true, tag_number)?;
        let child_depth = self.child_depth(element)?;
        self.require_element_capacity(element.value_offset())?;
        let child = decode_exact(element.value(), self.limits.der).map_err(Asn1Error::framing)?;
        self.elements_read += 1;
        Ok(Asn1Element {
            element: child,
            depth: child_depth,
        })
    }

    pub const fn elements_read(&self) -> usize {
        self.elements_read
    }

    pub const fn limits(&self) -> Asn1Limits {
        self.limits
    }

    fn constructed<'a>(
        &self,
        element: Asn1Element<'a>,
        class: TagClass,
        number: u32,
    ) -> Result<Asn1Cursor<'a>, Asn1Error> {
        expect_tag(element, class, true, number)?;
        let child_depth = self.child_depth(element)?;
        let cursor =
            DerCursor::new(element.value(), self.limits.der).map_err(Asn1Error::framing)?;
        Ok(Asn1Cursor {
            cursor,
            child_depth,
            limits: self.limits,
        })
    }

    fn child_depth(&self, element: Asn1Element<'_>) -> Result<usize, Asn1Error> {
        let child_depth = element
            .depth
            .checked_add(1)
            .ok_or_else(|| Asn1Error::new(Asn1ErrorKind::DepthLimitExceeded, 0))?;
        if child_depth >= self.limits.max_depth {
            Err(Asn1Error::new(Asn1ErrorKind::DepthLimitExceeded, 0))
        } else {
            Ok(child_depth)
        }
    }

    fn require_element_capacity(&self, offset: usize) -> Result<(), Asn1Error> {
        if self.elements_read >= self.limits.max_total_elements {
            Err(Asn1Error::new(Asn1ErrorKind::ElementLimitExceeded, offset))
        } else {
            Ok(())
        }
    }
}

/// An iterative sibling cursor whose reads share an [`Asn1Decoder`] budget.
#[derive(Debug)]
pub struct Asn1Cursor<'a> {
    cursor: DerCursor<'a>,
    child_depth: usize,
    limits: Asn1Limits,
}

impl<'a> Asn1Cursor<'a> {
    pub fn read(
        &mut self,
        decoder: &mut Asn1Decoder,
    ) -> Result<Option<Asn1Element<'a>>, Asn1Error> {
        if self.cursor.remaining().is_empty() {
            return Ok(None);
        }
        if decoder.limits != self.limits {
            return Err(Asn1Error::new(Asn1ErrorKind::DecoderLimitMismatch, 0));
        }
        decoder.require_element_capacity(0)?;
        let element = self.cursor.read().map_err(Asn1Error::framing)?;
        let Some(element) = element else {
            return Ok(None);
        };
        decoder.elements_read += 1;
        Ok(Some(Asn1Element {
            element,
            depth: self.child_depth,
        }))
    }

    pub fn finish(self) -> Result<(), Asn1Error> {
        self.cursor.finish().map_err(Asn1Error::framing)
    }

    pub const fn remaining(&self) -> &'a [u8] {
        self.cursor.remaining()
    }
}

/// Decode an exact canonical BOOLEAN.
pub fn decode_boolean(element: Asn1Element<'_>) -> Result<bool, Asn1Error> {
    expect_universal_primitive(element, BOOLEAN_TAG)?;
    let value = element.value();
    if value.len() != 1 {
        return Err(Asn1Error::new(
            Asn1ErrorKind::InvalidBooleanLength,
            element.value_offset(),
        ));
    }
    match value[0] {
        0x00 => Ok(false),
        0xff => Ok(true),
        _ => Err(Asn1Error::new(
            Asn1ErrorKind::InvalidBooleanValue,
            element.value_offset(),
        )),
    }
}

/// A borrowed, already-canonical two's-complement INTEGER.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DerInteger<'a> {
    bytes: &'a [u8],
    value_offset: usize,
}

impl<'a> DerInteger<'a> {
    pub const fn signed_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub fn is_negative(&self) -> bool {
        self.bytes[0] & 0x80 != 0
    }

    pub fn to_u64(self) -> Result<u64, Asn1Error> {
        if self.is_negative() {
            return Err(Asn1Error::new(
                Asn1ErrorKind::NegativeInteger,
                self.value_offset,
            ));
        }
        let magnitude = if self.bytes[0] == 0 {
            &self.bytes[1..]
        } else {
            self.bytes
        };
        if magnitude.len() > std::mem::size_of::<u64>() {
            return Err(Asn1Error::new(
                Asn1ErrorKind::IntegerOverflow,
                self.value_offset,
            ));
        }
        Ok(magnitude
            .iter()
            .fold(0u64, |value, byte| (value << 8) | u64::from(*byte)))
    }
}

/// Validate and borrow one canonical INTEGER value.
pub fn decode_integer(element: Asn1Element<'_>) -> Result<DerInteger<'_>, Asn1Error> {
    expect_universal_primitive(element, INTEGER_TAG)?;
    let bytes = element.value();
    if bytes.is_empty() {
        return Err(Asn1Error::new(
            Asn1ErrorKind::EmptyInteger,
            element.value_offset(),
        ));
    }
    if bytes.len() > 1 {
        let redundant_positive = bytes[0] == 0x00 && bytes[1] & 0x80 == 0;
        let redundant_negative = bytes[0] == 0xff && bytes[1] & 0x80 != 0;
        if redundant_positive || redundant_negative {
            return Err(Asn1Error::new(
                Asn1ErrorKind::NonMinimalInteger,
                element.value_offset(),
            ));
        }
    }
    Ok(DerInteger {
        bytes,
        value_offset: element.value_offset(),
    })
}

/// A borrowed BIT STRING after unused-bit and zero-padding validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DerBitString<'a> {
    bytes: &'a [u8],
    unused_bits: u8,
    bit_len: usize,
}

impl<'a> DerBitString<'a> {
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub const fn unused_bits(&self) -> u8 {
        self.unused_bits
    }

    pub const fn bit_len(&self) -> usize {
        self.bit_len
    }
}

/// Validate and borrow one canonical BIT STRING value.
pub fn decode_bit_string(element: Asn1Element<'_>) -> Result<DerBitString<'_>, Asn1Error> {
    expect_universal_primitive(element, BIT_STRING_TAG)?;
    let value = element.value();
    let Some((&unused_bits, bytes)) = value.split_first() else {
        return Err(Asn1Error::new(
            Asn1ErrorKind::MissingUnusedBitCount,
            element.value_offset(),
        ));
    };
    if unused_bits > 7 || (bytes.is_empty() && unused_bits != 0) {
        return Err(Asn1Error::new(
            Asn1ErrorKind::InvalidUnusedBitCount,
            element.value_offset(),
        ));
    }
    if unused_bits != 0 {
        let mask = (1u8 << unused_bits) - 1;
        if bytes.last().is_some_and(|byte| byte & mask != 0) {
            return Err(Asn1Error::new(
                Asn1ErrorKind::NonZeroBitPadding,
                element.value_offset() + value.len() - 1,
            ));
        }
    }
    let bit_len = bytes
        .len()
        .checked_mul(8)
        .and_then(|length| length.checked_sub(usize::from(unused_bits)))
        .ok_or_else(|| Asn1Error::new(Asn1ErrorKind::BitLengthOverflow, element.value_offset()))?;
    Ok(DerBitString {
        bytes,
        unused_bits,
        bit_len,
    })
}

/// Borrow one exact primitive OCTET STRING.
pub fn decode_octet_string(element: Asn1Element<'_>) -> Result<&[u8], Asn1Error> {
    expect_universal_primitive(element, OCTET_STRING_TAG)?;
    Ok(element.value())
}

/// Validate one exact, empty NULL value.
pub fn decode_null(element: Asn1Element<'_>) -> Result<(), Asn1Error> {
    expect_universal_primitive(element, NULL_TAG)?;
    if element.value().is_empty() {
        Ok(())
    } else {
        Err(Asn1Error::new(
            Asn1ErrorKind::NonEmptyNull,
            element.value_offset(),
        ))
    }
}

/// A borrowed, fully validated OBJECT IDENTIFIER.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectIdentifier<'a> {
    encoded: &'a [u8],
    first_end: usize,
    first_arc: u64,
    second_arc: u64,
    arc_count: usize,
}

impl<'a> ObjectIdentifier<'a> {
    pub const fn encoded(&self) -> &'a [u8] {
        self.encoded
    }

    pub const fn arc_count(&self) -> usize {
        self.arc_count
    }

    pub const fn arcs(&self) -> OidArcs<'a> {
        OidArcs {
            encoded: self.encoded,
            offset: self.first_end,
            first_arc: self.first_arc,
            second_arc: self.second_arc,
            prefix_index: 0,
            remaining: self.arc_count,
        }
    }

    pub fn equals(&self, expected: &[u64]) -> bool {
        expected.len() == self.arc_count && self.arcs().eq(expected.iter().copied())
    }
}

/// Infallible iteration over an already validated object identifier.
#[derive(Clone, Debug)]
pub struct OidArcs<'a> {
    encoded: &'a [u8],
    offset: usize,
    first_arc: u64,
    second_arc: u64,
    prefix_index: u8,
    remaining: usize,
}

impl Iterator for OidArcs<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let value = match self.prefix_index {
            0 => {
                self.prefix_index = 1;
                self.first_arc
            }
            1 => {
                self.prefix_index = 2;
                self.second_arc
            }
            _ => {
                let (value, next) = parse_base128_validated(self.encoded, self.offset)?;
                self.offset = next;
                value
            }
        };
        self.remaining -= 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for OidArcs<'_> {
    fn len(&self) -> usize {
        self.remaining
    }
}

/// Validate and borrow one canonical OBJECT IDENTIFIER.
pub fn decode_object_identifier(
    element: Asn1Element<'_>,
    limits: Asn1Limits,
) -> Result<ObjectIdentifier<'_>, Asn1Error> {
    expect_universal_primitive(element, OBJECT_IDENTIFIER_TAG)?;
    let encoded = element.value();
    if encoded.is_empty() {
        return Err(Asn1Error::new(
            Asn1ErrorKind::EmptyObjectIdentifier,
            element.value_offset(),
        ));
    }

    let (first_combined, first_end) = parse_base128(encoded, 0, element.value_offset())?;
    let (first_arc, second_arc) = if first_combined < 40 {
        (0, first_combined)
    } else if first_combined < 80 {
        (1, first_combined - 40)
    } else {
        (2, first_combined - 80)
    };

    let mut arc_count = 2usize;
    if arc_count > limits.max_oid_arcs {
        return Err(Asn1Error::new(
            Asn1ErrorKind::OidArcLimitExceeded,
            element.value_offset(),
        ));
    }
    let mut offset = first_end;
    while offset < encoded.len() {
        let (_, next) = parse_base128(encoded, offset, element.value_offset())?;
        arc_count = arc_count.checked_add(1).ok_or_else(|| {
            Asn1Error::new(
                Asn1ErrorKind::OidArcLimitExceeded,
                element.value_offset() + offset,
            )
        })?;
        if arc_count > limits.max_oid_arcs {
            return Err(Asn1Error::new(
                Asn1ErrorKind::OidArcLimitExceeded,
                element.value_offset() + offset,
            ));
        }
        offset = next;
    }

    Ok(ObjectIdentifier {
        encoded,
        first_end,
        first_arc,
        second_arc,
        arc_count,
    })
}

fn expect_universal_primitive(element: Asn1Element<'_>, number: u32) -> Result<(), Asn1Error> {
    expect_tag(element, TagClass::Universal, false, number)
}

fn expect_tag(
    element: Asn1Element<'_>,
    class: TagClass,
    constructed: bool,
    number: u32,
) -> Result<(), Asn1Error> {
    let tag = element.tag();
    if tag.class == class && tag.constructed == constructed && tag.number == number {
        Ok(())
    } else {
        Err(Asn1Error::new(Asn1ErrorKind::UnexpectedTag, 0))
    }
}

fn parse_base128(
    encoded: &[u8],
    start: usize,
    value_offset: usize,
) -> Result<(u64, usize), Asn1Error> {
    let first = *encoded.get(start).ok_or_else(|| {
        Asn1Error::new(
            Asn1ErrorKind::UnterminatedObjectIdentifier,
            value_offset + start,
        )
    })?;
    if first == 0x80 {
        return Err(Asn1Error::new(
            Asn1ErrorKind::NonMinimalObjectIdentifier,
            value_offset + start,
        ));
    }

    let mut value = 0u64;
    let mut offset = start;
    loop {
        let octet = *encoded.get(offset).ok_or_else(|| {
            Asn1Error::new(
                Asn1ErrorKind::UnterminatedObjectIdentifier,
                value_offset + offset,
            )
        })?;
        value = value
            .checked_mul(128)
            .and_then(|current| current.checked_add(u64::from(octet & 0x7f)))
            .ok_or_else(|| {
                Asn1Error::new(
                    Asn1ErrorKind::ObjectIdentifierOverflow,
                    value_offset + offset,
                )
            })?;
        offset += 1;
        if octet & 0x80 == 0 {
            return Ok((value, offset));
        }
    }
}

fn parse_base128_validated(encoded: &[u8], start: usize) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut offset = start;
    loop {
        let octet = *encoded.get(offset)?;
        value = value
            .checked_mul(128)?
            .checked_add(u64::from(octet & 0x7f))?;
        offset += 1;
        if octet & 0x80 == 0 {
            return Some((value, offset));
        }
    }
}
