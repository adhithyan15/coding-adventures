//! # Bounded DER TLV framing
//!
//! This crate splits one borrowed ASN.1 Distinguished Encoding Rules value
//! into its identifier, header, value, and complete encoding. It deliberately
//! stops before ASN.1 type semantics, recursive traversal, X.509, trust, and
//! transport. Accepting an element proves only that its framing is canonical.

#![deny(unsafe_code)]

use std::error::Error;
use std::fmt;

/// Default maximum input accepted by one decoder or cursor (1 MiB).
pub const DEFAULT_MAX_INPUT_LEN: usize = 1024 * 1024;
/// Default maximum value accepted by one element (1 MiB).
pub const DEFAULT_MAX_VALUE_LEN: usize = 1024 * 1024;
/// Default maximum number of sibling elements consumed by one cursor.
pub const DEFAULT_MAX_ELEMENTS: usize = 4096;

/// Explicit work and representation limits for hostile DER input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DerLimits {
    pub max_input_len: usize,
    pub max_value_len: usize,
    pub max_elements: usize,
    pub max_tag_number: u32,
}

impl Default for DerLimits {
    fn default() -> Self {
        Self {
            max_input_len: DEFAULT_MAX_INPUT_LEN,
            max_value_len: DEFAULT_MAX_VALUE_LEN,
            max_elements: DEFAULT_MAX_ELEMENTS,
            max_tag_number: u32::MAX,
        }
    }
}

/// The class bits from an ASN.1 identifier octet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagClass {
    Universal,
    Application,
    ContextSpecific,
    Private,
}

/// One decoded DER identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DerTag {
    pub class: TagClass,
    pub constructed: bool,
    pub number: u32,
}

/// One borrowed DER element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DerElement<'a> {
    tag: DerTag,
    encoded: &'a [u8],
    header_len: usize,
}

impl<'a> DerElement<'a> {
    pub const fn tag(&self) -> DerTag {
        self.tag
    }

    pub fn header(&self) -> &'a [u8] {
        &self.encoded[..self.header_len]
    }

    pub fn value(&self) -> &'a [u8] {
        &self.encoded[self.header_len..]
    }

    pub const fn encoded(&self) -> &'a [u8] {
        self.encoded
    }
}

/// Stable, bounded failure categories. No variant retains input bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DerErrorKind {
    EmptyInput,
    TruncatedHighTag,
    TruncatedLength,
    TruncatedValue,
    EndOfContents,
    NonMinimalTag,
    TagOverflow,
    IndefiniteLength,
    ReservedLength,
    NonMinimalLength,
    LengthTooWide,
    LengthHostOverflow,
    InputLimitExceeded,
    ValueLimitExceeded,
    ElementLimitExceeded,
    TagLimitExceeded,
    TrailingData,
}

/// A redacted framing failure at one byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DerError {
    kind: DerErrorKind,
    offset: usize,
}

impl DerError {
    const fn new(kind: DerErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    pub const fn kind(self) -> DerErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for DerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "DER framing error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for DerError {}

/// Decode one canonical DER element and return its untouched remainder.
pub fn decode_one(input: &[u8], limits: DerLimits) -> Result<(DerElement<'_>, &[u8]), DerError> {
    decode_one_at(input, limits, 0)
}

/// Decode exactly one canonical DER element, rejecting trailing bytes.
pub fn decode_exact(input: &[u8], limits: DerLimits) -> Result<DerElement<'_>, DerError> {
    let (element, remainder) = decode_one(input, limits)?;
    if !remainder.is_empty() {
        return Err(DerError::new(
            DerErrorKind::TrailingData,
            element.encoded.len(),
        ));
    }
    Ok(element)
}

/// Iterative, bounded decoding of sibling elements from one borrowed slice.
#[derive(Clone, Debug)]
pub struct DerCursor<'a> {
    remaining: &'a [u8],
    limits: DerLimits,
    initial_len: usize,
    elements_read: usize,
}

impl<'a> DerCursor<'a> {
    pub fn new(input: &'a [u8], limits: DerLimits) -> Result<Self, DerError> {
        check_input_limit(input, limits, 0)?;
        Ok(Self {
            remaining: input,
            limits,
            initial_len: input.len(),
            elements_read: 0,
        })
    }

    /// Read the next sibling. Failure leaves the cursor unchanged.
    pub fn read(&mut self) -> Result<Option<DerElement<'a>>, DerError> {
        if self.remaining.is_empty() {
            return Ok(None);
        }

        let offset = self.initial_len - self.remaining.len();
        if self.elements_read >= self.limits.max_elements {
            return Err(DerError::new(DerErrorKind::ElementLimitExceeded, offset));
        }

        let (element, remainder) = decode_one_at(self.remaining, self.limits, offset)?;
        self.remaining = remainder;
        self.elements_read += 1;
        Ok(Some(element))
    }

    /// Require that all cursor input has been consumed.
    pub fn finish(self) -> Result<(), DerError> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            Err(DerError::new(
                DerErrorKind::TrailingData,
                self.initial_len - self.remaining.len(),
            ))
        }
    }

    pub const fn elements_read(&self) -> usize {
        self.elements_read
    }

    pub const fn remaining(&self) -> &'a [u8] {
        self.remaining
    }
}

fn decode_one_at<'a>(
    input: &'a [u8],
    limits: DerLimits,
    base_offset: usize,
) -> Result<(DerElement<'a>, &'a [u8]), DerError> {
    check_input_limit(input, limits, base_offset)?;
    let first = *input
        .first()
        .ok_or_else(|| DerError::new(DerErrorKind::EmptyInput, base_offset))?;

    let class = match first >> 6 {
        0 => TagClass::Universal,
        1 => TagClass::Application,
        2 => TagClass::ContextSpecific,
        _ => TagClass::Private,
    };
    let constructed = first & 0x20 != 0;
    let (number, identifier_len) = decode_tag_number(input, first, limits, base_offset)?;
    if class == TagClass::Universal && number == 0 {
        return Err(DerError::new(DerErrorKind::EndOfContents, base_offset));
    }

    let (value_len, length_len) = decode_length(input, identifier_len, base_offset)?;
    if value_len > limits.max_value_len {
        return Err(DerError::new(
            DerErrorKind::ValueLimitExceeded,
            add_offset(base_offset, identifier_len),
        ));
    }

    let header_len = identifier_len
        .checked_add(length_len)
        .ok_or_else(|| DerError::new(DerErrorKind::LengthHostOverflow, base_offset))?;
    let encoded_len = header_len
        .checked_add(value_len)
        .ok_or_else(|| DerError::new(DerErrorKind::LengthHostOverflow, base_offset))?;
    if encoded_len > input.len() {
        return Err(DerError::new(
            DerErrorKind::TruncatedValue,
            add_offset(base_offset, input.len()),
        ));
    }

    let encoded = &input[..encoded_len];
    let element = DerElement {
        tag: DerTag {
            class,
            constructed,
            number,
        },
        encoded,
        header_len,
    };
    Ok((element, &input[encoded_len..]))
}

fn check_input_limit(input: &[u8], limits: DerLimits, offset: usize) -> Result<(), DerError> {
    if input.len() > limits.max_input_len {
        Err(DerError::new(DerErrorKind::InputLimitExceeded, offset))
    } else {
        Ok(())
    }
}

fn decode_tag_number(
    input: &[u8],
    first: u8,
    limits: DerLimits,
    base_offset: usize,
) -> Result<(u32, usize), DerError> {
    let low = first & 0x1f;
    if low != 0x1f {
        let number = u32::from(low);
        if number > limits.max_tag_number {
            return Err(DerError::new(DerErrorKind::TagLimitExceeded, base_offset));
        }
        return Ok((number, 1));
    }

    let mut number = 0u32;
    let mut index = 1usize;
    loop {
        let octet = *input.get(index).ok_or_else(|| {
            DerError::new(
                DerErrorKind::TruncatedHighTag,
                add_offset(base_offset, index),
            )
        })?;
        let payload = u32::from(octet & 0x7f);
        if index == 1 && payload == 0 {
            return Err(DerError::new(
                DerErrorKind::NonMinimalTag,
                add_offset(base_offset, index),
            ));
        }
        number = number
            .checked_mul(128)
            .and_then(|value| value.checked_add(payload))
            .ok_or_else(|| {
                DerError::new(DerErrorKind::TagOverflow, add_offset(base_offset, index))
            })?;
        if number > limits.max_tag_number {
            return Err(DerError::new(
                DerErrorKind::TagLimitExceeded,
                add_offset(base_offset, index),
            ));
        }
        index += 1;
        if octet & 0x80 == 0 {
            if number < 31 {
                return Err(DerError::new(DerErrorKind::NonMinimalTag, base_offset));
            }
            return Ok((number, index));
        }
    }
}

fn decode_length(
    input: &[u8],
    identifier_len: usize,
    base_offset: usize,
) -> Result<(usize, usize), DerError> {
    let length_offset = add_offset(base_offset, identifier_len);
    let first = *input
        .get(identifier_len)
        .ok_or_else(|| DerError::new(DerErrorKind::TruncatedLength, length_offset))?;
    if first < 0x80 {
        return Ok((usize::from(first), 1));
    }
    if first == 0x80 {
        return Err(DerError::new(DerErrorKind::IndefiniteLength, length_offset));
    }
    if first == 0xff {
        return Err(DerError::new(DerErrorKind::ReservedLength, length_offset));
    }

    let octet_count = usize::from(first & 0x7f);
    if octet_count > std::mem::size_of::<u64>() {
        return Err(DerError::new(DerErrorKind::LengthTooWide, length_offset));
    }
    let first_value_offset = identifier_len
        .checked_add(1)
        .ok_or_else(|| DerError::new(DerErrorKind::LengthHostOverflow, length_offset))?;
    let end = first_value_offset
        .checked_add(octet_count)
        .ok_or_else(|| DerError::new(DerErrorKind::LengthHostOverflow, length_offset))?;
    let octets = input.get(first_value_offset..end).ok_or_else(|| {
        DerError::new(
            DerErrorKind::TruncatedLength,
            add_offset(base_offset, input.len()),
        )
    })?;
    if octets.first() == Some(&0) {
        return Err(DerError::new(
            DerErrorKind::NonMinimalLength,
            add_offset(base_offset, first_value_offset),
        ));
    }

    let mut value = 0u64;
    for octet in octets {
        value = value
            .checked_mul(256)
            .and_then(|current| current.checked_add(u64::from(*octet)))
            .ok_or_else(|| DerError::new(DerErrorKind::LengthTooWide, length_offset))?;
    }
    if value < 128 {
        return Err(DerError::new(DerErrorKind::NonMinimalLength, length_offset));
    }
    let value = usize::try_from(value)
        .map_err(|_| DerError::new(DerErrorKind::LengthHostOverflow, length_offset))?;
    Ok((value, 1 + octet_count))
}

fn add_offset(base: usize, relative: usize) -> usize {
    base.saturating_add(relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact(input: &[u8]) -> DerElement<'_> {
        decode_exact(input, DerLimits::default()).expect("valid DER framing")
    }

    fn assert_error(input: &[u8], kind: DerErrorKind) -> DerError {
        let error = decode_exact(input, DerLimits::default()).expect_err("must fail closed");
        assert_eq!(error.kind(), kind);
        error
    }

    #[test]
    fn decodes_classes_and_constructed_bit() {
        let cases = [
            (0x02, TagClass::Universal, false),
            (0x42, TagClass::Application, false),
            (0xa2, TagClass::ContextSpecific, true),
            (0xe2, TagClass::Private, true),
        ];
        for (identifier, class, constructed) in cases {
            let encoded = [identifier, 0];
            let element = exact(&encoded);
            assert_eq!(
                element.tag(),
                DerTag {
                    class,
                    constructed,
                    number: 2
                }
            );
        }
    }

    #[test]
    fn exposes_borrowed_header_value_and_encoding() {
        let input = [0x04, 0x03, 1, 2, 3];
        let element = exact(&input);
        assert_eq!(element.header(), &[0x04, 0x03]);
        assert_eq!(element.value(), &[1, 2, 3]);
        assert_eq!(element.encoded(), &input);
    }

    #[test]
    fn accepts_tag_thirty_and_minimal_high_tags() {
        assert_eq!(exact(&[0x1e, 0]).tag().number, 30);
        assert_eq!(exact(&[0x1f, 0x1f, 0]).tag().number, 31);
        assert_eq!(exact(&[0x9f, 0x81, 0x00, 0]).tag().number, 128);
    }

    #[test]
    fn rejects_invalid_high_tags() {
        assert_error(&[0x1f], DerErrorKind::TruncatedHighTag);
        assert_error(&[0x1f, 0x81], DerErrorKind::TruncatedHighTag);
        assert_error(&[0x1f, 0x00, 0], DerErrorKind::NonMinimalTag);
        assert_error(&[0x1f, 0x1e, 0], DerErrorKind::NonMinimalTag);
    }

    #[test]
    fn rejects_tag_overflow_and_configured_tag_limit() {
        assert_error(
            &[0x1f, 0x90, 0x80, 0x80, 0x80, 0x00, 0],
            DerErrorKind::TagOverflow,
        );
        let limits = DerLimits {
            max_tag_number: 30,
            ..DerLimits::default()
        };
        assert_eq!(
            decode_exact(&[0x1f, 0x1f, 0], limits).unwrap_err().kind(),
            DerErrorKind::TagLimitExceeded
        );
    }

    #[test]
    fn rejects_universal_end_of_contents() {
        assert_error(&[0x00, 0x00], DerErrorKind::EndOfContents);
        assert_error(&[0x20, 0x00], DerErrorKind::EndOfContents);
    }

    #[test]
    fn accepts_short_length_boundaries() {
        assert!(exact(&[0x04, 0]).value().is_empty());
        let mut encoded = vec![0x04, 127];
        encoded.extend([0; 127]);
        assert_eq!(exact(&encoded).value().len(), 127);
    }

    #[test]
    fn accepts_minimal_long_length() {
        let mut encoded = vec![0x04, 0x81, 0x80];
        encoded.extend([0; 128]);
        let element = exact(&encoded);
        assert_eq!(element.header(), &[0x04, 0x81, 0x80]);
        assert_eq!(element.value().len(), 128);
    }

    #[test]
    fn rejects_ber_and_nonminimal_lengths() {
        assert_error(&[0x04, 0x80], DerErrorKind::IndefiniteLength);
        assert_error(&[0x04, 0xff], DerErrorKind::ReservedLength);
        assert_error(&[0x04, 0x81, 0x7f], DerErrorKind::NonMinimalLength);
        assert_error(&[0x04, 0x82, 0x00, 0x80], DerErrorKind::NonMinimalLength);
        assert_error(&[0x04, 0x89], DerErrorKind::LengthTooWide);
    }

    #[test]
    fn rejects_truncated_input_at_each_boundary() {
        assert_error(&[], DerErrorKind::EmptyInput);
        assert_error(&[0x04], DerErrorKind::TruncatedLength);
        assert_error(&[0x04, 0x82, 0x01], DerErrorKind::TruncatedLength);
        let error = assert_error(&[0x04, 0x03, 1, 2], DerErrorKind::TruncatedValue);
        assert_eq!(error.offset(), 4);
    }

    #[test]
    fn decode_one_returns_untouched_remainder() {
        let input = [0x02, 0x01, 7, 0x05, 0x00];
        let (first, remainder) = decode_one(&input, DerLimits::default()).unwrap();
        assert_eq!(first.encoded(), &[0x02, 0x01, 7]);
        assert_eq!(remainder, &[0x05, 0x00]);
        assert_eq!(
            decode_exact(&input, DerLimits::default())
                .unwrap_err()
                .kind(),
            DerErrorKind::TrailingData
        );
    }

    #[test]
    fn enforces_input_and_value_limits_at_boundaries() {
        let input = [0x04, 0x01, 0xaa];
        let exact_limits = DerLimits {
            max_input_len: 3,
            max_value_len: 1,
            ..DerLimits::default()
        };
        assert_eq!(decode_exact(&input, exact_limits).unwrap().value(), &[0xaa]);
        let input_limit = DerLimits {
            max_input_len: 2,
            ..exact_limits
        };
        assert_eq!(
            decode_exact(&input, input_limit).unwrap_err().kind(),
            DerErrorKind::InputLimitExceeded
        );
        let value_limit = DerLimits {
            max_value_len: 0,
            ..exact_limits
        };
        assert_eq!(
            decode_exact(&input, value_limit).unwrap_err().kind(),
            DerErrorKind::ValueLimitExceeded
        );
    }

    #[test]
    fn cursor_decodes_siblings_and_tracks_offsets() {
        let input = [0x02, 0x01, 1, 0x04, 0x01, 2];
        let mut cursor = DerCursor::new(&input, DerLimits::default()).unwrap();
        assert_eq!(cursor.read().unwrap().unwrap().value(), &[1]);
        assert_eq!(cursor.elements_read(), 1);
        assert_eq!(cursor.read().unwrap().unwrap().value(), &[2]);
        assert_eq!(cursor.read().unwrap(), None);
        cursor.finish().unwrap();

        let mut invalid = DerCursor::new(&[0x05, 0x00, 0x04], DerLimits::default()).unwrap();
        invalid.read().unwrap();
        let error = invalid.read().unwrap_err();
        assert_eq!(error.kind(), DerErrorKind::TruncatedLength);
        assert_eq!(error.offset(), 3);
    }

    #[test]
    fn cursor_budget_fails_without_advancing() {
        let limits = DerLimits {
            max_elements: 1,
            ..DerLimits::default()
        };
        let input = [0x05, 0x00, 0x05, 0x00];
        let mut cursor = DerCursor::new(&input, limits).unwrap();
        cursor.read().unwrap();
        let before = cursor.remaining();
        let error = cursor.read().unwrap_err();
        assert_eq!(error.kind(), DerErrorKind::ElementLimitExceeded);
        assert_eq!(error.offset(), 2);
        assert_eq!(cursor.remaining(), before);
        assert_eq!(cursor.elements_read(), 1);
    }

    #[test]
    fn parse_failure_does_not_advance_cursor() {
        let mut cursor = DerCursor::new(&[0x04, 0x02, 1], DerLimits::default()).unwrap();
        let before = cursor.remaining();
        assert_eq!(
            cursor.read().unwrap_err().kind(),
            DerErrorKind::TruncatedValue
        );
        assert_eq!(cursor.remaining(), before);
        assert_eq!(cursor.elements_read(), 0);
    }

    #[test]
    fn cursor_finish_rejects_unconsumed_bytes() {
        let cursor = DerCursor::new(&[0x05, 0x00], DerLimits::default()).unwrap();
        assert_eq!(
            cursor.finish().unwrap_err().kind(),
            DerErrorKind::TrailingData
        );
    }

    #[test]
    fn errors_are_redacted() {
        let error = decode_exact(&[0xde, 0xad, 0xbe, 0xef], DerLimits::default()).unwrap_err();
        let rendered = error.to_string();
        assert_eq!(rendered, "DER framing error LengthTooWide at byte 1");
    }
}
