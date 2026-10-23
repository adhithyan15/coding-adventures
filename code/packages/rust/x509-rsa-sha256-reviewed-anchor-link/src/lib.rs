//! # RSA SHA-256 reviewed-anchor link binding
//!
//! This crate binds existing authenticated issuer-link evidence to existing
//! reviewed-root selection evidence for one exact issuer object. It can also
//! join that reviewed parent link to one authenticated child-to-intermediate
//! path-length transition through the same exact intermediate object. It
//! performs no new certificate parsing, signature work, root lookup, or path
//! search.

#![deny(unsafe_code)]

use core::fmt;
use x509_rsa_sha256_issuer_link::{
    ValidatedX509RsaSha256AuthenticatedIntermediatePathLength,
    VerifiedX509RsaSha256IssuerPolicyLink,
};
use x509_webpki_root_anchor_selection::SelectedReviewedWebPkiRootAnchor;

/// Stable, payload-free reviewed-anchor link binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256ReviewedAnchorLinkErrorKind {
    EvidenceMismatch,
}

/// A redacted reviewed-anchor link binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256ReviewedAnchorLinkError {
    kind: X509RsaSha256ReviewedAnchorLinkErrorKind,
}

impl X509RsaSha256ReviewedAnchorLinkError {
    const fn evidence_mismatch() -> Self {
        Self {
            kind: X509RsaSha256ReviewedAnchorLinkErrorKind::EvidenceMismatch,
        }
    }

    pub const fn kind(self) -> X509RsaSha256ReviewedAnchorLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha256ReviewedAnchorLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-256 reviewed-anchor link error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha256ReviewedAnchorLinkError {}

/// Stable, payload-free failures while joining two authenticated path links.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256ReviewedIntermediatePathErrorKind {
    EvidenceMismatch,
}

/// A redacted failure while joining through one exact intermediate object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256ReviewedIntermediatePathError {
    kind: X509RsaSha256ReviewedIntermediatePathErrorKind,
}

impl X509RsaSha256ReviewedIntermediatePathError {
    const fn evidence_mismatch() -> Self {
        Self {
            kind: X509RsaSha256ReviewedIntermediatePathErrorKind::EvidenceMismatch,
        }
    }

    pub const fn kind(self) -> X509RsaSha256ReviewedIntermediatePathErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha256ReviewedIntermediatePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-256 reviewed intermediate path error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha256ReviewedIntermediatePathError {}

/// Non-copyable evidence that one authenticated link terminates at the exact
/// certificate selected from the reviewed Mozilla root snapshot.
///
/// The retained issuer-link evidence already proves Name linkage, issuing-CA
/// role and critical-extension recognition, caller-owned algorithm and RSA key
/// policy, and the exact child signature for the fixed RSA SHA-256 path. The
/// retained anchor evidence proves only unique subject-and-SPKI membership in
/// the reviewed snapshot. This wrapper does not apply anchor name constraints,
/// validate target policy, construct a multi-hop path, or grant TLS authority.
pub struct BoundX509RsaSha256ReviewedAnchorLink<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
> {
    verified_issuer_link: VerifiedX509RsaSha256IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'issuer_certificate, 'issuer_encoded>,
}

impl<'child_certificate, 'child_encoded, 'issuer_certificate, 'issuer_encoded>
    BoundX509RsaSha256ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >
{
    pub const fn verified_issuer_link(
        &self,
    ) -> &VerifiedX509RsaSha256IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    > {
        &self.verified_issuer_link
    }

    pub const fn reviewed_anchor(
        &self,
    ) -> &SelectedReviewedWebPkiRootAnchor<'issuer_certificate, 'issuer_encoded> {
        &self.reviewed_anchor
    }
}

impl fmt::Debug for BoundX509RsaSha256ReviewedAnchorLink<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaSha256ReviewedAnchorLink")
    }
}

/// Bind one strongest authenticated RSA SHA-256 issuer link to one reviewed
/// root-anchor selection for the same exact issuer certificate object.
///
/// Byte equality is deliberately insufficient. Both evidence values must have
/// originated from the same decoded issuer object, preventing a caller from
/// mixing independently checked certificates that happen to encode equally.
/// Success proves only the retained narrow properties; it is not complete path
/// validation or TLS server authentication.
pub fn bind_x509_rsa_sha256_issuer_link_to_reviewed_anchor<
    'child_certificate,
    'child_encoded,
    'issuer_certificate,
    'issuer_encoded,
>(
    verified_issuer_link: VerifiedX509RsaSha256IssuerPolicyLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'issuer_certificate, 'issuer_encoded>,
) -> Result<
    BoundX509RsaSha256ReviewedAnchorLink<
        'child_certificate,
        'child_encoded,
        'issuer_certificate,
        'issuer_encoded,
    >,
    X509RsaSha256ReviewedAnchorLinkError,
> {
    let verified_issuer = verified_issuer_link
        .role_policy_link()
        .verified_link()
        .candidate_issuer();
    if !std::ptr::eq(verified_issuer, reviewed_anchor.candidate_certificate()) {
        return Err(X509RsaSha256ReviewedAnchorLinkError::evidence_mismatch());
    }

    Ok(BoundX509RsaSha256ReviewedAnchorLink {
        verified_issuer_link,
        reviewed_anchor,
    })
}

/// Non-copyable evidence joining one authenticated child-to-intermediate step
/// to one authenticated intermediate-to-reviewed-root step.
///
/// The first input retains complete policy for its exact intermediate together
/// with the resulting path-length state. The second retains that same
/// intermediate as its authenticated child and terminates at a uniquely
/// selected reviewed root. This wrapper keeps both inputs intact so later code
/// cannot silently discard either adjacent-link proof or the advanced state.
/// It performs no path search, target policy, anchor-constraint, or revocation
/// processing and grants no TLS or network authority.
pub struct BoundX509RsaSha256ReviewedIntermediatePath<
    'child_certificate,
    'child_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
    'parent_child_certificate,
    'parent_child_encoded,
    'root_certificate,
    'root_encoded,
> {
    authenticated_path: ValidatedX509RsaSha256AuthenticatedIntermediatePathLength<
        'child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
    >,
    reviewed_parent_link: BoundX509RsaSha256ReviewedAnchorLink<
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
    BoundX509RsaSha256ReviewedIntermediatePath<
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
    pub const fn authenticated_path(
        &self,
    ) -> &ValidatedX509RsaSha256AuthenticatedIntermediatePathLength<
        'child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
    > {
        &self.authenticated_path
    }

    pub const fn reviewed_parent_link(
        &self,
    ) -> &BoundX509RsaSha256ReviewedAnchorLink<
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    > {
        &self.reviewed_parent_link
    }
}

impl fmt::Debug for BoundX509RsaSha256ReviewedIntermediatePath<'_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509RsaSha256ReviewedIntermediatePath")
    }
}

/// Join two authenticated links only when they retain the same intermediate.
///
/// Comparing encoded bytes would permit independently decoded certificate
/// objects to cross this boundary. Pointer identity instead proves that the
/// complete intermediate policy and path-length transition from the first
/// input belong to the exact child authenticated by the reviewed-root link.
pub fn bind_x509_rsa_sha256_authenticated_intermediate_path_to_reviewed_anchor<
    'child_certificate,
    'child_encoded,
    'intermediate_certificate,
    'intermediate_encoded,
    'parent_child_certificate,
    'parent_child_encoded,
    'root_certificate,
    'root_encoded,
>(
    authenticated_path: ValidatedX509RsaSha256AuthenticatedIntermediatePathLength<
        'child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
    >,
    reviewed_parent_link: BoundX509RsaSha256ReviewedAnchorLink<
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >,
) -> Result<
    BoundX509RsaSha256ReviewedIntermediatePath<
        'child_certificate,
        'child_encoded,
        'intermediate_certificate,
        'intermediate_encoded,
        'parent_child_certificate,
        'parent_child_encoded,
        'root_certificate,
        'root_encoded,
    >,
    X509RsaSha256ReviewedIntermediatePathError,
> {
    let policy_intermediate = authenticated_path
        .authenticated_intermediate()
        .intermediate_policy()
        .certificate();
    let parent_link_child = reviewed_parent_link
        .verified_issuer_link()
        .role_policy_link()
        .verified_link()
        .child();
    if !std::ptr::eq(policy_intermediate, parent_link_child) {
        return Err(X509RsaSha256ReviewedIntermediatePathError::evidence_mismatch());
    }

    Ok(BoundX509RsaSha256ReviewedIntermediatePath {
        authenticated_path,
        reviewed_parent_link,
    })
}
