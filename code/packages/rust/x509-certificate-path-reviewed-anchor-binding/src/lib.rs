//! # Constructed-path reviewed-anchor binding
//!
//! Certificate-path construction and reviewed-root selection produce separate
//! evidence objects. This crate joins those objects only when the constructed
//! path selected at least one issuer and its final selected candidate is the
//! exact decoded certificate object retained by the reviewed-root evidence.
//! Pointer identity deliberately rejects a byte-equal certificate produced by
//! another decode, preventing evidence for one object from being substituted
//! for the object actually selected into the path.
//!
//! Binding is not validation. Success does not authenticate any path link or
//! establish certificate role, self-signature, algorithm, time, constraints,
//! name, revocation, TLS, HTTP, OAuth, or network policy.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate_path_construction::ConstructedX509CertificatePath;
use x509_certificate_policy::{ValidatedCertificateRolePolicy, X509CertificateRole};
use x509_webpki_root_anchor_selection::SelectedReviewedWebPkiRootAnchor;

/// Stable, payload-free reviewed-anchor binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509CertificatePathReviewedAnchorBindingErrorKind {
    /// Construction stopped at the target without selecting an issuer.
    TargetIsTerminal,
    /// The reviewed-root evidence belongs to a different decoded object.
    EvidenceMismatch,
    /// The reviewed terminal and role policy retain different decoded objects.
    RoleEvidenceMismatch,
    /// The reviewed terminal's complete role policy does not authorize a CA.
    AnchorNotCertificateAuthority,
}

/// A redacted constructed-path reviewed-anchor binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509CertificatePathReviewedAnchorBindingError {
    kind: X509CertificatePathReviewedAnchorBindingErrorKind,
}

impl X509CertificatePathReviewedAnchorBindingError {
    const fn new(kind: X509CertificatePathReviewedAnchorBindingErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category without certificate payloads.
    pub const fn kind(self) -> X509CertificatePathReviewedAnchorBindingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509CertificatePathReviewedAnchorBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate path reviewed-anchor binding error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509CertificatePathReviewedAnchorBindingError {}

/// Non-copyable evidence retaining both inputs to an exact terminal binding.
///
/// Keeping both evidence values prevents callers from changing the constructed
/// candidate pool or substituting different reviewed-root evidence after the
/// identity check succeeds.
pub struct BoundConstructedX509CertificatePathToReviewedAnchor<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
> {
    constructed_path:
        ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded>,
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded>,
}

impl<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >
    BoundConstructedX509CertificatePathToReviewedAnchor<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >
{
    /// Return the exact constructed path retained by this evidence.
    pub const fn constructed_path(
        &self,
    ) -> &ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded>
    {
        &self.constructed_path
    }

    /// Return the exact reviewed-root selection retained by this evidence.
    pub const fn reviewed_anchor(
        &self,
    ) -> &SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded> {
        &self.reviewed_anchor
    }
}

impl fmt::Debug for BoundConstructedX509CertificatePathToReviewedAnchor<'_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundConstructedX509CertificatePathToReviewedAnchor")
    }
}

/// Non-copyable reviewed-terminal evidence with exact issuing-CA role policy.
///
/// The retained reviewed path preserves the target, candidate pool, ordered
/// selected indices, and exact reviewed-root selection. The retained outer
/// role-policy evidence proves certificate-wide critical-extension recognition
/// and issuing-CA semantics for that same decoded terminal object. This value
/// does not authenticate a path link or establish complete path validity.
pub struct BoundReviewedConstructedX509CertificatePathToAnchorRolePolicy<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    reviewed_path: BoundConstructedX509CertificatePathToReviewedAnchor<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >,
    anchor_role_policy: ValidatedCertificateRolePolicy<'role_certificate, 'role_encoded>,
}

impl<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
    BoundReviewedConstructedX509CertificatePathToAnchorRolePolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >
{
    /// Return the exact reviewed-terminal-bound path.
    #[must_use]
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

    /// Return complete issuing-CA role policy for the exact reviewed terminal.
    #[must_use]
    pub const fn anchor_role_policy(
        &self,
    ) -> &ValidatedCertificateRolePolicy<'role_certificate, 'role_encoded> {
        &self.anchor_role_policy
    }
}

impl fmt::Debug
    for BoundReviewedConstructedX509CertificatePathToAnchorRolePolicy<
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
        formatter.write_str("BoundReviewedConstructedX509CertificatePathToAnchorRolePolicy")
    }
}

/// Bind a constructed path's exact terminal candidate to reviewed-root evidence.
///
/// A target-only construction is rejected because this boundary is intended to
/// bind a server path's selected issuer terminus. For a nontrivial path, the
/// final selected candidate must be the same decoded certificate object held
/// by `reviewed_anchor`; matching bytes or matching root identity alone are not
/// sufficient.
///
/// Success remains composition evidence only. It does not validate the
/// constructed path or make it suitable for TLS or OAuth transport.
pub fn bind_constructed_x509_certificate_path_to_reviewed_anchor<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
>(
    constructed_path: ConstructedX509CertificatePath<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded>,
) -> Result<
    BoundConstructedX509CertificatePathToReviewedAnchor<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >,
    X509CertificatePathReviewedAnchorBindingError,
> {
    let Some(terminal_depth) = constructed_path.issuer_count().checked_sub(1) else {
        return Err(X509CertificatePathReviewedAnchorBindingError::new(
            X509CertificatePathReviewedAnchorBindingErrorKind::TargetIsTerminal,
        ));
    };
    let Some(terminal_certificate) = constructed_path.issuer_certificate(terminal_depth) else {
        return Err(X509CertificatePathReviewedAnchorBindingError::new(
            X509CertificatePathReviewedAnchorBindingErrorKind::EvidenceMismatch,
        ));
    };

    if !std::ptr::eq(
        terminal_certificate,
        reviewed_anchor.candidate_certificate(),
    ) {
        return Err(X509CertificatePathReviewedAnchorBindingError::new(
            X509CertificatePathReviewedAnchorBindingErrorKind::EvidenceMismatch,
        ));
    }

    Ok(BoundConstructedX509CertificatePathToReviewedAnchor {
        constructed_path,
        reviewed_anchor,
    })
}

/// Bind complete issuing-CA role policy to the exact reviewed terminal.
///
/// Object identity is checked before the role, so policy for another decoded
/// certificate cannot authorize this terminal. The outer policy is consumed
/// whole so its certificate-wide critical-extension recognition cannot be
/// detached from the issuing-CA result. Success remains evidence composition
/// only; it verifies no signature and grants no complete path or trust policy.
pub fn bind_reviewed_constructed_x509_certificate_path_to_anchor_role_policy<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    reviewed_path: BoundConstructedX509CertificatePathToReviewedAnchor<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
    >,
    anchor_role_policy: ValidatedCertificateRolePolicy<'role_certificate, 'role_encoded>,
) -> Result<
    BoundReviewedConstructedX509CertificatePathToAnchorRolePolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    X509CertificatePathReviewedAnchorBindingError,
> {
    if !core::ptr::eq(
        reviewed_path.reviewed_anchor().candidate_certificate(),
        anchor_role_policy.certificate(),
    ) {
        return Err(X509CertificatePathReviewedAnchorBindingError::new(
            X509CertificatePathReviewedAnchorBindingErrorKind::RoleEvidenceMismatch,
        ));
    }
    if anchor_role_policy.role_extensions().role()
        != X509CertificateRole::IssuingCertificateAuthority
    {
        return Err(X509CertificatePathReviewedAnchorBindingError::new(
            X509CertificatePathReviewedAnchorBindingErrorKind::AnchorNotCertificateAuthority,
        ));
    }

    Ok(
        BoundReviewedConstructedX509CertificatePathToAnchorRolePolicy {
            reviewed_path,
            anchor_role_policy,
        },
    )
}
