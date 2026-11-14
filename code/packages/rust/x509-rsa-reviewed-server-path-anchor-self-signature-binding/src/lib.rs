//! # Reviewed server-path anchor self-signature binding
//!
//! The strongest accumulated reviewed RSA server-path policy and the reviewed
//! anchor self-signature proof are deliberately produced by separate packages.
//! This crate closes the resulting evidence-integrity gap: both complete inputs
//! are retained only when their reviewed-root selections point at the same
//! decoded certificate object.
//!
//! ```text
//! reviewed server path ── reviewed terminal ──┐
//!                                             ├── exact decoded root
//! reviewed self-signature ── reviewed root ───┘
//! ```
//!
//! This is a pure composition boundary. It performs no parsing, signature
//! verification, path or Name validation, revocation, TLS, HTTP, OAuth, I/O,
//! credential operation, or external effect. Concrete HTTPS and live-provider
//! readiness remain unavailable.

#![forbid(unsafe_code)]

use core::fmt;
use x509_rsa_constructed_path_signature_routing::ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy;
use x509_rsa_reviewed_anchor_self_signature_binding::BoundX509RsaReviewedAnchorSelfSignature;

/// The strongest accumulated reviewed RSA server-path policy currently
/// available to this boundary.
///
/// The concrete type retains Name-policy-backed construction, reviewed-root
/// selection, anchor role, ordered routed signatures, path-length state,
/// complete target server policy, and supported target and subordinate
/// NameConstraints evidence.
pub type ReviewedRsaServerPath<
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

/// Stable, payload-free composition failure categories.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaReviewedServerPathAnchorSelfSignatureBindingError {
    /// The path terminal and self-signature proof select different objects.
    AnchorCertificateMismatch,
}

impl fmt::Display for X509RsaReviewedServerPathAnchorSelfSignatureBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "X.509 reviewed server-path anchor self-signature binding failed: anchor mismatch",
        )
    }
}

impl std::error::Error for X509RsaReviewedServerPathAnchorSelfSignatureBindingError {}

/// Non-copyable evidence retaining the complete path and anchor signature.
///
/// Matching lifetimes are not treated as identity. Construction performs a
/// pointer-identity check before moving both independently produced evidence
/// values into this result.
pub struct BoundX509RsaReviewedServerPathAnchorSelfSignature<
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
    reviewed_server_path: ReviewedRsaServerPath<
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
    anchor_self_signature: BoundX509RsaReviewedAnchorSelfSignature<
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    >,
}

impl<
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
    BoundX509RsaReviewedServerPathAnchorSelfSignature<
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
    /// Return the complete accumulated reviewed server-path policy.
    #[must_use]
    pub const fn reviewed_server_path(
        &self,
    ) -> &ReviewedRsaServerPath<
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

    /// Return the complete routed reviewed-anchor self-signature proof.
    #[must_use]
    pub const fn anchor_self_signature(
        &self,
    ) -> &BoundX509RsaReviewedAnchorSelfSignature<
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    > {
        &self.anchor_self_signature
    }
}

impl fmt::Debug
    for BoundX509RsaReviewedServerPathAnchorSelfSignature<
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
            .debug_struct("BoundX509RsaReviewedServerPathAnchorSelfSignature")
            .field("reviewed_server_path", &"[REDACTED]")
            .field("anchor_self_signature", &"[REDACTED]")
            .finish()
    }
}

/// Bind the strongest reviewed path to its exact anchor self-signature proof.
///
/// The path anchor is derived only through the complete retained construction.
/// The self-signature input already binds its routed child and issuer-key
/// policies to its own reviewed selection. Byte-equal independently decoded
/// roots therefore fail closed at this final cross-boundary identity check.
pub fn bind_x509_rsa_reviewed_server_path_anchor_self_signature<
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
    reviewed_server_path: ReviewedRsaServerPath<
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
    anchor_self_signature: BoundX509RsaReviewedAnchorSelfSignature<
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    >,
) -> Result<
    BoundX509RsaReviewedServerPathAnchorSelfSignature<
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
    X509RsaReviewedServerPathAnchorSelfSignatureBindingError,
> {
    let path_anchor = reviewed_server_path
        .subordinate_policy()
        .target_policy()
        .anchor_policy()
        .target_signature_closure()
        .accumulated_intermediates()
        .reviewed_terminal()
        .reviewed_anchor()
        .candidate_certificate();
    let signature_anchor = anchor_self_signature
        .reviewed_anchor()
        .candidate_certificate();

    if !same_object(path_anchor, signature_anchor) {
        return Err(
            X509RsaReviewedServerPathAnchorSelfSignatureBindingError::AnchorCertificateMismatch,
        );
    }

    Ok(BoundX509RsaReviewedServerPathAnchorSelfSignature {
        reviewed_server_path,
        anchor_self_signature,
    })
}

fn same_object<T: ?Sized>(left: &T, right: &T) -> bool {
    core::ptr::eq(left, right)
}

#[cfg(test)]
mod tests {
    use super::{same_object, X509RsaReviewedServerPathAnchorSelfSignatureBindingError};

    #[test]
    fn exact_object_helper_rejects_equal_values_from_distinct_allocations() {
        let retained = vec![0xde, 0xad, 0xbe, 0xef];
        let byte_equal_independent = vec![0xde, 0xad, 0xbe, 0xef];

        assert!(same_object(&retained, &retained));
        assert!(!same_object(&retained, &byte_equal_independent));
    }

    #[test]
    fn failure_is_stable_and_payload_free() {
        let error =
            X509RsaReviewedServerPathAnchorSelfSignatureBindingError::AnchorCertificateMismatch;
        let display = error.to_string();
        let debug = format!("{error:?}");

        assert_eq!(
            display,
            "X.509 reviewed server-path anchor self-signature binding failed: anchor mismatch"
        );
        assert_eq!(debug, "AnchorCertificateMismatch");
        for payload in ["USERTrust", "sectigo.com", "SHA-384", "4096", "65537"] {
            assert!(!display.contains(payload));
            assert!(!debug.contains(payload));
        }
    }
}
