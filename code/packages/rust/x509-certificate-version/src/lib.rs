//! # RFC 5280 certificate versions
//!
//! This crate decodes only the optional explicit certificate-version wrapper.
//! It owns neither the surrounding certificate schema nor version-dependent
//! field policy.

#![deny(unsafe_code)]

use der_asn1::{decode_integer, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind};
use std::error::Error;
use std::fmt;

/// One RFC 5280 certificate version.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum X509CertificateVersion {
    /// The default when the version field is absent.
    #[default]
    V1,
    V2,
    V3,
}

impl X509CertificateVersion {
    /// Return the ASN.1 integer assigned by RFC 5280.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::V1 => 0,
            Self::V2 => 1,
            Self::V3 => 2,
        }
    }
}

/// Stable failure categories. No category retains source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificateVersionErrorKind {
    Structure(Asn1ErrorKind),
    InvalidInteger(Asn1ErrorKind),
    ExplicitDefault,
    UnsupportedVersion,
}

/// A redacted failure at an offset local to the explicit wrapper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificateVersionError {
    kind: X509CertificateVersionErrorKind,
    offset: usize,
}

impl X509CertificateVersionError {
    const fn new(kind: X509CertificateVersionErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn wrapper(error: Asn1Error, value_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => error.offset(),
        };
        Self::new(
            X509CertificateVersionErrorKind::Structure(error.kind()),
            offset,
        )
    }

    fn integer(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509CertificateVersionErrorKind::InvalidInteger(error.kind()),
            child_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> X509CertificateVersionErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509CertificateVersionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate version error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509CertificateVersionError {}

/// Decode one present `[0] EXPLICIT Version` field.
///
/// An absent field is not passed to this function and defaults to
/// [`X509CertificateVersion::V1`]. DER requires that default value to be
/// omitted, so an explicitly encoded `v1` is rejected.
pub fn decode_explicit_x509_certificate_version(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'_>,
) -> Result<X509CertificateVersion, X509CertificateVersionError> {
    let value_offset = element.header().len();
    let child = decoder
        .explicit(element, 0)
        .map_err(|error| X509CertificateVersionError::wrapper(error, value_offset))?;
    let child_offset = value_offset;
    let integer = decode_integer(child)
        .map_err(|error| X509CertificateVersionError::integer(error, child_offset))?;
    let integer_offset = child_offset + child.header().len();
    let value = integer
        .to_u64()
        .map_err(|error| X509CertificateVersionError::integer(error, child_offset))?;

    match value {
        0 => Err(X509CertificateVersionError::new(
            X509CertificateVersionErrorKind::ExplicitDefault,
            integer_offset,
        )),
        1 => Ok(X509CertificateVersion::V2),
        2 => Ok(X509CertificateVersion::V3),
        _ => Err(X509CertificateVersionError::new(
            X509CertificateVersionErrorKind::UnsupportedVersion,
            integer_offset,
        )),
    }
}
