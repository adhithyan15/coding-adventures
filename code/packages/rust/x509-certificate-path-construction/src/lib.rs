//! # Bounded unique-chain X.509 path construction
//!
//! Path construction and path validation are different jobs. Construction
//! answers only "which ordered certificates did this deterministic policy
//! choose?" This crate repeats the repository-owned unique issuer selector
//! until caller policy identifies a terminal certificate. The result retains
//! the target, candidate pool, and selected indices without allocating from
//! certificate input.
//!
//! The caller still owns distinguished-name semantics and the meaning of
//! "terminal". In particular, a terminal callback is not a trust decision made
//! by this crate. Later validation must authenticate every link and apply role,
//! algorithm, time, path-length, name-constraint, revocation, and trust policy.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate::X509Certificate;
use x509_certificate_issuer_candidate_selection::{
    select_unique_x509_issuer_candidate_by, X509IssuerCandidateSelectionErrorKind,
};
use x509_name::X509Name;

/// Fixed storage ceiling for a constructed path, including its target.
///
/// Callers choose a smaller active limit per construction. The fixed ceiling
/// prevents a wire-controlled path from causing allocation or unbounded stack
/// growth while remaining far above ordinary public-Web-PKI chain lengths.
pub const MAX_X509_CONSTRUCTED_PATH_CERTIFICATES: usize = 64;

/// Stable, payload-free construction failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509CertificatePathConstructionErrorKind {
    InvalidCandidateLimit,
    CandidateLimitExceeded,
    InvalidPathCertificateLimit,
    UnsupportedPathCertificateLimit,
    IssuerSelection(X509IssuerCandidateSelectionErrorKind),
    CertificateCycle,
    PathCertificateLimitExceeded,
}

/// A redacted certificate-path construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509CertificatePathConstructionError {
    kind: X509CertificatePathConstructionErrorKind,
}

impl X509CertificatePathConstructionError {
    const fn new(kind: X509CertificatePathConstructionErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509CertificatePathConstructionErrorKind {
        self.kind
    }
}

impl fmt::Display for X509CertificatePathConstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 certificate path construction error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509CertificatePathConstructionError {}

/// Non-copyable evidence retaining one exact, ordered constructed chain.
///
/// Index zero in the logical chain is always `target_certificate`. Every later
/// certificate is addressed by the corresponding entry in `candidate_indices`.
/// Keeping the pool borrowed prevents a caller from replacing a certificate
/// after construction while reusing the same numeric indices.
pub struct ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded>
{
    target_certificate: &'target X509Certificate<'target_encoded>,
    candidate_certificates: &'candidates [X509Certificate<'candidate_encoded>],
    candidate_indices: [Option<usize>; MAX_X509_CONSTRUCTED_PATH_CERTIFICATES],
    issuer_count: usize,
}

impl<'target, 'target_encoded, 'candidates, 'candidate_encoded>
    ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded>
{
    pub const fn target_certificate(&self) -> &'target X509Certificate<'target_encoded> {
        self.target_certificate
    }

    pub const fn candidate_certificates(
        &self,
    ) -> &'candidates [X509Certificate<'candidate_encoded>] {
        self.candidate_certificates
    }

    /// Number of selected issuers after the target.
    pub const fn issuer_count(&self) -> usize {
        self.issuer_count
    }

    /// Number of certificates in the logical path, including the target.
    pub const fn certificate_count(&self) -> usize {
        self.issuer_count + 1
    }

    pub const fn target_is_terminal(&self) -> bool {
        self.issuer_count == 0
    }

    /// Candidate-pool index for one issuer depth, where zero is the target's
    /// direct issuer.
    pub fn candidate_index(&self, issuer_depth: usize) -> Option<usize> {
        if issuer_depth >= self.issuer_count {
            return None;
        }
        self.candidate_indices[issuer_depth]
    }

    pub fn candidate_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.candidate_indices[..self.issuer_count]
            .iter()
            .copied()
            .flatten()
    }

    pub fn issuer_certificate(
        &self,
        issuer_depth: usize,
    ) -> Option<&'candidates X509Certificate<'candidate_encoded>> {
        let candidate_index = self.candidate_index(issuer_depth)?;
        self.candidate_certificates.get(candidate_index)
    }
}

impl fmt::Debug for ConstructedX509CertificatePath<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ConstructedX509CertificatePath")
    }
}

enum CurrentCertificate<'target, 'target_encoded, 'candidates, 'candidate_encoded> {
    Target(&'target X509Certificate<'target_encoded>),
    Candidate(&'candidates X509Certificate<'candidate_encoded>),
}

/// Construct one deterministic target-to-terminal certificate chain.
///
/// All limits are checked before either policy callback. For every accepted
/// path certificate, `is_terminal` is invoked exactly once before any issuer
/// selection. Every non-terminal step delegates to the unique issuer selector,
/// which scans the complete accepted pool and invokes `same_name` exactly once
/// per candidate with child issuer first and candidate subject second.
///
/// Exact complete-certificate DER equality defines cycle identity. This closes
/// both repeated-index cycles and cycles through a separately decoded copy of
/// the same certificate. A terminal certificate at the active depth limit is
/// accepted; a non-terminal one fails before another selection callback.
///
/// Success is construction evidence only. It does not authenticate links,
/// identify a trust anchor, or validate any certificate or path policy.
pub fn construct_unique_x509_certificate_path_by<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
>(
    target_certificate: &'target X509Certificate<'target_encoded>,
    candidate_certificates: &'candidates [X509Certificate<'candidate_encoded>],
    maximum_candidates: usize,
    maximum_path_certificates: usize,
    mut is_terminal: impl for<'certificate, 'encoded> FnMut(
        &'certificate X509Certificate<'encoded>,
    ) -> bool,
    mut same_name: impl for<'left, 'right> FnMut(&X509Name<'left>, &X509Name<'right>) -> bool,
) -> Result<
    ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded>,
    X509CertificatePathConstructionError,
> {
    validate_limits(
        candidate_certificates.len(),
        maximum_candidates,
        maximum_path_certificates,
    )?;

    let mut candidate_indices = [None; MAX_X509_CONSTRUCTED_PATH_CERTIFICATES];
    let mut issuer_count = 0usize;
    let mut current = CurrentCertificate::Target(target_certificate);

    loop {
        let terminal = match current {
            CurrentCertificate::Target(certificate) => is_terminal(certificate),
            CurrentCertificate::Candidate(certificate) => is_terminal(certificate),
        };
        if terminal {
            return Ok(ConstructedX509CertificatePath {
                target_certificate,
                candidate_certificates,
                candidate_indices,
                issuer_count,
            });
        }

        if issuer_count + 1 == maximum_path_certificates {
            return Err(X509CertificatePathConstructionError::new(
                X509CertificatePathConstructionErrorKind::PathCertificateLimitExceeded,
            ));
        }

        let candidate_index = match current {
            CurrentCertificate::Target(certificate) => select_unique_x509_issuer_candidate_by(
                certificate,
                candidate_certificates,
                maximum_candidates,
                |issuer, subject| same_name(issuer, subject),
            ),
            CurrentCertificate::Candidate(certificate) => select_unique_x509_issuer_candidate_by(
                certificate,
                candidate_certificates,
                maximum_candidates,
                |issuer, subject| same_name(issuer, subject),
            ),
        }
        .map_err(|error| {
            X509CertificatePathConstructionError::new(
                X509CertificatePathConstructionErrorKind::IssuerSelection(error.kind()),
            )
        })?
        .candidate_index();

        let candidate = &candidate_certificates[candidate_index];
        if repeats_certificate(
            candidate,
            target_certificate,
            candidate_certificates,
            &candidate_indices,
            issuer_count,
        ) {
            return Err(X509CertificatePathConstructionError::new(
                X509CertificatePathConstructionErrorKind::CertificateCycle,
            ));
        }

        candidate_indices[issuer_count] = Some(candidate_index);
        issuer_count += 1;
        current = CurrentCertificate::Candidate(candidate);
    }
}

fn validate_limits(
    candidate_count: usize,
    maximum_candidates: usize,
    maximum_path_certificates: usize,
) -> Result<(), X509CertificatePathConstructionError> {
    if maximum_candidates == 0 {
        return Err(X509CertificatePathConstructionError::new(
            X509CertificatePathConstructionErrorKind::InvalidCandidateLimit,
        ));
    }
    if candidate_count > maximum_candidates {
        return Err(X509CertificatePathConstructionError::new(
            X509CertificatePathConstructionErrorKind::CandidateLimitExceeded,
        ));
    }
    if maximum_path_certificates == 0 {
        return Err(X509CertificatePathConstructionError::new(
            X509CertificatePathConstructionErrorKind::InvalidPathCertificateLimit,
        ));
    }
    if maximum_path_certificates > MAX_X509_CONSTRUCTED_PATH_CERTIFICATES {
        return Err(X509CertificatePathConstructionError::new(
            X509CertificatePathConstructionErrorKind::UnsupportedPathCertificateLimit,
        ));
    }
    Ok(())
}

fn repeats_certificate(
    candidate: &X509Certificate<'_>,
    target: &X509Certificate<'_>,
    candidate_certificates: &[X509Certificate<'_>],
    selected_indices: &[Option<usize>; MAX_X509_CONSTRUCTED_PATH_CERTIFICATES],
    selected_count: usize,
) -> bool {
    candidate.encoded() == target.encoded()
        || selected_indices[..selected_count]
            .iter()
            .copied()
            .flatten()
            .any(|index| candidate.encoded() == candidate_certificates[index].encoded())
}
