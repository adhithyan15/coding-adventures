//! # Constructed-path server-policy binding
//!
//! Path construction and complete target server policy produce separate
//! evidence objects. A later path validator must not be able to attach the
//! hostname, validity, purpose, role, critical-extension, and algorithm policy
//! of one decoded certificate to a path constructed from another certificate.
//! This crate closes that evidence-mixing gap by joining the two objects only
//! when they retain the same decoded target certificate object.
//!
//! ## Why object identity is deliberate
//!
//! Equal DER does not make independently produced evidence interchangeable.
//! A second decode can have a different lifetime, owner, candidate pool, or
//! policy history. Comparing pointers rejects that substitution and the
//! returned non-copyable value keeps both complete inputs alive:
//!
//! ```text
//! constructed path ── target ──┐
//!                              ├── exact object? ── bound evidence
//! server policy ─── certificate┘
//! ```
//!
//! Binding is not path validation. Success authenticates no issuer link,
//! grants no trust anchor, applies no path-length or name constraint, checks no
//! revocation state, and provides no TLS, HTTP, OAuth, or network authority.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate_path_construction::ConstructedX509CertificatePath;
use x509_server_certificate_policy::ValidatedX509ServerCertificatePolicy;

/// Stable, payload-free constructed-target binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509CertificatePathServerPolicyBindingErrorKind {
    /// The path and server policy retain different decoded certificate objects.
    EvidenceMismatch,
}

/// A redacted constructed-path server-policy binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509CertificatePathServerPolicyBindingError {
    kind: X509CertificatePathServerPolicyBindingErrorKind,
}

impl X509CertificatePathServerPolicyBindingError {
    const fn new(kind: X509CertificatePathServerPolicyBindingErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category without certificate payloads.
    pub const fn kind(self) -> X509CertificatePathServerPolicyBindingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509CertificatePathServerPolicyBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate path server-policy binding error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509CertificatePathServerPolicyBindingError {}

/// Non-copyable evidence retaining both inputs to an exact target binding.
///
/// Retaining the complete constructed path preserves its target, candidate
/// pool, and selected indices. Retaining complete server policy preserves the
/// target's role, critical-extension recognition, algorithm profile, identity,
/// validity, and purpose evidence. Neither input can be silently replaced after
/// the object-identity check succeeds.
pub struct BoundConstructedX509CertificatePathToServerPolicy<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy_certificate,
    'policy_encoded,
> {
    constructed_path:
        ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded>,
    server_policy: ValidatedX509ServerCertificatePolicy<'policy_certificate, 'policy_encoded>,
}

impl<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy_certificate,
        'policy_encoded,
    >
    BoundConstructedX509CertificatePathToServerPolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy_certificate,
        'policy_encoded,
    >
{
    /// Return the exact constructed path retained by this evidence.
    pub const fn constructed_path(
        &self,
    ) -> &ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded>
    {
        &self.constructed_path
    }

    /// Return the exact complete target server policy retained by this evidence.
    pub const fn server_policy(
        &self,
    ) -> &ValidatedX509ServerCertificatePolicy<'policy_certificate, 'policy_encoded> {
        &self.server_policy
    }
}

impl fmt::Debug for BoundConstructedX509CertificatePathToServerPolicy<'_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundConstructedX509CertificatePathToServerPolicy")
    }
}

/// Bind complete server policy to a constructed path's exact target object.
///
/// The identity check happens before either evidence value is moved into the
/// returned boundary. A byte-equal certificate produced by another decode is
/// therefore rejected. The result proves only evidence identity; it does not
/// authenticate or validate the constructed certification path.
pub fn bind_constructed_x509_certificate_path_to_server_policy<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy_certificate,
    'policy_encoded,
>(
    constructed_path: ConstructedX509CertificatePath<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    server_policy: ValidatedX509ServerCertificatePolicy<'policy_certificate, 'policy_encoded>,
) -> Result<
    BoundConstructedX509CertificatePathToServerPolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy_certificate,
        'policy_encoded,
    >,
    X509CertificatePathServerPolicyBindingError,
> {
    if !core::ptr::eq(
        constructed_path.target_certificate(),
        server_policy.certificate(),
    ) {
        return Err(X509CertificatePathServerPolicyBindingError::new(
            X509CertificatePathServerPolicyBindingErrorKind::EvidenceMismatch,
        ));
    }

    Ok(BoundConstructedX509CertificatePathToServerPolicy {
        constructed_path,
        server_policy,
    })
}
