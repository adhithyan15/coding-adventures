//! # Exact OCSP CertID-bound revocation policy
//!
//! This crate derives one revocation observation only from the exact OCSP
//! `SingleResponse` already bound to exact target and issuer certificates. It
//! then applies the repository-owned strict status and freshness policy at one
//! explicit caller-owned time. It deliberately grants no response-signature,
//! responder-authorization, path, trust, TLS, HTTP, OAuth-transport, clock, or
//! network authority.

#![deny(unsafe_code)]

use core::fmt;
use x509_ocsp_cert_id_binding::BoundX509OcspCertId;
use x509_ocsp_cert_status::X509OcspCertStatus;
use x509_revocation_policy::{
    evaluate_x509_revocation_observation, X509RevocationObservation, X509RevocationPolicyErrorKind,
    X509RevocationStatus,
};
use x509_time::X509Time;

/// Stable, payload-free OCSP revocation-policy failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspRevocationPolicyErrorKind {
    /// The bound `SingleResponse` carries extensions whose semantics this
    /// deliberately narrow policy does not implement.
    UnsupportedSingleResponseExtensions,
    /// The exact response status or freshness interval failed the shared
    /// repository-owned revocation policy.
    Policy(X509RevocationPolicyErrorKind),
}

/// One redacted OCSP revocation-policy failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRevocationPolicyError {
    kind: X509OcspRevocationPolicyErrorKind,
}

impl X509OcspRevocationPolicyError {
    const fn new(kind: X509OcspRevocationPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspRevocationPolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspRevocationPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP revocation policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspRevocationPolicyError {}

/// Non-copyable evidence retaining the exact CertID binding and explicit time.
///
/// The retained binding fixes the exact `SingleResponse`, target certificate,
/// and candidate issuer whose identifier hashes matched. This result adds only
/// strict good-status and closed-interval freshness policy for that exact
/// response. It does not authenticate the response or authorize its responder.
pub struct ValidatedBoundX509OcspRevocation<
    'binding,
    'response,
    'response_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
> {
    cert_id_binding: &'binding BoundX509OcspCertId<
        'response,
        'response_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    observation_time: X509Time,
}

impl<
        'binding,
        'response,
        'response_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
    ValidatedBoundX509OcspRevocation<
        'binding,
        'response,
        'response_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    /// Return the exact target/issuer/response binding whose status was used.
    pub const fn cert_id_binding(
        &self,
    ) -> &'binding BoundX509OcspCertId<
        'response,
        'response_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > {
        self.cert_id_binding
    }

    /// Return the explicit caller-owned time used for freshness policy.
    pub const fn observation_time(&self) -> X509Time {
        self.observation_time
    }
}

impl fmt::Debug for ValidatedBoundX509OcspRevocation<'_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedBoundX509OcspRevocation")
    }
}

/// Apply strict status and freshness policy to one exact CertID-bound response.
///
/// Failure order is stable: unsupported `singleExtensions`, then the shared
/// revocation policy's status and freshness ordering. The observation is
/// derived internally from the retained response, preventing a caller from
/// attaching detached status or time endpoints to another certificate.
pub fn validate_bound_x509_ocsp_revocation<
    'binding,
    'response,
    'response_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
>(
    cert_id_binding: &'binding BoundX509OcspCertId<
        'response,
        'response_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    observation_time: X509Time,
) -> Result<
    ValidatedBoundX509OcspRevocation<
        'binding,
        'response,
        'response_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    X509OcspRevocationPolicyError,
> {
    let response = cert_id_binding.response();
    if response.single_extensions().is_some() {
        return Err(X509OcspRevocationPolicyError::new(
            X509OcspRevocationPolicyErrorKind::UnsupportedSingleResponseExtensions,
        ));
    }

    let status = match response.cert_status() {
        X509OcspCertStatus::Good => X509RevocationStatus::Good,
        X509OcspCertStatus::Revoked(_) => X509RevocationStatus::Revoked,
        X509OcspCertStatus::Unknown => X509RevocationStatus::Unknown,
    };
    let observation = X509RevocationObservation::new(
        status,
        Some(response.this_update()),
        response.next_update(),
    );
    evaluate_x509_revocation_observation(Some(&observation), observation_time).map_err(
        |error| {
            X509OcspRevocationPolicyError::new(X509OcspRevocationPolicyErrorKind::Policy(
                error.kind(),
            ))
        },
    )?;

    Ok(ValidatedBoundX509OcspRevocation {
        cert_id_binding,
        observation_time,
    })
}
