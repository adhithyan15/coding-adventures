//! # Reviewed HTTPS destination and server-path binding
//!
//! An HTTPS client must not validate a certificate path for one decoded leaf
//! and then send TLS and HTTP protocol identities proven against another leaf.
//! This crate closes that one composition gap: it consumes the complete DNS
//! destination proof and either the strongest accumulated reviewed RSA
//! server-path policy or that path already bound to its reviewed anchor's
//! repository-owned self-signature proof. Both entry points require their
//! complete inputs to retain the same exact decoded target object.
//!
//! ```text
//! HTTPS destination ── certificate ──┐
//!                                    ├── exact decoded object
//! reviewed server path ── target ────┘
//! ```
//!
//! The stronger result retains the complete path-to-anchor-self-signature
//! proof after this target check; it does not replace the path-only boundary.
//!
//! This is an evidence-integrity boundary, not certificate validation. It
//! performs no new parsing, signature verification, path or Name validation,
//! revocation, TLS, HTTP, credential access, audit, trace, I/O, or external
//! effect. Concrete HTTPS and live-provider readiness remain unavailable.

#![forbid(unsafe_code)]

use core::fmt;
use https_destination_name_binding::BoundHttpsDnsDestination;
use x509_rsa_constructed_path_signature_routing::ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy;
use x509_rsa_reviewed_server_path_anchor_self_signature_binding::BoundX509RsaReviewedServerPathAnchorSelfSignature;

/// The strongest accumulated reviewed RSA server-path policy currently
/// available to this composition boundary.
///
/// Keeping the complete concrete type prevents callers from substituting a
/// detached certificate or a weaker callback-shaped claim. The retained proof
/// includes reviewed-root selection, routed signatures, complete endpoint
/// role and algorithm policy, path-length state, and the supported target and
/// subordinate NameConstraints evidence.
pub type ReviewedHttpsServerPath<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> = ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>;

/// Complete reviewed server-path evidence with its exact anchor signature.
///
/// This alias deliberately preserves the concrete non-copyable proof rather
/// than accepting a detached root or callback-shaped assertion. The retained
/// path includes the strongest accumulated Name-policy evidence, while the
/// retained anchor proof includes data-driven repository-owned RSA signature
/// verification and reviewed-snapshot selection.
pub type ReviewedHttpsServerPathAnchorSelfSignature<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
    'signature_certificate,
    'signature_encoded,
    'signature_issuer_certificate,
    'signature_issuer_encoded,
    'signature_anchor_certificate,
    'signature_anchor_encoded,
> = BoundX509RsaReviewedServerPathAnchorSelfSignature<
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
    'signature_certificate,
    'signature_encoded,
    'signature_issuer_certificate,
    'signature_issuer_encoded,
    'signature_anchor_certificate,
    'signature_anchor_encoded,
>;

/// Stable, payload-free binding failure categories.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsReviewedServerPathBindingError {
    /// The destination and reviewed path retain different decoded leaves.
    TargetCertificateMismatch,
}

impl fmt::Display for HttpsReviewedServerPathBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HTTPS reviewed server-path binding failed: target mismatch")
    }
}

impl std::error::Error for HttpsReviewedServerPathBindingError {}

/// Non-copyable evidence retaining one destination and its exact reviewed path.
///
/// The shared target lifetimes are not treated as identity: two independent
/// decoded certificates can live for the same duration. Construction also
/// checks pointer identity before moving both complete inputs into this result.
pub struct BoundHttpsDnsDestinationToReviewedServerPath<
    'server_name,
    'host,
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    destination: BoundHttpsDnsDestination<'target, 'target_encoded, 'server_name, 'host>,
    reviewed_server_path: ReviewedHttpsServerPath<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
}

impl<
        'server_name,
        'host,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    BoundHttpsDnsDestinationToReviewedServerPath<
        'server_name,
        'host,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the complete X.509 SAN, TLS SNI, and HTTP Host destination proof.
    #[must_use]
    pub const fn destination(
        &self,
    ) -> &BoundHttpsDnsDestination<'target, 'target_encoded, 'server_name, 'host> {
        &self.destination
    }

    /// Return the complete reviewed RSA server-path policy.
    #[must_use]
    pub const fn reviewed_server_path(
        &self,
    ) -> &ReviewedHttpsServerPath<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        &self.reviewed_server_path
    }
}

impl fmt::Debug
    for BoundHttpsDnsDestinationToReviewedServerPath<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundHttpsDnsDestinationToReviewedServerPath")
            .field("destination", &"[REDACTED]")
            .field("reviewed_server_path", &"[REDACTED]")
            .finish()
    }
}

/// Bind one HTTPS DNS destination to the exact target of reviewed path policy.
///
/// The path target is derived exclusively through the complete server policy
/// retained by `reviewed_server_path`. A byte-equal certificate produced by an
/// independent decode is a different object and fails closed. No certificate,
/// path, algorithm, signature, identity, or policy input can be substituted
/// after success because both complete non-copyable inputs are moved into the
/// returned evidence.
pub fn bind_https_dns_destination_to_reviewed_server_path<
    'server_name,
    'host,
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    destination: BoundHttpsDnsDestination<'target, 'target_encoded, 'server_name, 'host>,
    reviewed_server_path: ReviewedHttpsServerPath<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
) -> Result<
    BoundHttpsDnsDestinationToReviewedServerPath<
        'server_name,
        'host,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    HttpsReviewedServerPathBindingError,
> {
    let path_certificate = reviewed_server_path
        .subordinate_policy()
        .target_policy()
        .anchor_policy()
        .target_signature_closure()
        .server_policy()
        .certificate();

    if !same_object(destination.certificate(), path_certificate) {
        return Err(HttpsReviewedServerPathBindingError::TargetCertificateMismatch);
    }

    Ok(BoundHttpsDnsDestinationToReviewedServerPath {
        destination,
        reviewed_server_path,
    })
}

/// Non-copyable HTTPS destination, reviewed path, and anchor-signature proof.
///
/// The complete path and anchor self-signature have already been bound to one
/// exact reviewed root. Construction adds the final exact-object check between
/// the destination certificate and the path target, then moves both complete
/// inputs into this result so no detached evidence can be substituted later.
pub struct BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature<
    'server_name,
    'host,
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
    'signature_certificate,
    'signature_encoded,
    'signature_issuer_certificate,
    'signature_issuer_encoded,
    'signature_anchor_certificate,
    'signature_anchor_encoded,
> {
    destination: BoundHttpsDnsDestination<'target, 'target_encoded, 'server_name, 'host>,
    reviewed_server_path_anchor_self_signature: ReviewedHttpsServerPathAnchorSelfSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    >,
}

impl<
        'server_name,
        'host,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    >
    BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature<
        'server_name,
        'host,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    >
{
    /// Return the complete certificate, SNI, Host, and SAN destination proof.
    #[must_use]
    pub const fn destination(
        &self,
    ) -> &BoundHttpsDnsDestination<'target, 'target_encoded, 'server_name, 'host> {
        &self.destination
    }

    /// Return the complete reviewed path and anchor self-signature evidence.
    #[must_use]
    pub const fn reviewed_server_path_anchor_self_signature(
        &self,
    ) -> &ReviewedHttpsServerPathAnchorSelfSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    > {
        &self.reviewed_server_path_anchor_self_signature
    }
}

impl fmt::Debug
    for BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature")
            .field("destination", &"[REDACTED]")
            .field("reviewed_server_path_anchor_self_signature", &"[REDACTED]")
            .finish()
    }
}

/// Bind an HTTPS destination to a reviewed path with anchor self-signature.
///
/// The target certificate is derived only through the complete retained path;
/// the caller cannot supply a detached leaf. Byte-equal independent decodes
/// therefore fail closed, and success retains the destination, accumulated
/// path, reviewed root selection, and repository-owned anchor signature proof.
pub fn bind_https_dns_destination_to_reviewed_server_path_anchor_self_signature<
    'server_name,
    'host,
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
    'signature_certificate,
    'signature_encoded,
    'signature_issuer_certificate,
    'signature_issuer_encoded,
    'signature_anchor_certificate,
    'signature_anchor_encoded,
>(
    destination: BoundHttpsDnsDestination<'target, 'target_encoded, 'server_name, 'host>,
    reviewed_server_path_anchor_self_signature: ReviewedHttpsServerPathAnchorSelfSignature<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    >,
) -> Result<
    BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature<
        'server_name,
        'host,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    >,
    HttpsReviewedServerPathBindingError,
> {
    let path_certificate = reviewed_server_path_anchor_self_signature
        .reviewed_server_path()
        .subordinate_policy()
        .target_policy()
        .anchor_policy()
        .target_signature_closure()
        .server_policy()
        .certificate();

    if !same_object(destination.certificate(), path_certificate) {
        return Err(HttpsReviewedServerPathBindingError::TargetCertificateMismatch);
    }

    Ok(
        BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature {
            destination,
            reviewed_server_path_anchor_self_signature,
        },
    )
}

fn same_object<T: ?Sized>(left: &T, right: &T) -> bool {
    core::ptr::eq(left, right)
}

#[cfg(test)]
mod tests {
    use super::{same_object, HttpsReviewedServerPathBindingError};

    #[test]
    fn exact_object_check_accepts_only_the_same_allocation() {
        let retained = vec![0xde, 0xad, 0xbe, 0xef];
        let byte_equal_independent = vec![0xde, 0xad, 0xbe, 0xef];

        assert!(same_object(&retained, &retained));
        assert!(!same_object(&retained, &byte_equal_independent));
    }

    #[test]
    fn stable_error_discloses_no_binding_payload() {
        let error = HttpsReviewedServerPathBindingError::TargetCertificateMismatch;
        let display = error.to_string();
        let debug = format!("{error:?}");

        assert_eq!(
            display,
            "HTTPS reviewed server-path binding failed: target mismatch"
        );
        assert_eq!(debug, "TargetCertificateMismatch");
        for secret in ["api.example.test", "certificate", "signature", "root"] {
            assert!(!display.contains(secret));
            assert!(!debug.contains(secret));
        }
    }
}
