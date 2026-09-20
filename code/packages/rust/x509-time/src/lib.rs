//! # RFC 5280 certificate time values
//!
//! DER's generic time types and the Internet PKI profile are deliberately
//! separate layers. `der-asn1` proves the element framing; this crate applies
//! RFC 5280's exact spelling, century, tag-selection, and calendar rules. It
//! never asks a clock whether the decoded instant is current.

#![deny(unsafe_code)]

use core::cmp::Ordering;
use der_asn1::Asn1Element;
use der_tlv::TagClass;
use std::error::Error;
use std::fmt;

const UTC_TIME_TAG: u32 = 23;
const GENERALIZED_TIME_TAG: u32 = 24;
const UTC_TIME_LENGTH: usize = 13;
const GENERALIZED_TIME_LENGTH: usize = 15;

/// Which RFC 5280 ASN.1 alternative carried the decoded instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509TimeEncoding {
    UtcTime,
    GeneralizedTime,
}

/// Stable failure categories. No variant stores certificate bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509TimeErrorKind {
    UnexpectedTag,
    InvalidLength,
    MissingZulu,
    NonDigit,
    GeneralizedTimeBefore2050,
    InvalidMonth,
    InvalidDay,
    InvalidHour,
    InvalidMinute,
    InvalidSecond,
}

/// A redacted profile-time failure at a local DER byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509TimeError {
    kind: X509TimeErrorKind,
    offset: usize,
}

impl X509TimeError {
    const fn new(kind: X509TimeErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    pub const fn kind(self) -> X509TimeErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509TimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 time error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509TimeError {}

/// One completely validated RFC 5280 time value.
///
/// Keeping plain fields instead of an epoch count avoids platform date APIs,
/// timezone databases, overflow-prone unit conversion, and an ambient clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509Time {
    encoding: X509TimeEncoding,
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl X509Time {
    pub const fn encoding(self) -> X509TimeEncoding {
        self.encoding
    }

    pub const fn year(self) -> u16 {
        self.year
    }

    pub const fn month(self) -> u8 {
        self.month
    }

    pub const fn day(self) -> u8 {
        self.day
    }

    pub const fn hour(self) -> u8 {
        self.hour
    }

    pub const fn minute(self) -> u8 {
        self.minute
    }

    pub const fn second(self) -> u8 {
        self.second
    }

    /// Compare two validated instants without consulting an ambient clock.
    pub fn cmp_fields(&self, other: &Self) -> Ordering {
        self.fields().cmp(&other.fields())
    }

    const fn fields(&self) -> (u16, u8, u8, u8, u8, u8) {
        (
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
        )
    }
}

/// Decode one exact RFC 5280 UTCTime or GeneralizedTime element.
pub fn decode_x509_time(element: Asn1Element<'_>) -> Result<X509Time, X509TimeError> {
    let tag = element.tag();
    if tag.class != TagClass::Universal || tag.constructed {
        return Err(X509TimeError::new(X509TimeErrorKind::UnexpectedTag, 0));
    }

    match tag.number {
        UTC_TIME_TAG => decode_utc_time(element),
        GENERALIZED_TIME_TAG => decode_generalized_time(element),
        _ => Err(X509TimeError::new(X509TimeErrorKind::UnexpectedTag, 0)),
    }
}

fn decode_utc_time(element: Asn1Element<'_>) -> Result<X509Time, X509TimeError> {
    let value = require_shape(element, UTC_TIME_LENGTH)?;
    let value_offset = element.header().len();
    let short_year = parse_digits(value, 0, 2, value_offset)?;
    let year = if short_year >= 50 {
        1900 + short_year
    } else {
        2000 + short_year
    };
    decode_fields(X509TimeEncoding::UtcTime, year, value, 2, value_offset)
}

fn decode_generalized_time(element: Asn1Element<'_>) -> Result<X509Time, X509TimeError> {
    let value = require_shape(element, GENERALIZED_TIME_LENGTH)?;
    let value_offset = element.header().len();
    let year = parse_digits(value, 0, 4, value_offset)?;
    if year < 2050 {
        return Err(X509TimeError::new(
            X509TimeErrorKind::GeneralizedTimeBefore2050,
            value_offset,
        ));
    }
    decode_fields(
        X509TimeEncoding::GeneralizedTime,
        year,
        value,
        4,
        value_offset,
    )
}

fn require_shape(element: Asn1Element<'_>, expected_length: usize) -> Result<&[u8], X509TimeError> {
    let value = element.value();
    let value_offset = element.header().len();
    if value.len() != expected_length {
        return Err(X509TimeError::new(
            X509TimeErrorKind::InvalidLength,
            value_offset,
        ));
    }
    if value.last() != Some(&b'Z') {
        return Err(X509TimeError::new(
            X509TimeErrorKind::MissingZulu,
            value_offset + value.len() - 1,
        ));
    }
    Ok(value)
}

fn decode_fields(
    encoding: X509TimeEncoding,
    year: u16,
    value: &[u8],
    field_start: usize,
    value_offset: usize,
) -> Result<X509Time, X509TimeError> {
    let month_offset = field_start;
    let day_offset = month_offset + 2;
    let hour_offset = day_offset + 2;
    let minute_offset = hour_offset + 2;
    let second_offset = minute_offset + 2;

    // Parse and validate in wire order. Besides making offsets intuitive, this
    // prevents a later malformed field from masking an earlier invalid one.
    let month = parse_u8(value, month_offset, value_offset)?;
    if !(1..=12).contains(&month) {
        return Err(X509TimeError::new(
            X509TimeErrorKind::InvalidMonth,
            value_offset + month_offset,
        ));
    }

    let day = parse_u8(value, day_offset, value_offset)?;
    if day == 0 || day > days_in_month(year, month) {
        return Err(X509TimeError::new(
            X509TimeErrorKind::InvalidDay,
            value_offset + day_offset,
        ));
    }

    let hour = parse_u8(value, hour_offset, value_offset)?;
    if hour > 23 {
        return Err(X509TimeError::new(
            X509TimeErrorKind::InvalidHour,
            value_offset + hour_offset,
        ));
    }

    let minute = parse_u8(value, minute_offset, value_offset)?;
    if minute > 59 {
        return Err(X509TimeError::new(
            X509TimeErrorKind::InvalidMinute,
            value_offset + minute_offset,
        ));
    }

    let second = parse_u8(value, second_offset, value_offset)?;
    if second > 59 {
        return Err(X509TimeError::new(
            X509TimeErrorKind::InvalidSecond,
            value_offset + second_offset,
        ));
    }

    Ok(X509Time {
        encoding,
        year,
        month,
        day,
        hour,
        minute,
        second,
    })
}

fn parse_u8(value: &[u8], start: usize, value_offset: usize) -> Result<u8, X509TimeError> {
    let parsed = parse_digits(value, start, 2, value_offset)?;
    u8::try_from(parsed)
        .map_err(|_| X509TimeError::new(X509TimeErrorKind::NonDigit, value_offset + start))
}

fn parse_digits(
    value: &[u8],
    start: usize,
    length: usize,
    value_offset: usize,
) -> Result<u16, X509TimeError> {
    let mut parsed = 0u16;
    for (offset, byte) in value.iter().copied().enumerate().skip(start).take(length) {
        if !byte.is_ascii_digit() {
            return Err(X509TimeError::new(
                X509TimeErrorKind::NonDigit,
                value_offset + offset,
            ));
        }
        parsed = parsed * 10 + u16::from(byte - b'0');
    }
    Ok(parsed)
}

const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

const fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}
