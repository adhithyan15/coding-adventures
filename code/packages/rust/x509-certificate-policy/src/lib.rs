//! # Data-driven RFC 5280 certificate policy gates
//!
//! This crate applies certificate-wide policy to already decoded values. It
//! deliberately embeds no extension registry, certificate-role rules,
//! algorithm policy, trust, cryptographic authority, clock, TLS, or network.

#![deny(unsafe_code)]

use der_asn1::ObjectIdentifier;
use std::error::Error;
use std::fmt;
use x509_certificate::X509Certificate;

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
