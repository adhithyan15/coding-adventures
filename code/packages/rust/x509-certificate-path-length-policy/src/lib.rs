//! # Bounded RFC 5280 certificate path-length policy
//!
//! This crate applies only the path-length state transition to caller-ordered
//! intermediate issuing-CA role evidence. It does not decide self-issuance,
//! construct or authenticate a path, process a trust anchor, or grant trust.

#![deny(unsafe_code)]

use std::error::Error;
use std::fmt;
use x509_certificate_policy::{ValidatedCertificateRoleExtensions, X509CertificateRole};

/// Stable path-length failure categories. No category retains input data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509CertificatePathLengthErrorKind {
    NotIssuingCertificateAuthority,
    MaximumExceeded,
}

/// A redacted certificate path-length failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509CertificatePathLengthError {
    kind: X509CertificatePathLengthErrorKind,
}

impl X509CertificatePathLengthError {
    const fn new(kind: X509CertificatePathLengthErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509CertificatePathLengthErrorKind {
        self.kind
    }
}

impl fmt::Display for X509CertificatePathLengthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate path length policy error {:?}",
            self.kind
        )
    }
}

impl Error for X509CertificatePathLengthError {}

/// Remaining non-self-issued intermediate-CA capacity.
///
/// Start this state with an explicit caller-owned maximum, then consume it in
/// path order from the trust-anchor side toward the target. The target and
/// trust anchor are outside this transition. This state proves no path link,
/// signature, identity, validity, trust, revocation, TLS, or network result.
#[derive(Debug, PartialEq, Eq)]
pub struct X509CertificatePathLengthState {
    remaining_non_self_issued_intermediate_cas: u64,
}

impl X509CertificatePathLengthState {
    pub const fn new(maximum_non_self_issued_intermediate_cas: u64) -> Self {
        Self {
            remaining_non_self_issued_intermediate_cas: maximum_non_self_issued_intermediate_cas,
        }
    }

    pub const fn remaining_non_self_issued_intermediate_cas(&self) -> u64 {
        self.remaining_non_self_issued_intermediate_cas
    }

    /// Advance across one intermediate CA in trust-anchor-to-target order.
    ///
    /// `is_self_issued` is an explicit caller-owned classification. Following
    /// RFC 5280 section 6.1.4, a non-self-issued intermediate first consumes
    /// one unit and the certificate's pathLenConstraint then tightens the
    /// remaining maximum. A self-issued intermediate skips only the decrement.
    pub fn advance_intermediate_ca(
        self,
        role_evidence: &ValidatedCertificateRoleExtensions<'_>,
        is_self_issued: bool,
    ) -> Result<Self, X509CertificatePathLengthError> {
        if role_evidence.role() != X509CertificateRole::IssuingCertificateAuthority {
            return Err(X509CertificatePathLengthError::new(
                X509CertificatePathLengthErrorKind::NotIssuingCertificateAuthority,
            ));
        }

        let mut remaining = self.remaining_non_self_issued_intermediate_cas;
        if !is_self_issued {
            remaining = remaining.checked_sub(1).ok_or_else(|| {
                X509CertificatePathLengthError::new(
                    X509CertificatePathLengthErrorKind::MaximumExceeded,
                )
            })?;
        }

        if let Some(constraint) = role_evidence
            .basic_constraints()
            .and_then(|constraints| constraints.path_len_constraint())
        {
            // A canonical non-negative constraint larger than u64 cannot be
            // less than this u64 state, so it cannot tighten the current cap.
            if let Ok(constraint) = constraint.to_u64() {
                remaining = remaining.min(constraint);
            }
        }

        Ok(Self {
            remaining_non_self_issued_intermediate_cas: remaining,
        })
    }
}
