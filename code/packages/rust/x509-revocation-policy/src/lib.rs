//! Fail-closed policy for explicit caller-supplied X.509 revocation observations.

#![deny(unsafe_code)]

use core::fmt;
use x509_time::X509Time;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RevocationStatus {
    Good,
    Revoked,
    Unknown,
}

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
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RevocationPolicyError {
    kind: X509RevocationPolicyErrorKind,
}

impl X509RevocationPolicyError {
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

pub struct AcceptedX509RevocationObservation<'observation> {
    observation: &'observation X509RevocationObservation,
    observation_time: X509Time,
}

impl<'observation> AcceptedX509RevocationObservation<'observation> {
    pub const fn observation(&self) -> &'observation X509RevocationObservation {
        self.observation
    }

    pub const fn observation_time(&self) -> X509Time {
        self.observation_time
    }
}

impl fmt::Debug for AcceptedX509RevocationObservation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AcceptedX509RevocationObservation")
    }
}

pub fn evaluate_x509_revocation_observation<'observation>(
    _observation: Option<&'observation X509RevocationObservation>,
    _observation_time: X509Time,
) -> Result<AcceptedX509RevocationObservation<'observation>, X509RevocationPolicyError> {
    todo!("implemented after the failing policy tests")
}
