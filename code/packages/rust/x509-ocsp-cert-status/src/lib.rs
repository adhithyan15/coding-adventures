//! # RFC 6960 OCSP certificate status
//!
//! One OCSP `SingleResponse` reports a certificate status through a tagged
//! choice. This crate decodes only that choice:
//!
//! ```text
//! good    [0] IMPLICIT NULL
//! revoked [1] IMPLICIT RevokedInfo
//! unknown [2] IMPLICIT NULL
//! ```
//!
//! The revoked arm contains an OCSP GeneralizedTime and an optional explicitly
//! tagged RFC 5280 CRL reason. Parsing that shape neither authenticates the
//! response nor proves that it applies to any certificate.

#![deny(unsafe_code)]

use der_asn1::{
    decode_enumerated, decode_implicit_null, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind,
};
use der_tlv::TagClass;
use std::error::Error;
use std::fmt;
use x509_time::{decode_x509_generalized_time, X509Time, X509TimeError, X509TimeErrorKind};

/// One RFC 5280 CRL reason recognized by RFC 6960 `RevokedInfo`.
///
/// Seven is intentionally absent from the registry. Values are exposed through
/// `as_u8` so callers can serialize policy or telemetry without relying on Rust
/// enum representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509OcspRevocationReason {
    Unspecified,
    KeyCompromise,
    CaCompromise,
    AffiliationChanged,
    Superseded,
    CessationOfOperation,
    CertificateHold,
    RemoveFromCrl,
    PrivilegeWithdrawn,
    AaCompromise,
}

impl X509OcspRevocationReason {
    /// Return the standards-assigned ENUMERATED value.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Unspecified => 0,
            Self::KeyCompromise => 1,
            Self::CaCompromise => 2,
            Self::AffiliationChanged => 3,
            Self::Superseded => 4,
            Self::CessationOfOperation => 5,
            Self::CertificateHold => 6,
            Self::RemoveFromCrl => 8,
            Self::PrivilegeWithdrawn => 9,
            Self::AaCompromise => 10,
        }
    }

    const fn from_u64(value: u64) -> Option<Self> {
        match value {
            0 => Some(Self::Unspecified),
            1 => Some(Self::KeyCompromise),
            2 => Some(Self::CaCompromise),
            3 => Some(Self::AffiliationChanged),
            4 => Some(Self::Superseded),
            5 => Some(Self::CessationOfOperation),
            6 => Some(Self::CertificateHold),
            8 => Some(Self::RemoveFromCrl),
            9 => Some(Self::PrivilegeWithdrawn),
            10 => Some(Self::AaCompromise),
            _ => None,
        }
    }
}

/// The validated fields of the `revoked [1]` choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509OcspRevokedInfo {
    revocation_time: X509Time,
    revocation_reason: Option<X509OcspRevocationReason>,
}

impl X509OcspRevokedInfo {
    /// Return the clock-free, validated OCSP revocation time.
    pub const fn revocation_time(self) -> X509Time {
        self.revocation_time
    }

    /// Return the optional standards-defined reason.
    pub const fn revocation_reason(self) -> Option<X509OcspRevocationReason> {
        self.revocation_reason
    }
}

/// One structurally valid RFC 6960 certificate-status choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509OcspCertStatus {
    Good,
    Revoked(X509OcspRevokedInfo),
    Unknown,
}

/// Stable failure categories. No category retains response bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509OcspCertStatusErrorKind {
    Structure(Asn1ErrorKind),
    MissingRevocationTime,
    InvalidRevocationTime(X509TimeErrorKind),
    InvalidRevocationReason(Asn1ErrorKind),
    UnsupportedRevocationReason,
    TrailingElement,
}

/// A redacted failure at a byte offset local to the `CertStatus` element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509OcspCertStatusError {
    kind: X509OcspCertStatusErrorKind,
    offset: usize,
}

impl X509OcspCertStatusError {
    const fn new(kind: X509OcspCertStatusErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            X509OcspCertStatusErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(X509OcspCertStatusErrorKind::Structure(error.kind()), offset)
    }

    fn revocation_time(error: X509TimeError, child_offset: usize) -> Self {
        Self::new(
            X509OcspCertStatusErrorKind::InvalidRevocationTime(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn reason_wrapper(
        error: Asn1Error,
        wrapper_offset: usize,
        wrapper_value_offset: usize,
    ) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => wrapper_value_offset + error.offset(),
            _ => wrapper_offset + error.offset(),
        };
        Self::new(
            X509OcspCertStatusErrorKind::InvalidRevocationReason(error.kind()),
            offset,
        )
    }

    fn reason_value(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            X509OcspCertStatusErrorKind::InvalidRevocationReason(error.kind()),
            child_offset + error.offset(),
        )
    }

    /// Return the stable, payload-free failure category.
    pub const fn kind(self) -> X509OcspCertStatusErrorKind {
        self.kind
    }

    /// Return the failing byte offset relative to the `CertStatus` element.
    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for X509OcspCertStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP certificate status error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for X509OcspCertStatusError {}

/// Decode one exact RFC 6960 `CertStatus` choice.
///
/// The supplied decoder owns the shared DER depth, element, and length budgets.
/// The result is syntax only: callers must separately authenticate and
/// authorize the response, bind it to the intended certificate, and apply
/// freshness and revocation policy.
pub fn decode_x509_ocsp_cert_status(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'_>,
) -> Result<X509OcspCertStatus, X509OcspCertStatusError> {
    let tag = element.tag();
    if tag.class != TagClass::ContextSpecific {
        return Err(X509OcspCertStatusError::new(
            X509OcspCertStatusErrorKind::Structure(Asn1ErrorKind::UnexpectedTag),
            0,
        ));
    }

    match (tag.constructed, tag.number) {
        (false, 0) => {
            decode_implicit_null(element, 0).map_err(X509OcspCertStatusError::structure)?;
            Ok(X509OcspCertStatus::Good)
        }
        (true, 1) => decode_revoked_info(decoder, element),
        (false, 2) => {
            decode_implicit_null(element, 2).map_err(X509OcspCertStatusError::structure)?;
            Ok(X509OcspCertStatus::Unknown)
        }
        _ => Err(X509OcspCertStatusError::new(
            X509OcspCertStatusErrorKind::Structure(Asn1ErrorKind::UnexpectedTag),
            0,
        )),
    }
}

fn decode_revoked_info(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'_>,
) -> Result<X509OcspCertStatus, X509OcspCertStatusError> {
    // IMPLICIT tagging replaces RevokedInfo's SEQUENCE identifier, so the
    // context-specific value contains its fields directly.
    let value_offset = element.header().len();
    let value_length = element.value().len();
    let mut fields = decoder
        .implicit_constructed(element, 1)
        .map_err(X509OcspCertStatusError::structure)?;

    let time_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let time_element = fields
        .read(decoder)
        .map_err(|error| {
            X509OcspCertStatusError::child_structure(error, value_offset, time_offset)
        })?
        .ok_or_else(|| {
            X509OcspCertStatusError::new(
                X509OcspCertStatusErrorKind::MissingRevocationTime,
                time_offset,
            )
        })?;
    let revocation_time = decode_x509_generalized_time(time_element)
        .map_err(|error| X509OcspCertStatusError::revocation_time(error, time_offset))?;

    let reason_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let reason_element = fields.read(decoder).map_err(|error| {
        X509OcspCertStatusError::child_structure(error, value_offset, reason_offset)
    })?;
    let revocation_reason = reason_element
        .map(|wrapper| decode_revocation_reason(decoder, wrapper, reason_offset))
        .transpose()?;

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            X509OcspCertStatusError::child_structure(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(X509OcspCertStatusError::new(
            X509OcspCertStatusErrorKind::TrailingElement,
            trailing_offset,
        ));
    }

    Ok(X509OcspCertStatus::Revoked(X509OcspRevokedInfo {
        revocation_time,
        revocation_reason,
    }))
}

fn decode_revocation_reason(
    decoder: &mut Asn1Decoder,
    wrapper: Asn1Element<'_>,
    wrapper_offset: usize,
) -> Result<X509OcspRevocationReason, X509OcspCertStatusError> {
    let child_offset = wrapper_offset + wrapper.header().len();
    let child = decoder.explicit(wrapper, 0).map_err(|error| {
        X509OcspCertStatusError::reason_wrapper(error, wrapper_offset, child_offset)
    })?;
    let enumerated = decode_enumerated(child)
        .map_err(|error| X509OcspCertStatusError::reason_value(error, child_offset))?;
    let value = enumerated
        .to_u64()
        .map_err(|error| X509OcspCertStatusError::reason_value(error, child_offset))?;

    X509OcspRevocationReason::from_u64(value).ok_or_else(|| {
        X509OcspCertStatusError::new(
            X509OcspCertStatusErrorKind::UnsupportedRevocationReason,
            child_offset + child.header().len(),
        )
    })
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
