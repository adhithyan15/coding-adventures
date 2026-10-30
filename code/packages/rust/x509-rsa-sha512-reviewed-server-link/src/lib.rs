//! # Exact RSA SHA-512 reviewed server-link binding
//!
//! This crate closes one evidence-mixing gap between target-server policy and
//! one authenticated RSA SHA-512 issuer link terminating at a reviewed root.
//! It performs no new parsing, signature work, root lookup, path search,
//! revocation processing, or network effect.

#![deny(unsafe_code)]

use core::fmt;
use x509_rsa_sha512_reviewed_anchor_link::BoundX509RsaSha512ReviewedAnchorLink;
use x509_server_certificate_policy::ValidatedX509ServerCertificatePolicy;

/// Stable, payload-free reviewed server-link binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha512ReviewedServerLinkErrorKind {
    /// The inputs retain different decoded target certificate objects.
    EvidenceMismatch,
}

/// A redacted reviewed server-link binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha512ReviewedServerLinkError {
    kind: X509RsaSha512ReviewedServerLinkErrorKind,
}

impl X509RsaSha512ReviewedServerLinkError {
    const fn evidence_mismatch() -> Self {
        Self {
            kind: X509RsaSha512ReviewedServerLinkErrorKind::EvidenceMismatch,
        }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha512ReviewedServerLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha512ReviewedServerLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-512 reviewed server-link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha512ReviewedServerLinkError {}

/// Non-copyable target-server policy bound to one reviewed RSA SHA-512 link.
///
/// The retained target evidence covers caller-owned critical-extension and
/// algorithm policy, end-entity role, SAN-only identity, explicit-time
/// validity, and one exact Extended Key Usage purpose. The retained link
/// evidence covers one RSA SHA-512 child-to-issuer signature and exact issuer
/// membership in the reviewed Mozilla snapshot. This wrapper does not apply
/// anchor constraints, construct a multi-hop path, check revocation, or grant
/// TLS, HTTP, OAuth, or network authority.
pub struct BoundX509RsaSha512ReviewedServerLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    server_policy: ValidatedX509ServerCertificatePolicy<'child_certificate, 'child_encoded>,
    reviewed_anchor_link: BoundX509RsaSha512ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    BoundX509RsaSha512ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the consumed exact-certificate target-server policy.
    #[must_use]
    pub const fn server_policy(
        &self,
    ) -> &ValidatedX509ServerCertificatePolicy<'child_certificate, 'child_encoded> {
        &self.server_policy
    }

    /// Return the consumed authenticated link and reviewed-root binding.
    #[must_use]
    pub const fn reviewed_anchor_link(
        &self,
    ) -> &BoundX509RsaSha512ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.reviewed_anchor_link
    }
}

impl fmt::Debug for BoundX509RsaSha512ReviewedServerLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaSha512ReviewedServerLink")
    }
}

/// Bind target-server policy to one reviewed-root issuer link for the same
/// exact decoded target certificate object.
///
/// Byte equality is deliberately insufficient. Both non-copyable inputs must
/// originate from the same decoded target object. Success retains both
/// complete inputs but performs no additional certificate validation.
pub fn bind_x509_rsa_sha512_reviewed_server_link<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    server_policy: ValidatedX509ServerCertificatePolicy<'child_certificate, 'child_encoded>,
    reviewed_anchor_link: BoundX509RsaSha512ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
) -> Result<
    BoundX509RsaSha512ReviewedServerLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha512ReviewedServerLinkError,
> {
    if !same_exact_object(
        server_policy.certificate(),
        reviewed_anchor_link.verified_issuer_link().child(),
    ) {
        return Err(X509RsaSha512ReviewedServerLinkError::evidence_mismatch());
    }

    Ok(BoundX509RsaSha512ReviewedServerLink {
        server_policy,
        reviewed_anchor_link,
    })
}

fn same_exact_object<T>(left: &T, right: &T) -> bool {
    std::ptr::eq(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_object_identity_rejects_byte_equal_distinct_values() {
        let left = [1_u8, 2, 3];
        let right = [1_u8, 2, 3];

        assert!(same_exact_object(&left, &left));
        assert!(!same_exact_object(&left, &right));
    }

    #[test]
    fn mismatch_error_is_stable_and_payload_free() {
        let error = X509RsaSha512ReviewedServerLinkError::evidence_mismatch();

        assert_eq!(
            error.kind(),
            X509RsaSha512ReviewedServerLinkErrorKind::EvidenceMismatch
        );
        assert_eq!(
            error.to_string(),
            "X.509 RSA SHA-512 reviewed server-link error EvidenceMismatch"
        );
    }
}
