//! Allocation-free TLS 1.3 alert message framing and disposition.
//!
//! RFC 8446 section 6 gives every alert exactly two octets: one legacy level
//! and one description. TLS 1.3 derives severity from the description, not
//! the received level. This crate keeps that asymmetry explicit: decoding
//! preserves every wire value, while encoding admits only standard
//! descriptions and requires `fatal` for standard error alerts.
//!
//! ```
//! use tls13_alert_message::{
//!     decode_alert_message, encode_alert_message, AlertDescription,
//!     AlertDisposition, AlertLevel,
//! };
//!
//! let received = decode_alert_message(&[1, 0])?;
//! assert_eq!(received.disposition(), AlertDisposition::CloseNotify);
//!
//! let mut output = [0xa5; 4];
//! let used = encode_alert_message(
//!     AlertLevel::Fatal,
//!     AlertDescription::DECODE_ERROR,
//!     &mut output,
//! )?;
//! assert_eq!(&output[..used], &[2, 50]);
//! assert_eq!(&output[used..], &[0xa5, 0xa5]);
//! # Ok::<(), tls13_alert_message::AlertError>(())
//! ```

#![forbid(unsafe_code)]

use core::fmt;

/// The exact encoded length of one RFC 8446 alert message.
pub const ALERT_MESSAGE_LENGTH: usize = 2;

/// A standard legacy alert level available to an encoder.
///
/// A decoder deliberately retains the received level as an unclassified byte:
/// TLS 1.3 says that byte can safely be ignored when deciding severity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum AlertLevel {
    /// The legacy warning level.
    Warning = 1,
    /// The legacy fatal level.
    Fatal = 2,
}

impl AlertLevel {
    /// Return the RFC wire value.
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }
}

/// One received alert's TLS 1.3 disposition.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AlertDisposition {
    /// Orderly closure of the sender's write side.
    CloseNotify,
    /// User cancellation unrelated to a protocol failure.
    UserCanceled,
    /// Abortive closure, including every unknown description.
    Error,
}

/// One exact alert-description wire value.
///
/// The private representation prevents callers from confusing a named
/// standard constant with a differently labelled value. `from_wire` remains
/// total because TLS 1.3 requires unknown descriptions to be treated as errors.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct AlertDescription(u8);

impl AlertDescription {
    /// Orderly closure of one connection direction.
    pub const CLOSE_NOTIFY: Self = Self(0);
    /// An inappropriate protocol message was received.
    pub const UNEXPECTED_MESSAGE: Self = Self(10);
    /// Record deprotection failed.
    pub const BAD_RECORD_MAC: Self = Self(20);
    /// A record exceeded the protocol or negotiated limit.
    pub const RECORD_OVERFLOW: Self = Self(22);
    /// No acceptable security-parameter set was negotiated.
    pub const HANDSHAKE_FAILURE: Self = Self(40);
    /// A certificate was corrupt or failed verification.
    pub const BAD_CERTIFICATE: Self = Self(42);
    /// A certificate type was unsupported.
    pub const UNSUPPORTED_CERTIFICATE: Self = Self(43);
    /// A certificate was revoked.
    pub const CERTIFICATE_REVOKED: Self = Self(44);
    /// A certificate was expired or not yet valid.
    pub const CERTIFICATE_EXPIRED: Self = Self(45);
    /// An otherwise unspecified certificate failure occurred.
    pub const CERTIFICATE_UNKNOWN: Self = Self(46);
    /// A syntactically valid field was semantically invalid.
    pub const ILLEGAL_PARAMETER: Self = Self(47);
    /// No acceptable trust anchor was found.
    pub const UNKNOWN_CA: Self = Self(48);
    /// Access control rejected an otherwise valid credential.
    pub const ACCESS_DENIED: Self = Self(49);
    /// Protocol syntax or a length was invalid.
    pub const DECODE_ERROR: Self = Self(50);
    /// A handshake cryptographic operation failed.
    pub const DECRYPT_ERROR: Self = Self(51);
    /// The attempted protocol version was unsupported.
    pub const PROTOCOL_VERSION: Self = Self(70);
    /// Negotiation did not meet the required security level.
    pub const INSUFFICIENT_SECURITY: Self = Self(71);
    /// An internal failure occurred.
    pub const INTERNAL_ERROR: Self = Self(80);
    /// A prohibited fallback was detected.
    pub const INAPPROPRIATE_FALLBACK: Self = Self(86);
    /// A user canceled the handshake without a protocol failure.
    pub const USER_CANCELED: Self = Self(90);
    /// A mandatory extension was absent.
    pub const MISSING_EXTENSION: Self = Self(109);
    /// A prohibited or unsupported extension was received.
    pub const UNSUPPORTED_EXTENSION: Self = Self(110);
    /// No certificate name matched the requested name.
    pub const UNRECOGNIZED_NAME: Self = Self(112);
    /// A certificate-status response was invalid.
    pub const BAD_CERTIFICATE_STATUS_RESPONSE: Self = Self(113);
    /// A PSK identity was unknown.
    pub const UNKNOWN_PSK_IDENTITY: Self = Self(115);
    /// A client certificate was required but unavailable.
    pub const CERTIFICATE_REQUIRED: Self = Self(116);
    /// No application protocol could be negotiated.
    pub const NO_APPLICATION_PROTOCOL: Self = Self(120);

    /// Retain any received description value, including an unknown one.
    #[must_use]
    pub const fn from_wire(value: u8) -> Self {
        Self(value)
    }

    /// Return the exact received or standard wire value.
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self.0
    }

    /// Return whether RFC 8446 section 6 names this description.
    #[must_use]
    pub const fn is_standard(self) -> bool {
        matches!(
            self.0,
            0 | 10
                | 20
                | 22
                | 40
                | 42
                | 43
                | 44
                | 45
                | 46
                | 47
                | 48
                | 49
                | 50
                | 51
                | 70
                | 71
                | 80
                | 86
                | 90
                | 109
                | 110
                | 112
                | 113
                | 115
                | 116
                | 120
        )
    }

    /// Classify this description under TLS 1.3 rules.
    ///
    /// Unknown descriptions fail closed as errors. The legacy level is not an
    /// input because RFC 8446 requires error classification regardless of it.
    #[must_use]
    pub const fn disposition(self) -> AlertDisposition {
        match self.0 {
            0 => AlertDisposition::CloseNotify,
            90 => AlertDisposition::UserCanceled,
            _ => AlertDisposition::Error,
        }
    }
}

impl fmt::Debug for AlertDescription {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AlertDescription")
            .field("standard", &self.is_standard())
            .field("disposition", &self.disposition())
            .finish_non_exhaustive()
    }
}

/// One decoded two-octet TLS 1.3 alert.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ReceivedAlert {
    legacy_level: u8,
    description: AlertDescription,
}

impl ReceivedAlert {
    /// Return the exact untrusted legacy level byte.
    #[must_use]
    pub const fn legacy_level(&self) -> u8 {
        self.legacy_level
    }

    /// Return a named level only when the peer sent a standard value.
    #[must_use]
    pub const fn recognized_legacy_level(&self) -> Option<AlertLevel> {
        match self.legacy_level {
            1 => Some(AlertLevel::Warning),
            2 => Some(AlertLevel::Fatal),
            _ => None,
        }
    }

    /// Return the exact recognized-or-unknown description value.
    #[must_use]
    pub const fn description(&self) -> AlertDescription {
        self.description
    }

    /// Return the TLS 1.3 disposition, ignoring the unreliable legacy level.
    #[must_use]
    pub const fn disposition(&self) -> AlertDisposition {
        self.description.disposition()
    }
}

/// Stable, payload-free alert framing and encoding failures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AlertErrorKind {
    /// A decoded value was shorter or longer than two octets.
    InvalidMessageLength,
    /// An encoder was asked to send an unknown description.
    UnknownDescriptionForEncoding,
    /// A standard error alert was not assigned the required fatal level.
    ErrorAlertMustBeFatal,
    /// Caller-owned output could not hold the complete two-octet alert.
    OutputTooSmall,
}

/// One redacted alert failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AlertError {
    kind: AlertErrorKind,
}

impl AlertError {
    const fn new(kind: AlertErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(self) -> AlertErrorKind {
        self.kind
    }
}

impl fmt::Display for AlertError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TLS 1.3 alert error {:?}", self.kind)
    }
}

impl std::error::Error for AlertError {}

/// Decode exactly one complete RFC 8446 section 6 alert message.
///
/// Every description byte is accepted because an unknown description has the
/// defined fail-closed error disposition. Every level byte is retained but is
/// excluded from disposition classification.
pub fn decode_alert_message(input: &[u8]) -> Result<ReceivedAlert, AlertError> {
    let [legacy_level, description] = input else {
        return Err(AlertError::new(AlertErrorKind::InvalidMessageLength));
    };

    Ok(ReceivedAlert {
        legacy_level: *legacy_level,
        description: AlertDescription::from_wire(*description),
    })
}

/// Encode one standard TLS 1.3 alert into caller-owned output.
///
/// Section 6.2 error alerts require the fatal legacy level. Closure alerts may
/// retain either standard level because their disposition comes from the
/// description. Validation completes before output is changed.
pub fn encode_alert_message(
    level: AlertLevel,
    description: AlertDescription,
    output: &mut [u8],
) -> Result<usize, AlertError> {
    if !description.is_standard() {
        return Err(AlertError::new(
            AlertErrorKind::UnknownDescriptionForEncoding,
        ));
    }
    if description.disposition() == AlertDisposition::Error && level != AlertLevel::Fatal {
        return Err(AlertError::new(AlertErrorKind::ErrorAlertMustBeFatal));
    }
    if output.len() < ALERT_MESSAGE_LENGTH {
        return Err(AlertError::new(AlertErrorKind::OutputTooSmall));
    }

    output[0] = level.wire_value();
    output[1] = description.wire_value();
    Ok(ALERT_MESSAGE_LENGTH)
}
