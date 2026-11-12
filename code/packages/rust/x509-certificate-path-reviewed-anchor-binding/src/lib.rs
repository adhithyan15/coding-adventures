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
use x509_certificate_path_construction::{
    ConstructedX509CertificatePath, ConstructedX509CertificatePathWithNamePolicy,
};
use x509_certificate_path_length_policy::X509CertificatePathLengthState;
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

/// Policy-backed construction bound to its exact reviewed terminal object.
///
/// This evidence remains distinct from
/// [`BoundConstructedX509CertificatePathToReviewedAnchor`]. The complete
/// policy-backed construction is consumed and retained, so generic callback
/// construction cannot substitute and the exact Name-value policy cannot be
/// detached after binding.
pub struct BoundNamePolicyConstructedX509CertificatePathToReviewedAnchor<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
> {
    constructed_path_with_name_policy: ConstructedX509CertificatePathWithNamePolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
    >,
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded>,
}

impl<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
    >
    BoundNamePolicyConstructedX509CertificatePathToReviewedAnchor<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
    >
{
    /// Return the complete policy-backed construction retained by this evidence.
    pub const fn constructed_path_with_name_policy(
        &self,
    ) -> &ConstructedX509CertificatePathWithNamePolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
    > {
        &self.constructed_path_with_name_policy
    }

    /// Return the exact reviewed-root selection retained by this evidence.
    pub const fn reviewed_anchor(
        &self,
    ) -> &SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded> {
        &self.reviewed_anchor
    }
}

impl fmt::Debug
    for BoundNamePolicyConstructedX509CertificatePathToReviewedAnchor<'_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundNamePolicyConstructedX509CertificatePathToReviewedAnchor")
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

/// Construction-derived path-length capacity bound to a reviewed terminal.
///
/// RFC 5280 initializes path-length processing to the number of non-anchor
/// certificates between the target and trust anchor. This evidence derives
/// that capacity from the retained constructed path and keeps the complete
/// reviewed-terminal and issuing-CA role proof beside it. Callers therefore
/// cannot initialize traversal with detached or inflated state. This does not
/// authenticate a path link, process an intermediate, or establish complete
/// path validity.
pub struct InitializedReviewedConstructedX509CertificatePathLength<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    reviewed_path: BoundReviewedConstructedX509CertificatePathToAnchorRolePolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    path_length_state: X509CertificatePathLengthState,
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
    InitializedReviewedConstructedX509CertificatePathLength<
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
    /// Return the complete reviewed-terminal and issuing-CA role proof.
    #[must_use]
    pub const fn reviewed_path(
        &self,
    ) -> &BoundReviewedConstructedX509CertificatePathToAnchorRolePolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        &self.reviewed_path
    }

    /// Return the initial capacity derived from the retained construction.
    #[must_use]
    pub const fn path_length_state(&self) -> &X509CertificatePathLengthState {
        &self.path_length_state
    }
}

impl fmt::Debug
    for InitializedReviewedConstructedX509CertificatePathLength<'_, '_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("InitializedReviewedConstructedX509CertificatePathLength")
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
    validate_constructed_path_reviewed_anchor_binding(&constructed_path, &reviewed_anchor)?;

    Ok(BoundConstructedX509CertificatePathToReviewedAnchor {
        constructed_path,
        reviewed_anchor,
    })
}

/// Bind one policy-backed construction to its exact reviewed terminal object.
///
/// The same nontrivial-path and exact decoded-object checks as the generic
/// binder apply, but success consumes and retains the complete
/// [`ConstructedX509CertificatePathWithNamePolicy`]. No conversion from generic
/// construction evidence is available. The retained Name rules remain
/// incomplete for RFC 5280 Name equality, and success does not authenticate a
/// link, validate a path, or grant TLS, HTTP, OAuth, or network authority.
pub fn bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
>(
    constructed_path_with_name_policy: ConstructedX509CertificatePathWithNamePolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
    >,
    reviewed_anchor: SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded>,
) -> Result<
    BoundNamePolicyConstructedX509CertificatePathToReviewedAnchor<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
    >,
    X509CertificatePathReviewedAnchorBindingError,
> {
    validate_constructed_path_reviewed_anchor_binding(
        constructed_path_with_name_policy.constructed_path(),
        &reviewed_anchor,
    )?;

    Ok(
        BoundNamePolicyConstructedX509CertificatePathToReviewedAnchor {
            constructed_path_with_name_policy,
            reviewed_anchor,
        },
    )
}

fn validate_constructed_path_reviewed_anchor_binding<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
>(
    constructed_path: &ConstructedX509CertificatePath<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    reviewed_anchor: &SelectedReviewedWebPkiRootAnchor<'anchor_certificate, 'anchor_encoded>,
) -> Result<(), X509CertificatePathReviewedAnchorBindingError> {
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

    Ok(())
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

/// Initialize path-length state from one exact reviewed construction.
///
/// The reviewed-path binding guarantees at least one selected issuer and that
/// the last selected issuer is the exact reviewed root. Subtracting that trust
/// anchor from the construction's issuer count therefore yields the number of
/// prospective intermediate certificates. The construction crate's fixed
/// capacity bounds that value well below `u64::MAX`.
///
/// Success consumes and retains the complete reviewed-terminal role evidence,
/// preventing the construction-derived state from being silently attached to
/// another path. This is initialization only: it applies no intermediate
/// transition, signature, name, time, revocation, TLS, HTTP, OAuth, or network
/// policy.
#[must_use]
pub fn initialize_reviewed_constructed_x509_certificate_path_length<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
>(
    reviewed_path: BoundReviewedConstructedX509CertificatePathToAnchorRolePolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
) -> InitializedReviewedConstructedX509CertificatePathLength<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
> {
    let issuer_count = reviewed_path
        .reviewed_path()
        .constructed_path()
        .issuer_count();
    let intermediate_count = issuer_count.saturating_sub(1) as u64;

    InitializedReviewedConstructedX509CertificatePathLength {
        reviewed_path,
        path_length_state: X509CertificatePathLengthState::new(intermediate_count),
    }
}
