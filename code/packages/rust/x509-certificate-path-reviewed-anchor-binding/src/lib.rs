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
use x509_webpki_root_anchor_selection::SelectedReviewedWebPkiRootAnchor;

/// Stable, payload-free reviewed-anchor binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509CertificatePathReviewedAnchorBindingErrorKind {
    /// Construction stopped at the target without selecting an issuer.
    TargetIsTerminal,
    /// The reviewed-root evidence belongs to a different decoded object.
    EvidenceMismatch,
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
