//! # RFC 5280 certificate Validity values
//!
//! This crate decodes only the two-time `Validity` sequence. It deliberately
//! owns neither a clock nor the rest of the certificate schema.

#![deny(unsafe_code)]

use core::cmp::Ordering;
use der_asn1::{Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind};
use std::error::Error;
use std::fmt;
use x509_time::{decode_x509_time, X509Time, X509TimeError, X509TimeErrorKind};

/// Stable failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509ValidityErrorKind {
    Structure(Asn1ErrorKind),
    MissingNotBefore,
    MissingNotAfter,
    TrailingElement,
    InvalidNotBefore(X509TimeErrorKind),
    InvalidNotAfter(X509TimeErrorKind),
    InvertedRange,
}

/// A redacted validity failure at a byte offset local to the Validity element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509ValidityError {
    kind: X509ValidityErrorKind,
    offset: usize,
}

impl X509ValidityError {
    const fn new(kind: X509ValidityErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509ValidityErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(X509ValidityErrorKind::Structure(error.kind()), offset)
    }

    fn time(error: X509TimeError, child_offset: usize, not_before: bool) -> Self {
        let kind = if not_before {
            X509ValidityErrorKind::InvalidNotBefore(error.kind())
        } else {
            X509ValidityErrorKind::InvalidNotAfter(error.kind())
        };
        Self::new(kind, child_offset + error.offset())
    }

    pub const fn kind(self) -> X509ValidityErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509ValidityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 validity error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509ValidityError {}

/// Classification of an explicit caller-supplied time against the interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509ValidityStatus {
    NotYetValid,
    Valid,
    Expired,
}

/// One ordered, inclusive RFC 5280 certificate validity interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509Validity {
    not_before: X509Time,
    not_after: X509Time,
}

impl X509Validity {
    pub const fn not_before(self) -> X509Time {
        self.not_before
    }

    pub const fn not_after(self) -> X509Time {
        self.not_after
    }

    /// Classify one already-validated explicit time using inclusive endpoints.
    pub fn classify(&self, observed: &X509Time) -> X509ValidityStatus {
        if observed.cmp_fields(&self.not_before) == Ordering::Less {
            X509ValidityStatus::NotYetValid
        } else if observed.cmp_fields(&self.not_after) == Ordering::Greater {
            X509ValidityStatus::Expired
        } else {
            X509ValidityStatus::Valid
        }
    }
}

/// Decode one exact RFC 5280 `Validity` sequence.
pub fn decode_x509_validity(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'_>,
) -> Result<X509Validity, X509ValidityError> {
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .sequence(element)
        .map_err(X509ValidityError::structure)?;

    let not_before_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let not_before_element = fields
        .read(decoder)
        .map_err(|error| {
            X509ValidityError::child_structure(error, value_offset, not_before_offset)
        })?
        .ok_or_else(|| {
            X509ValidityError::new(X509ValidityErrorKind::MissingNotBefore, not_before_offset)
        })?;
    let not_before = decode_x509_time(not_before_element)
        .map_err(|error| X509ValidityError::time(error, not_before_offset, true))?;

    let not_after_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let not_after_element = fields
        .read(decoder)
        .map_err(|error| X509ValidityError::child_structure(error, value_offset, not_after_offset))?
        .ok_or_else(|| {
            X509ValidityError::new(X509ValidityErrorKind::MissingNotAfter, not_after_offset)
        })?;
    let not_after = decode_x509_time(not_after_element)
        .map_err(|error| X509ValidityError::time(error, not_after_offset, false))?;

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| X509ValidityError::child_structure(error, value_offset, trailing_offset))?
        .is_some()
    {
        return Err(X509ValidityError::new(
            X509ValidityErrorKind::TrailingElement,
            trailing_offset,
        ));
    }
    fields.finish().map_err(X509ValidityError::structure)?;

    if not_after.cmp_fields(&not_before) == Ordering::Less {
        return Err(X509ValidityError::new(
            X509ValidityErrorKind::InvertedRange,
            not_after_offset,
        ));
    }

    Ok(X509Validity {
        not_before,
        not_after,
    })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
