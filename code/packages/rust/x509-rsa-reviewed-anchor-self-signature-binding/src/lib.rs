//! # Reviewed-anchor RSA self-signature binding
//!
//! Repository-owned RSA signature routing and reviewed-root selection are
//! deliberately separate operations. This crate joins their strongest public
//! evidence only when all three certificate identities are the same object:
//!
//! ```text
//! routed signature child ─────────────┐
//! routed signature issuer-key policy ├── exact decoded certificate object
//! reviewed snapshot selection ───────┘
//! ```
//!
//! Requiring both sides of the routed signature to be the reviewed certificate
//! turns the existing child-by-issuer verification into a self-signature proof
//! without repeating algorithm dispatch or RSA work. Pointer identity rejects
//! an independently decoded byte-equal certificate at either boundary.
//!
//! A self-signature is not a source of trust. This crate does not decide
//! trust-anchor semantics, certificate role, time, constraints, path validity,
//! revocation, TLS, HTTP, OAuth, credential access, I/O, or external effects.

#![forbid(unsafe_code)]

use core::fmt;
use x509_rsa_signature_routing::VerifiedX509RsaPolicySignature;
use x509_webpki_root_anchor_selection::SelectedReviewedWebPkiRootAnchor;

/// Stable, payload-free binding failures.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaReviewedAnchorSelfSignatureBindingError {
    /// The signed child is not the exact reviewed certificate object.
    ChildCertificateMismatch,
    /// The RSA key policy belongs to a different certificate object.
    IssuerCertificateMismatch,
}

impl fmt::Display for X509RsaReviewedAnchorSelfSignatureBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ChildCertificateMismatch => {
                "X.509 reviewed-anchor RSA self-signature binding failed: child mismatch"
            }
            Self::IssuerCertificateMismatch => {
                "X.509 reviewed-anchor RSA self-signature binding failed: issuer mismatch"
            }
        })
    }
}

impl std::error::Error for X509RsaReviewedAnchorSelfSignatureBindingError {}

/// Non-copyable evidence for one exact reviewed root's routed self-signature.
///
/// The routed proof retains caller-selected profile routing, complete child
/// certificate algorithm policy, complete issuer RSA algorithm-and-key policy,
/// and the selected fixed SHA-256, SHA-384, or SHA-512 verifier result. The
/// reviewed selection retains exact membership in the pinned root snapshot.
/// This wrapper keeps both inputs intact after checking their object identity.
pub struct BoundX509RsaReviewedAnchorSelfSignature<
    'certificate,
    'encoded,
    'issuer_certificate,
    'issuer_encoded,
    'anchor_certificate,
    'anchor_encoded,
> {
    verified_self_signature: VerifiedX509RsaPolicySignature<
        'certificate,
        'encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded>,
}

impl<
        'certificate,
        'encoded,
        'issuer_certificate,
        'issuer_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >
    BoundX509RsaReviewedAnchorSelfSignature<
        'certificate,
        'encoded,
        'issuer_certificate,
        'issuer_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >
{
    /// Return the complete routed repository-owned RSA signature proof.
    #[must_use]
    pub const fn verified_self_signature(
        &self,
    ) -> &VerifiedX509RsaPolicySignature<'certificate, 'encoded, 'issuer_certificate, 'issuer_encoded>
    {
        &self.verified_self_signature
    }

    /// Return reviewed-snapshot selection for the same exact certificate.
    #[must_use]
    pub const fn reviewed_anchor(
        &self,
    ) -> &SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded> {
        &self.reviewed_anchor
    }
}

impl fmt::Debug for BoundX509RsaReviewedAnchorSelfSignature<'_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaReviewedAnchorSelfSignature")
    }
}

/// Bind routed RSA verification to one exact reviewed root self-signature.
///
/// Byte equality is deliberately insufficient. The verified child, the
/// certificate that supplied the RSA issuer key policy, and the certificate
/// selected from the reviewed snapshot must be the identical decoded object.
/// The function performs no new parsing, routing, or cryptography.
pub fn bind_x509_rsa_reviewed_anchor_self_signature<
    'certificate,
    'encoded,
    'issuer_certificate,
    'issuer_encoded,
    'anchor_certificate,
    'anchor_encoded,
>(
    verified_self_signature: VerifiedX509RsaPolicySignature<
        'certificate,
        'encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded>,
) -> Result<
    BoundX509RsaReviewedAnchorSelfSignature<
        'certificate,
        'encoded,
        'issuer_certificate,
        'issuer_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >,
    X509RsaReviewedAnchorSelfSignatureBindingError,
> {
    let reviewed_certificate = reviewed_anchor.candidate_certificate();
    if !same_object(
        verified_self_signature
            .child_algorithm_policy()
            .certificate(),
        reviewed_certificate,
    ) {
        return Err(X509RsaReviewedAnchorSelfSignatureBindingError::ChildCertificateMismatch);
    }
    if !same_object(
        verified_self_signature
            .issuer_rsa_algorithm_policy()
            .certificate(),
        reviewed_certificate,
    ) {
        return Err(X509RsaReviewedAnchorSelfSignatureBindingError::IssuerCertificateMismatch);
    }

    Ok(BoundX509RsaReviewedAnchorSelfSignature {
        verified_self_signature,
        reviewed_anchor,
    })
}

fn same_object<T: ?Sized>(left: &T, right: &T) -> bool {
    core::ptr::eq(left, right)
}

#[cfg(test)]
mod tests {
    use super::same_object;

    #[test]
    fn exact_object_helper_rejects_equal_values_from_distinct_allocations() {
        let retained = [0xde, 0xad, 0xbe, 0xef];
        let byte_equal_independent = [0xde, 0xad, 0xbe, 0xef];

        assert!(same_object(&retained, &retained));
        assert!(!same_object(&retained, &byte_equal_independent));
    }
}
