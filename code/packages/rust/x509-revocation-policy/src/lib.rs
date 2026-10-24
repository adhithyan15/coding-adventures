//! # Fail-closed X.509 revocation-observation policy
//!
//! This crate classifies one explicit caller-supplied status observation at one
//! explicit validated time. It authenticates no observation and reads no clock.

#![deny(unsafe_code)]

use core::cmp::Ordering;
use core::fmt;
use x509_time::X509Time;

/// Caller-supplied certificate-status classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RevocationStatus {
    Good,
    Revoked,
    Unknown,
}

/// One unauthenticated caller-supplied revocation observation.
///
/// This value is policy input, not proof that an OCSP responder or CRL issuer
/// produced or signed the status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RevocationObservation {
    status: X509RevocationStatus,
    this_update: Option<X509Time>,
    next_update: Option<X509Time>,
}

impl X509RevocationObservation {
    pub const fn new(
        status: X509RevocationStatus,
        this_update: Option<X509Time>,
        next_update: Option<X509Time>,
    ) -> Self {
        Self {
            status,
            this_update,
            next_update,
        }
    }

    pub const fn status(self) -> X509RevocationStatus {
        self.status
    }

    pub const fn this_update(self) -> Option<X509Time> {
        self.this_update
    }

    pub const fn next_update(self) -> Option<X509Time> {
        self.next_update
    }
}

/// Stable payload-free revocation-policy failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RevocationPolicyErrorKind {
    MissingObservation,
    Revoked,
    Unknown,
    MissingThisUpdate,
    MissingNextUpdate,
    InvertedUpdateRange,
    FutureThisUpdate,
    StaleNextUpdate,
}

/// One payload-free revocation-policy failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RevocationPolicyError {
    kind: X509RevocationPolicyErrorKind,
}

impl X509RevocationPolicyError {
    const fn new(kind: X509RevocationPolicyErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509RevocationPolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RevocationPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "X.509 revocation policy error {:?}", self.kind)
    }
}

impl std::error::Error for X509RevocationPolicyError {}

/// Non-copyable evidence that one observation passed only this policy gate.
///
/// This result retains the exact caller-owned observation and explicit time.
/// It does not authenticate the status, bind it to a certificate or path, or
/// grant trust, TLS, HTTP, OAuth transport, or network authority.
pub struct X509RevocationPolicyAcceptance<'observation> {
    observation: &'observation X509RevocationObservation,
    observation_time: X509Time,
}

impl<'observation> X509RevocationPolicyAcceptance<'observation> {
    pub const fn observation(&self) -> &'observation X509RevocationObservation {
        self.observation
    }

    pub const fn observation_time(&self) -> X509Time {
        self.observation_time
    }
}

impl fmt::Debug for X509RevocationPolicyAcceptance<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("X509RevocationPolicyAcceptance")
    }
}

/// Evaluate one observation with strict closed-interval freshness.
///
/// Failure order is stable: presence, status, endpoint presence, interval
/// ordering, then the explicit observation time. `thisUpdate` and `nextUpdate`
/// are inclusive. No tolerance or unbounded-freshness mode exists.
pub fn evaluate_x509_revocation_observation<'observation>(
    observation: Option<&'observation X509RevocationObservation>,
    observation_time: X509Time,
) -> Result<X509RevocationPolicyAcceptance<'observation>, X509RevocationPolicyError> {
    let observation = observation.ok_or_else(|| {
        X509RevocationPolicyError::new(X509RevocationPolicyErrorKind::MissingObservation)
    })?;
    match observation.status {
        X509RevocationStatus::Good => {}
        X509RevocationStatus::Revoked => {
            return Err(X509RevocationPolicyError::new(
                X509RevocationPolicyErrorKind::Revoked,
            ));
        }
        X509RevocationStatus::Unknown => {
            return Err(X509RevocationPolicyError::new(
                X509RevocationPolicyErrorKind::Unknown,
            ));
        }
    }

    let this_update = observation.this_update.ok_or_else(|| {
        X509RevocationPolicyError::new(X509RevocationPolicyErrorKind::MissingThisUpdate)
    })?;
    let next_update = observation.next_update.ok_or_else(|| {
        X509RevocationPolicyError::new(X509RevocationPolicyErrorKind::MissingNextUpdate)
    })?;

    if next_update.cmp_fields(&this_update) == Ordering::Less {
        return Err(X509RevocationPolicyError::new(
            X509RevocationPolicyErrorKind::InvertedUpdateRange,
        ));
    }
    if observation_time.cmp_fields(&this_update) == Ordering::Less {
        return Err(X509RevocationPolicyError::new(
            X509RevocationPolicyErrorKind::FutureThisUpdate,
        ));
    }
    if observation_time.cmp_fields(&next_update) == Ordering::Greater {
        return Err(X509RevocationPolicyError::new(
            X509RevocationPolicyErrorKind::StaleNextUpdate,
        ));
    }

    Ok(X509RevocationPolicyAcceptance {
        observation,
        observation_time,
    })
}
