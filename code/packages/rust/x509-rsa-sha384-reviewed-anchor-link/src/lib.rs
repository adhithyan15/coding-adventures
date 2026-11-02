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

/// Stable, payload-free two-link reviewed-path binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha384ReviewedIntermediatePathErrorKind {
    /// The adjacent links do not retain the same decoded intermediate object.
    EvidenceMismatch,
}

/// A redacted two-link reviewed-path binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha384ReviewedIntermediatePathError {
    kind: X509RsaSha384ReviewedIntermediatePathErrorKind,
}

impl X509RsaSha384ReviewedIntermediatePathError {
    const fn evidence_mismatch() -> Self {
        Self {
            kind: X509RsaSha384ReviewedIntermediatePathErrorKind::EvidenceMismatch,
        }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha384ReviewedIntermediatePathErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha384ReviewedIntermediatePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-384 reviewed intermediate-path error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha384ReviewedIntermediatePathError {}

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

/// Non-copyable evidence joining one authenticated target-to-intermediate
/// SHA-384 link to one authenticated intermediate-to-reviewed-root SHA-384
/// link.
///
/// Both complete policy-bound link proofs remain retained. This wrapper does
/// not derive path-length state, apply name constraints, search a path, check
/// revocation, or grant TLS or network authority.
pub struct BoundX509RsaSha384ReviewedIntermediatePath<
    'child_certificate,
    'child_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
    'parent_child_certificate,
    'parent_child_encoded,
    'root_certificate,
    'root_encoded,
> {
    authenticated_child_link: VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
    >,
    reviewed_parent_link: BoundX509RsaSha384ReviewedAnchorLink<
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >,
}

impl<
        'child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >
    BoundX509RsaSha384ReviewedIntermediatePath<
        'child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >
{
    /// Return the consumed target-to-intermediate authenticated link.
    #[must_use]
    pub const fn authenticated_child_link(
        &self,
    ) -> &VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
    > {
        &self.authenticated_child_link
    }

    /// Return the consumed intermediate-to-reviewed-root binding.
    #[must_use]
    pub const fn reviewed_parent_link(
        &self,
    ) -> &BoundX509RsaSha384ReviewedAnchorLink<
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    > {
        &self.reviewed_parent_link
    }
}

impl fmt::Debug for BoundX509RsaSha384ReviewedIntermediatePath<'_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaSha384ReviewedIntermediatePath")
    }
}

/// Join two authenticated SHA-384 links only when they retain the same exact
/// decoded intermediate certificate object.
///
/// Encoded-byte equality is deliberately insufficient: the candidate issuer
/// from the first link must be pointer-identical to the child authenticated by
/// the reviewed parent link.
pub fn bind_x509_rsa_sha384_intermediate_path_to_reviewed_anchor<
    'child_certificate,
    'child_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
    'parent_child_certificate,
    'parent_child_encoded,
    'root_certificate,
    'root_encoded,
>(
    authenticated_child_link: VerifiedX509RsaSha384IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
    >,
    reviewed_parent_link: BoundX509RsaSha384ReviewedAnchorLink<
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >,
) -> Result<
    BoundX509RsaSha384ReviewedIntermediatePath<
        'child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >,
    X509RsaSha384ReviewedIntermediatePathError,
> {
    if !std::ptr::eq(
        authenticated_child_link.candidate_issuer(),
        reviewed_parent_link.verified_issuer_link().child(),
    ) {
        return Err(X509RsaSha384ReviewedIntermediatePathError::evidence_mismatch());
    }

    Ok(BoundX509RsaSha384ReviewedIntermediatePath {
        authenticated_child_link,
        reviewed_parent_link,
    })
}
