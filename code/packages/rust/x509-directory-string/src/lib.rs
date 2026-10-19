//! # Bounded RFC 5280 DirectoryString values
//!
//! This crate recognizes the five DirectoryString alternatives without
//! allocation. Four alternatives expose validated Unicode scalar iteration;
//! TeletexString remains opaque because this layer owns no T.61 mapping.

#![deny(unsafe_code)]

use der_asn1::Asn1Element;
use der_tlv::TagClass;
use std::error::Error;
use std::fmt;
use std::str::{Bytes, Chars};

const UTF8_STRING_TAG: u32 = 12;
const PRINTABLE_STRING_TAG: u32 = 19;
const TELETEX_STRING_TAG: u32 = 20;
const UNIVERSAL_STRING_TAG: u32 = 28;
const BMP_STRING_TAG: u32 = 30;

/// The wire encoding selected by one RFC 5280 DirectoryString value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509DirectoryStringEncoding {
    Utf8,
    Printable,
    Teletex,
    Universal,
    Bmp,
}

/// Stable decoding failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509DirectoryStringErrorKind {
    UnexpectedTag,
    EmptyValue,
    InvalidUtf8,
    InvalidPrintableCharacter,
    InvalidUniversalLength,
    InvalidUniversalScalar,
    InvalidBmpLength,
    InvalidBmpScalar,
}

/// A redacted failure at a byte offset local to the encoded element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509DirectoryStringError {
    kind: X509DirectoryStringErrorKind,
    offset: usize,
}

impl X509DirectoryStringError {
    const fn new(kind: X509DirectoryStringErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    pub const fn kind(self) -> X509DirectoryStringErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509DirectoryStringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 DirectoryString error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509DirectoryStringError {}

/// One borrowed, validated RFC 5280 DirectoryString alternative.
///
/// The representation is deliberately private: safe callers cannot bypass
/// [`decode_x509_directory_string`] and forge malformed fixed-width contents.
///
/// ```compile_fail
/// use x509_directory_string::X509DirectoryString;
///
/// let _forged = X509DirectoryString { value: () };
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509DirectoryString<'a> {
    value: DirectoryStringValue<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectoryStringValue<'a> {
    Utf8(&'a str),
    Printable(&'a str),
    Teletex(&'a [u8]),
    Universal(&'a [u8]),
    Bmp(&'a [u8]),
}

impl<'a> X509DirectoryString<'a> {
    pub const fn encoding(self) -> X509DirectoryStringEncoding {
        match self.value {
            DirectoryStringValue::Utf8(_) => X509DirectoryStringEncoding::Utf8,
            DirectoryStringValue::Printable(_) => X509DirectoryStringEncoding::Printable,
            DirectoryStringValue::Teletex(_) => X509DirectoryStringEncoding::Teletex,
            DirectoryStringValue::Universal(_) => X509DirectoryStringEncoding::Universal,
            DirectoryStringValue::Bmp(_) => X509DirectoryStringEncoding::Bmp,
        }
    }

    /// Return the exact content octets, excluding the DER tag and length.
    pub fn contents(self) -> &'a [u8] {
        match self.value {
            DirectoryStringValue::Utf8(value) | DirectoryStringValue::Printable(value) => {
                value.as_bytes()
            }
            DirectoryStringValue::Teletex(value)
            | DirectoryStringValue::Universal(value)
            | DirectoryStringValue::Bmp(value) => value,
        }
    }

    /// Iterate validated Unicode scalar values when this encoding has an
    /// unambiguous mapping owned by this package.
    ///
    /// TeletexString returns `None`; callers must not infer a T.61 mapping from
    /// its opaque bytes.
    pub fn unicode_scalars(self) -> Option<X509DirectoryStringScalars<'a>> {
        let source = match self.value {
            DirectoryStringValue::Utf8(value) => ScalarSource::Utf8(value.chars()),
            DirectoryStringValue::Printable(value) => ScalarSource::Printable(value.bytes()),
            DirectoryStringValue::Teletex(_) => return None,
            DirectoryStringValue::Universal(value) => ScalarSource::Universal(value),
            DirectoryStringValue::Bmp(value) => ScalarSource::Bmp(value),
        };
        Some(X509DirectoryStringScalars { source, offset: 0 })
    }
}

/// Failures from exact Unicode-scalar comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509DirectoryStringComparisonError {
    UnsupportedTeletexString,
}

impl fmt::Display for X509DirectoryStringComparisonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTeletexString => {
                formatter.write_str("TeletexString has no configured character mapping")
            }
        }
    }
}

impl Error for X509DirectoryStringComparisonError {}

/// Compare two decoded DirectoryString values by exact Unicode scalar
/// sequence, independent of their supported wire encodings.
///
/// This function refuses TeletexString. It performs no StringPrep, case
/// folding, normalization, whitespace handling, or attribute-schema policy,
/// and therefore is not complete RFC 5280 distinguished-name equality.
pub fn compare_x509_directory_strings_by_scalar(
    left: X509DirectoryString<'_>,
    right: X509DirectoryString<'_>,
) -> Result<bool, X509DirectoryStringComparisonError> {
    let mut left = left
        .unicode_scalars()
        .ok_or(X509DirectoryStringComparisonError::UnsupportedTeletexString)?;
    let mut right = right
        .unicode_scalars()
        .ok_or(X509DirectoryStringComparisonError::UnsupportedTeletexString)?;

    loop {
        match (left.next(), right.next()) {
            (Some(left), Some(right)) if left == right => {}
            (None, None) => return Ok(true),
            _ => return Ok(false),
        }
    }
}

/// Decode one already framed RFC 5280 DirectoryString value.
pub fn decode_x509_directory_string(
    element: Asn1Element<'_>,
) -> Result<X509DirectoryString<'_>, X509DirectoryStringError> {
    let tag = element.tag();
    if tag.class != TagClass::Universal || tag.constructed {
        return Err(X509DirectoryStringError::new(
            X509DirectoryStringErrorKind::UnexpectedTag,
            0,
        ));
    }

    let contents = element.value();
    let value_offset = element.header().len();
    if contents.is_empty() {
        return Err(X509DirectoryStringError::new(
            X509DirectoryStringErrorKind::EmptyValue,
            value_offset,
        ));
    }

    match tag.number {
        UTF8_STRING_TAG => decode_utf8(contents, value_offset),
        PRINTABLE_STRING_TAG => decode_printable(contents, value_offset),
        TELETEX_STRING_TAG => Ok(X509DirectoryString {
            value: DirectoryStringValue::Teletex(contents),
        }),
        UNIVERSAL_STRING_TAG => decode_universal(contents, value_offset),
        BMP_STRING_TAG => decode_bmp(contents, value_offset),
        _ => Err(X509DirectoryStringError::new(
            X509DirectoryStringErrorKind::UnexpectedTag,
            0,
        )),
    }
}

fn decode_utf8(
    contents: &[u8],
    value_offset: usize,
) -> Result<X509DirectoryString<'_>, X509DirectoryStringError> {
    std::str::from_utf8(contents)
        .map(|value| X509DirectoryString {
            value: DirectoryStringValue::Utf8(value),
        })
        .map_err(|error| {
            X509DirectoryStringError::new(
                X509DirectoryStringErrorKind::InvalidUtf8,
                value_offset + error.valid_up_to(),
            )
        })
}

fn decode_printable(
    contents: &[u8],
    value_offset: usize,
) -> Result<X509DirectoryString<'_>, X509DirectoryStringError> {
    if let Some(index) = contents
        .iter()
        .position(|byte| !is_printable_string_character(*byte))
    {
        return Err(X509DirectoryStringError::new(
            X509DirectoryStringErrorKind::InvalidPrintableCharacter,
            value_offset + index,
        ));
    }
    // Every permitted PrintableString octet is ASCII.
    let value = match std::str::from_utf8(contents) {
        Ok(value) => value,
        Err(_) => {
            return Err(X509DirectoryStringError::new(
                X509DirectoryStringErrorKind::InvalidPrintableCharacter,
                value_offset,
            ));
        }
    };
    Ok(X509DirectoryString {
        value: DirectoryStringValue::Printable(value),
    })
}

const fn is_printable_string_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b' ' | b'\'' | b'(' | b')' | b'+' | b',' | b'-' | b'.' | b'/' | b':' | b'=' | b'?'
        )
}

fn decode_universal(
    contents: &[u8],
    value_offset: usize,
) -> Result<X509DirectoryString<'_>, X509DirectoryStringError> {
    if !contents.len().is_multiple_of(4) {
        return Err(X509DirectoryStringError::new(
            X509DirectoryStringErrorKind::InvalidUniversalLength,
            value_offset + contents.len(),
        ));
    }
    for (index, scalar) in contents.as_chunks::<4>().0.iter().enumerate() {
        let scalar = u32::from_be_bytes([scalar[0], scalar[1], scalar[2], scalar[3]]);
        if char::from_u32(scalar).is_none() {
            return Err(X509DirectoryStringError::new(
                X509DirectoryStringErrorKind::InvalidUniversalScalar,
                value_offset + index * 4,
            ));
        }
    }
    Ok(X509DirectoryString {
        value: DirectoryStringValue::Universal(contents),
    })
}

fn decode_bmp(
    contents: &[u8],
    value_offset: usize,
) -> Result<X509DirectoryString<'_>, X509DirectoryStringError> {
    if !contents.len().is_multiple_of(2) {
        return Err(X509DirectoryStringError::new(
            X509DirectoryStringErrorKind::InvalidBmpLength,
            value_offset + contents.len(),
        ));
    }
    for (index, scalar) in contents.as_chunks::<2>().0.iter().enumerate() {
        let scalar = u16::from_be_bytes([scalar[0], scalar[1]]);
        if char::from_u32(u32::from(scalar)).is_none() {
            return Err(X509DirectoryStringError::new(
                X509DirectoryStringErrorKind::InvalidBmpScalar,
                value_offset + index * 2,
            ));
        }
    }
    Ok(X509DirectoryString {
        value: DirectoryStringValue::Bmp(contents),
    })
}

enum ScalarSource<'a> {
    Utf8(Chars<'a>),
    Printable(Bytes<'a>),
    Universal(&'a [u8]),
    Bmp(&'a [u8]),
}

/// Allocation-free Unicode scalar iterator over one validated value.
pub struct X509DirectoryStringScalars<'a> {
    source: ScalarSource<'a>,
    offset: usize,
}

impl Iterator for X509DirectoryStringScalars<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.source {
            ScalarSource::Utf8(chars) => chars.next(),
            ScalarSource::Printable(bytes) => bytes.next().map(char::from),
            ScalarSource::Universal(contents) => {
                let scalar = contents.get(self.offset..self.offset + 4)?;
                self.offset += 4;
                char::from_u32(u32::from_be_bytes([
                    scalar[0], scalar[1], scalar[2], scalar[3],
                ]))
            }
            ScalarSource::Bmp(contents) => {
                let scalar = contents.get(self.offset..self.offset + 2)?;
                self.offset += 2;
                char::from_u32(u32::from(u16::from_be_bytes([scalar[0], scalar[1]])))
            }
        }
    }
}
