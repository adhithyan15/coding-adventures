//! # RFC 5280 certificate serial numbers
//!
//! This crate decodes only one certificate serial-number INTEGER. It owns no
//! issuer state, certificate schema, path policy, or cryptographic authority.

#![deny(unsafe_code)]

use der_asn1::{decode_integer, Asn1Element, Asn1Error, Asn1ErrorKind};
use std::error::Error;
use std::fmt;

/// RFC 5280's maximum certificate serial-number content length.
pub const MAX_SERIAL_NUMBER_OCTETS: usize = 20;

/// Stable failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509SerialNumberErrorKind {
    Structure(Asn1ErrorKind),
    NotPositive,
    TooLong,
}

/// A redacted failure at an offset local to the serial-number element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509SerialNumberError {
    kind: X509SerialNumberErrorKind,
    offset: usize,
}

impl X509SerialNumberError {
    const fn new(kind: X509SerialNumberErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509SerialNumberErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    pub const fn kind(self) -> X509SerialNumberErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509SerialNumberError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 serial number error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509SerialNumberError {}

/// One canonical positive nonzero RFC 5280 certificate serial number.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct X509SerialNumber<'a> {
    encoded_value: &'a [u8],
    magnitude: &'a [u8],
}

impl<'a> X509SerialNumber<'a> {
    /// Return the canonical DER INTEGER contents, including a required sign
    /// octet when the magnitude's high bit is set.
    pub const fn encoded_value(self) -> &'a [u8] {
        self.encoded_value
    }

    /// Return the unsigned magnitude without a DER sign-protection octet.
    pub const fn magnitude(self) -> &'a [u8] {
        self.magnitude
    }

    /// Return the number of content octets counted by the RFC 5280 limit.
    pub const fn encoded_len(self) -> usize {
        self.encoded_value.len()
    }
}

impl fmt::Debug for X509SerialNumber<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("X509SerialNumber")
            .field("encoded_len", &self.encoded_value.len())
            .field("magnitude_len", &self.magnitude.len())
            .finish()
    }
}

/// Decode one exact RFC 5280 certificate serial-number value.
pub fn decode_x509_serial_number(
    element: Asn1Element<'_>,
) -> Result<X509SerialNumber<'_>, X509SerialNumberError> {
    let value_offset = element.header().len();
    let integer = decode_integer(element).map_err(X509SerialNumberError::structure)?;
    let encoded_value = integer.signed_bytes();

    if integer.is_negative() || encoded_value == [0] {
        return Err(X509SerialNumberError::new(
            X509SerialNumberErrorKind::NotPositive,
            value_offset,
        ));
    }
    if encoded_value.len() > MAX_SERIAL_NUMBER_OCTETS {
        return Err(X509SerialNumberError::new(
            X509SerialNumberErrorKind::TooLong,
            value_offset + MAX_SERIAL_NUMBER_OCTETS,
        ));
    }

    let magnitude = encoded_value.strip_prefix(&[0]).unwrap_or(encoded_value);
    Ok(X509SerialNumber {
        encoded_value,
        magnitude,
    })
}
