//! # Reviewed Web PKI root-anchor selection
//!
//! This crate binds one already decoded candidate certificate to at most one
//! exact subject-and-SPKI entry in the repository-reviewed Mozilla root
//! snapshot. It neither validates a certification path nor grants TLS trust.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate::X509Certificate;
use x509_webpki_roots_snapshot::{
    reviewed_webpki_roots, TrustAnchor, TrustAnchorSnapshot, TrustAnchorSnapshotErrorKind,
};

/// Stable, payload-free reviewed-anchor selection failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewedWebPkiRootAnchorSelectionErrorKind {
    Snapshot(TrustAnchorSnapshotErrorKind),
    NoMatch,
    AmbiguousMatch,
}

/// A redacted reviewed-anchor selection failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReviewedWebPkiRootAnchorSelectionError {
    kind: ReviewedWebPkiRootAnchorSelectionErrorKind,
}

impl ReviewedWebPkiRootAnchorSelectionError {
    const fn new(kind: ReviewedWebPkiRootAnchorSelectionErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> ReviewedWebPkiRootAnchorSelectionErrorKind {
        self.kind
    }
}

impl fmt::Display for ReviewedWebPkiRootAnchorSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "reviewed Web PKI root-anchor selection error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for ReviewedWebPkiRootAnchorSelectionError {}

/// Non-copyable evidence that one exact decoded certificate identity occurs
/// exactly once in the repository-reviewed root snapshot.
pub struct SelectedReviewedWebPkiRootAnchor<'certificate, 'encoded> {
    candidate_certificate: &'certificate X509Certificate<'encoded>,
    snapshot_index: usize,
    anchor: TrustAnchor<'static>,
}

impl<'certificate, 'encoded> SelectedReviewedWebPkiRootAnchor<'certificate, 'encoded> {
    pub const fn candidate_certificate(&self) -> &'certificate X509Certificate<'encoded> {
        self.candidate_certificate
    }

    pub const fn snapshot_index(&self) -> usize {
        self.snapshot_index
    }

    pub const fn subject_contents(&self) -> &'static [u8] {
        self.anchor.subject()
    }

    pub const fn subject_public_key_info_contents(&self) -> &'static [u8] {
        self.anchor.subject_public_key_info()
    }

    pub const fn name_constraints(&self) -> Option<&'static [u8]> {
        self.anchor.name_constraints()
    }
}

impl fmt::Debug for SelectedReviewedWebPkiRootAnchor<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SelectedReviewedWebPkiRootAnchor")
    }
}

/// Select a decoded candidate certificate from the exact reviewed snapshot.
///
/// Matching uses the candidate's canonical Name and SubjectPublicKeyInfo
/// sequence contents. The complete snapshot is scanned
/// so duplicate matches fail closed. Success establishes only reviewed-set
/// membership for that identity; it does not validate the candidate's role,
/// signature, time, constraints, path, revocation, or hostname.
pub fn select_reviewed_webpki_root_anchor<'certificate, 'encoded>(
    candidate_certificate: &'certificate X509Certificate<'encoded>,
) -> Result<
    SelectedReviewedWebPkiRootAnchor<'certificate, 'encoded>,
    ReviewedWebPkiRootAnchorSelectionError,
> {
    let identity = select_from_snapshot(
        reviewed_webpki_roots(),
        candidate_certificate.tbs_certificate().subject().contents(),
        candidate_certificate
            .tbs_certificate()
            .subject_public_key_info()
            .contents(),
    )?;

    Ok(SelectedReviewedWebPkiRootAnchor {
        candidate_certificate,
        snapshot_index: identity.snapshot_index,
        anchor: identity.anchor,
    })
}

#[cfg_attr(test, derive(Debug))]
struct SelectedIdentity<'snapshot> {
    snapshot_index: usize,
    anchor: TrustAnchor<'snapshot>,
}

fn select_from_snapshot<'snapshot>(
    snapshot: TrustAnchorSnapshot<'snapshot>,
    subject: &[u8],
    subject_public_key_info: &[u8],
) -> Result<SelectedIdentity<'snapshot>, ReviewedWebPkiRootAnchorSelectionError> {
    let mut selected = None;
    let mut match_count = 0usize;
    let mut anchor_count = 0usize;

    for (snapshot_index, anchor) in snapshot.iter().enumerate() {
        let anchor = anchor.map_err(|error| {
            ReviewedWebPkiRootAnchorSelectionError::new(
                ReviewedWebPkiRootAnchorSelectionErrorKind::Snapshot(error.kind()),
            )
        })?;
        anchor_count += 1;
        if anchor.subject() == subject
            && anchor.subject_public_key_info() == subject_public_key_info
        {
            match_count += 1;
            selected.get_or_insert(SelectedIdentity {
                snapshot_index,
                anchor,
            });
        }
    }

    if anchor_count != snapshot.expected_count() {
        return Err(ReviewedWebPkiRootAnchorSelectionError::new(
            ReviewedWebPkiRootAnchorSelectionErrorKind::Snapshot(
                TrustAnchorSnapshotErrorKind::RootCountMismatch,
            ),
        ));
    }

    match match_count {
        0 => Err(ReviewedWebPkiRootAnchorSelectionError::new(
            ReviewedWebPkiRootAnchorSelectionErrorKind::NoMatch,
        )),
        1 => Ok(selected.expect("one match stores one selected identity")),
        _ => Err(ReviewedWebPkiRootAnchorSelectionError::new(
            ReviewedWebPkiRootAnchorSelectionErrorKind::AmbiguousMatch,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAGIC: &[u8] = b"webpki-roots-canonical-v1\0";

    fn duplicate_snapshot() -> Vec<u8> {
        let mut encoded = MAGIC.to_vec();
        for _ in 0..2 {
            encoded.extend_from_slice(&7u64.to_be_bytes());
            encoded.extend_from_slice(b"subject");
            encoded.extend_from_slice(&3u64.to_be_bytes());
            encoded.extend_from_slice(b"key");
            encoded.push(0);
        }
        encoded
    }

    #[test]
    fn rejects_duplicate_identity_and_count_drift_after_complete_scan() {
        let encoded = duplicate_snapshot();
        let ambiguous = select_from_snapshot(
            TrustAnchorSnapshot::from_bytes(&encoded, 2),
            b"subject",
            b"key",
        )
        .unwrap_err();
        assert_eq!(
            ambiguous.kind(),
            ReviewedWebPkiRootAnchorSelectionErrorKind::AmbiguousMatch
        );

        let count_drift = select_from_snapshot(
            TrustAnchorSnapshot::from_bytes(&encoded, 3),
            b"subject",
            b"key",
        )
        .unwrap_err();
        assert_eq!(
            count_drift.kind(),
            ReviewedWebPkiRootAnchorSelectionErrorKind::Snapshot(
                TrustAnchorSnapshotErrorKind::RootCountMismatch
            )
        );
    }

    #[test]
    fn rejects_malformed_snapshot_before_returning_a_selection_result() {
        let error = select_from_snapshot(
            TrustAnchorSnapshot::from_bytes(b"not-the-reviewed-snapshot", 0),
            b"subject",
            b"key",
        )
        .unwrap_err();
        assert_eq!(
            error.kind(),
            ReviewedWebPkiRootAnchorSelectionErrorKind::Snapshot(
                TrustAnchorSnapshotErrorKind::InvalidMagic
            )
        );
    }
}
