//! # Data-driven RFC 5280 certificate policy gates
//!
//! This crate applies certificate-wide policy to already decoded values. It
//! deliberately embeds no general extension registry, algorithm policy, trust,
//! cryptographic authority, clock, TLS, or network. Its role gate enforces only
//! algorithm-independent relationships among Basic Constraints, Key Usage,
//! and Extended Key Usage.

#![deny(unsafe_code)]

use der_asn1::{Asn1Decoder, Asn1Limits, ObjectIdentifier};
use std::error::Error;
use std::fmt;
use x509_basic_constraints::{decode_basic_constraints, BasicConstraints, BASIC_CONSTRAINTS_OID};
use x509_certificate::X509Certificate;
use x509_extended_key_usage::{
    decode_extended_key_usage, ExtendedKeyUsage, EXTENDED_KEY_USAGE_OID,
};
use x509_key_usage::{decode_key_usage, KeyUsage, KeyUsageBit, KEY_USAGE_OID};

/// Stable certificate-policy failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificatePolicyErrorKind {
    /// A critical extension was not recognized by the caller's policy.
    UnsupportedCriticalExtension,
}

/// A redacted certificate-policy failure.
///
/// `extension_index` is the zero-based index in certificate wire order. The
/// error intentionally retains neither the extension OID nor certificate bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificatePolicyError {
    kind: X509CertificatePolicyErrorKind,
    extension_index: usize,
}

impl X509CertificatePolicyError {
    const fn unsupported_critical_extension(extension_index: usize) -> Self {
        Self {
            kind: X509CertificatePolicyErrorKind::UnsupportedCriticalExtension,
            extension_index,
        }
    }

    pub const fn kind(self) -> X509CertificatePolicyErrorKind {
        self.kind
    }

    pub const fn extension_index(self) -> usize {
        self.extension_index
    }
}

impl fmt::Display for X509CertificatePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate policy error {:?} at extension index {}",
            self.kind, self.extension_index
        )
    }
}

impl Error for X509CertificatePolicyError {}

/// One caller-selected, algorithm-independent certificate role.
///
/// Passing a role is a request to check the extension relationships for that
/// role. It does not grant the role or establish certificate trust.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificateRole {
    EndEntity,
    IssuingCertificateAuthority,
}

/// Stable certificate-role failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificateRoleErrorKind {
    InvalidBasicConstraints,
    InvalidKeyUsage,
    InvalidExtendedKeyUsage,
    MissingBasicConstraints,
    BasicConstraintsNotCritical,
    CertificateAuthorityNotAsserted,
    MissingKeyUsage,
    CertificateSigningNotAsserted,
    EndEntityAssertsCertificateAuthority,
    EndEntityAssertsCertificateSigning,
}

/// A redacted certificate-role failure.
///
/// `extension_index` is present only when the failure belongs to one extension
/// in certificate wire order. The error retains no OID or input bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificateRoleError {
    kind: X509CertificateRoleErrorKind,
    extension_index: Option<usize>,
}

impl X509CertificateRoleError {
    const fn new(kind: X509CertificateRoleErrorKind, extension_index: Option<usize>) -> Self {
        Self {
            kind,
            extension_index,
        }
    }

    pub const fn kind(self) -> X509CertificateRoleErrorKind {
        self.kind
    }

    pub const fn extension_index(self) -> Option<usize> {
        self.extension_index
    }
}

impl fmt::Display for X509CertificateRoleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.extension_index {
            Some(index) => write!(
                formatter,
                "X.509 certificate role error {:?} at extension index {}",
                self.kind, index
            ),
            None => write!(formatter, "X.509 certificate role error {:?}", self.kind),
        }
    }
}

impl Error for X509CertificateRoleError {}

/// Borrowed role-extension evidence after algorithm-independent checks.
///
/// This value is not evidence of a valid path, trusted CA, authenticated
/// server, permitted purpose, or usable public key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedCertificateRoleExtensions<'a> {
    role: X509CertificateRole,
    basic_constraints: Option<BasicConstraints<'a>>,
    key_usage: Option<KeyUsage>,
    extended_key_usage: Option<ExtendedKeyUsage<'a>>,
}

impl<'a> ValidatedCertificateRoleExtensions<'a> {
    pub const fn role(&self) -> X509CertificateRole {
        self.role
    }

    pub const fn basic_constraints(&self) -> Option<BasicConstraints<'a>> {
        self.basic_constraints
    }

    pub const fn key_usage(&self) -> Option<KeyUsage> {
        self.key_usage
    }

    pub fn extended_key_usage(&self) -> Option<&ExtendedKeyUsage<'a>> {
        self.extended_key_usage.as_ref()
    }
}

/// Require caller recognition of every critical certificate extension.
///
/// The recognizer is called exactly once for each critical extension, in wire
/// order, and is not called for non-critical extensions. Evaluation stops at
/// the first rejected critical extension. Returning `true` means only that the
/// caller recognizes the OID; extension-specific decoding and semantic policy
/// remain separate mandatory steps.
pub fn validate_critical_extensions<'a>(
    certificate: &X509Certificate<'a>,
    mut recognizes: impl FnMut(ObjectIdentifier<'a>) -> bool,
) -> Result<(), X509CertificatePolicyError> {
    let Some(extensions) = certificate.tbs_certificate().extensions() else {
        return Ok(());
    };

    for (extension_index, extension) in extensions.iter().enumerate() {
        if extension.critical() && !recognizes(extension.extension_id()) {
            return Err(X509CertificatePolicyError::unsupported_critical_extension(
                extension_index,
            ));
        }
    }

    Ok(())
}

/// Decode and validate algorithm-independent certificate-role extensions.
///
/// One decoder shares `limits` across every present Basic Constraints, Key
/// Usage, and Extended Key Usage value in certificate wire order. Issuing CAs
/// require critical `cA = TRUE` and `keyCertSign`; end entities must assert
/// neither. Extended Key Usage is decoded but deliberately uninterpreted.
///
/// This gate does not replace [`validate_critical_extensions`]. Callers must
/// separately reject unsupported critical extensions and semantically validate
/// every other recognized critical extension before assigning trust.
pub fn validate_certificate_role_extensions<'a>(
    certificate: &X509Certificate<'a>,
    role: X509CertificateRole,
    limits: Asn1Limits,
) -> Result<ValidatedCertificateRoleExtensions<'a>, X509CertificateRoleError> {
    let mut decoder = Asn1Decoder::new(limits);
    let mut basic_constraints = None;
    let mut key_usage = None;
    let mut extended_key_usage = None;

    if let Some(extensions) = certificate.tbs_certificate().extensions() {
        for (extension_index, extension) in extensions.iter().enumerate() {
            let extension_id = extension.extension_id();
            if extension_id.equals(BASIC_CONSTRAINTS_OID) {
                let value = decode_basic_constraints(&mut decoder, extension).map_err(|_| {
                    X509CertificateRoleError::new(
                        X509CertificateRoleErrorKind::InvalidBasicConstraints,
                        Some(extension_index),
                    )
                })?;
                basic_constraints = Some((extension_index, extension.critical(), value));
            } else if extension_id.equals(KEY_USAGE_OID) {
                let value = decode_key_usage(&mut decoder, extension).map_err(|_| {
                    X509CertificateRoleError::new(
                        X509CertificateRoleErrorKind::InvalidKeyUsage,
                        Some(extension_index),
                    )
                })?;
                key_usage = Some((extension_index, value));
            } else if extension_id.equals(EXTENDED_KEY_USAGE_OID) {
                let value = decode_extended_key_usage(&mut decoder, extension).map_err(|_| {
                    X509CertificateRoleError::new(
                        X509CertificateRoleErrorKind::InvalidExtendedKeyUsage,
                        Some(extension_index),
                    )
                })?;
                extended_key_usage = Some((extension_index, value));
            }
        }
    }

    match role {
        X509CertificateRole::IssuingCertificateAuthority => {
            let Some((extension_index, critical, constraints)) = basic_constraints.as_ref() else {
                return Err(X509CertificateRoleError::new(
                    X509CertificateRoleErrorKind::MissingBasicConstraints,
                    None,
                ));
            };
            if !critical {
                return Err(X509CertificateRoleError::new(
                    X509CertificateRoleErrorKind::BasicConstraintsNotCritical,
                    Some(*extension_index),
                ));
            }
            if !constraints.is_ca() {
                return Err(X509CertificateRoleError::new(
                    X509CertificateRoleErrorKind::CertificateAuthorityNotAsserted,
                    Some(*extension_index),
                ));
            }

            let Some((extension_index, usage)) = key_usage.as_ref() else {
                return Err(X509CertificateRoleError::new(
                    X509CertificateRoleErrorKind::MissingKeyUsage,
                    None,
                ));
            };
            if !usage.contains(KeyUsageBit::KeyCertSign) {
                return Err(X509CertificateRoleError::new(
                    X509CertificateRoleErrorKind::CertificateSigningNotAsserted,
                    Some(*extension_index),
                ));
            }
        }
        X509CertificateRole::EndEntity => {
            if let Some((extension_index, _, constraints)) = basic_constraints.as_ref() {
                if constraints.is_ca() {
                    return Err(X509CertificateRoleError::new(
                        X509CertificateRoleErrorKind::EndEntityAssertsCertificateAuthority,
                        Some(*extension_index),
                    ));
                }
            }
            if let Some((extension_index, usage)) = key_usage.as_ref() {
                if usage.contains(KeyUsageBit::KeyCertSign) {
                    return Err(X509CertificateRoleError::new(
                        X509CertificateRoleErrorKind::EndEntityAssertsCertificateSigning,
                        Some(*extension_index),
                    ));
                }
            }
        }
    }

    Ok(ValidatedCertificateRoleExtensions {
        role,
        basic_constraints: basic_constraints.map(|(_, _, value)| value),
        key_usage: key_usage.map(|(_, value)| value),
        extended_key_usage: extended_key_usage.map(|(_, value)| value),
    })
}
