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
//! When reviewed-terminal evidence already owns the construction, the
//! follow-up binder consumes that complete value and server policy directly.
//! Its result retains the path, reviewed-root selection, and exact target
//! policy together without requiring a second consumable path value.
//!
//! Binding is not path validation. Success authenticates no issuer link,
//! grants no trust anchor, applies no path-length or name constraint, checks no
//! revocation state, and provides no TLS, HTTP, OAuth, or network authority.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate_path_construction::ConstructedX509CertificatePath;
use x509_certificate_path_reviewed_anchor_binding::BoundConstructedX509CertificatePathToReviewedAnchor;
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

/// Non-copyable evidence retaining both endpoint policies around one path.
///
/// The reviewed-path input already retains construction and exact terminal
/// root selection. This result additionally retains complete policy for that
/// same construction's exact target, so neither endpoint proof nor the path
/// connecting them can be silently replaced or discarded at this boundary.
pub struct BoundReviewedConstructedX509CertificatePathToServerPolicy<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'policy_certificate,
    'policy_encoded,
> {
    reviewed_path: BoundConstructedX509CertificatePathToReviewedAnchor<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >,
    server_policy: ValidatedX509ServerCertificatePolicy<'policy_certificate, 'policy_encoded>,
}

impl<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'policy_certificate,
        'policy_encoded,
    >
    BoundReviewedConstructedX509CertificatePathToServerPolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'policy_certificate,
        'policy_encoded,
    >
{
    /// Return the exact reviewed-terminal-bound path retained by this evidence.
    pub const fn reviewed_path(
        &self,
    ) -> &BoundConstructedX509CertificatePathToReviewedAnchor<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
    > {
        &self.reviewed_path
    }

    /// Return the exact complete target server policy retained by this evidence.
    pub const fn server_policy(
        &self,
    ) -> &ValidatedX509ServerCertificatePolicy<'policy_certificate, 'policy_encoded> {
        &self.server_policy
    }
}

impl fmt::Debug
    for BoundReviewedConstructedX509CertificatePathToServerPolicy<'_, '_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundReviewedConstructedX509CertificatePathToServerPolicy")
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

/// Bind reviewed-terminal construction and complete policy for its exact target.
///
/// This follow-up consumes the reviewed-terminal-bound path rather than a
/// second copy of the non-copyable construction. A byte-equal target produced
/// by another decode is rejected before either input moves into the result.
/// Success proves endpoint evidence identity only; it does not authenticate
/// links or validate the certification path.
pub fn bind_reviewed_constructed_x509_certificate_path_to_server_policy<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'policy_certificate,
    'policy_encoded,
>(
    reviewed_path: BoundConstructedX509CertificatePathToReviewedAnchor<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >,
    server_policy: ValidatedX509ServerCertificatePolicy<'policy_certificate, 'policy_encoded>,
) -> Result<
    BoundReviewedConstructedX509CertificatePathToServerPolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'policy_certificate,
        'policy_encoded,
    >,
    X509CertificatePathServerPolicyBindingError,
> {
    if !core::ptr::eq(
        reviewed_path.constructed_path().target_certificate(),
        server_policy.certificate(),
    ) {
        return Err(X509CertificatePathServerPolicyBindingError::new(
            X509CertificatePathServerPolicyBindingErrorKind::EvidenceMismatch,
        ));
    }

    Ok(BoundReviewedConstructedX509CertificatePathToServerPolicy {
        reviewed_path,
        server_policy,
    })
}
