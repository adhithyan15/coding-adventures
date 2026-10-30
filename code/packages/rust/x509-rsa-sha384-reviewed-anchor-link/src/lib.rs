//! # RSA SHA-384 reviewed-anchor link binding
//!
//! This crate binds existing authenticated issuer-policy evidence to existing
//! reviewed-root selection evidence for one exact issuer object. It performs
//! no certificate parsing, signature work, root lookup, or path search.

#![deny(unsafe_code)]

use core::fmt;
use x509_rsa_sha384_issuer_link::VerifiedX509RsaSha384IssuerPolicyLink;
use x509_webpki_root_anchor_selection::SelectedReviewedWebPkiRootAnchor;

/// Stable, payload-free reviewed-anchor link binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha384ReviewedAnchorLinkErrorKind {
    /// The two evidence values do not retain the same decoded issuer object.
    EvidenceMismatch,
}

/// A redacted reviewed-anchor link binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha384ReviewedAnchorLinkError {
    kind: X509RsaSha384ReviewedAnchorLinkErrorKind,
}

impl X509RsaSha384ReviewedAnchorLinkError {
    const fn evidence_mismatch() -> Self {
        Self {
            kind: X509RsaSha384ReviewedAnchorLinkErrorKind::EvidenceMismatch,
        }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha384ReviewedAnchorLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha384ReviewedAnchorLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-384 reviewed-anchor link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha384ReviewedAnchorLinkError {}

/// Non-copyable evidence that one authenticated SHA-384 issuer-policy link
/// terminates at the exact certificate selected from the reviewed root set.
///
/// The retained issuer-link evidence already proves the exact child-to-issuer
/// Name relation, issuing-CA role and recognized critical extensions,
/// caller-owned RSA algorithm/key policy, and fixed RSA PKCS #1 v1.5 SHA-384
/// signature. The retained anchor evidence proves only unique subject-and-SPKI
/// membership in the reviewed snapshot. This wrapper grants no target policy,
/// anchor constraints, complete path, revocation, TLS, or network authority.
pub struct BoundX509RsaSha384ReviewedAnchorLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    verified_issuer_link: VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'issuer_certificate, 'issuer_encoded>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    BoundX509RsaSha384ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    /// Return the consumed policy-bound authenticated issuer link.
    #[must_use]
    pub const fn verified_issuer_link(
        &self,
    ) -> &VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.verified_issuer_link
    }

    /// Return the consumed reviewed-root selection evidence.
    #[must_use]
    pub const fn reviewed_anchor(
        &self,
    ) -> &SelectedReviewedWebPkiRootAnchor<'issuer_certificate, 'issuer_encoded> {
        &self.reviewed_anchor
    }
}

impl fmt::Debug for BoundX509RsaSha384ReviewedAnchorLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaSha384ReviewedAnchorLink")
    }
}

/// Bind one strongest authenticated RSA SHA-384 issuer-policy link to one
/// reviewed-root selection for the same exact issuer certificate object.
///
/// Byte equality is deliberately insufficient. Both evidence values must have
/// originated from the same decoded issuer object, preventing callers from
/// mixing independently checked certificates that happen to encode equally.
/// Success proves only the retained narrow properties; it is not complete path
/// validation or TLS server authentication.
pub fn bind_x509_rsa_sha384_issuer_link_to_reviewed_anchor<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    verified_issuer_link: VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'issuer_certificate, 'issuer_encoded>,
) -> Result<
    BoundX509RsaSha384ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha384ReviewedAnchorLinkError,
> {
    if !std::ptr::eq(
        verified_issuer_link.candidate_issuer(),
        reviewed_anchor.candidate_certificate(),
    ) {
        return Err(X509RsaSha384ReviewedAnchorLinkError::evidence_mismatch());
    }

    Ok(BoundX509RsaSha384ReviewedAnchorLink {
        verified_issuer_link,
        reviewed_anchor,
    })
}
