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
//! An additive entry point applies one validated repository-owned Name-value
//! policy at every selection step and returns a distinct result retaining that
//! exact policy. The available value rules are still not complete RFC 5280 Name
//! equality.
//!
//! To support that later work without rerunning selection, a constructed path
//! can project one exact adjacent child/issuer pair at a time. The projection
//! remains borrowed from the path and grants no authentication by itself.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate::X509Certificate;
use x509_certificate_issuer_candidate_selection::{
    select_unique_x509_issuer_candidate_by, select_unique_x509_issuer_candidate_with_name_policy,
    X509IssuerCandidateSelectionErrorKind, X509NamePolicyIssuerCandidateSelectionErrorKind,
};
use x509_name::X509Name;
use x509_name_value_policy::X509NameValuePolicy;

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

/// Stable failures from policy-backed certificate-path construction.
///
/// The distinct type preserves whether bounds, path shape, issuer selection,
/// or Name-value comparison rejected construction without retaining any Name,
/// certificate, candidate-index, or policy-entry data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509NamePolicyCertificatePathConstructionErrorKind {
    InvalidCandidateLimit,
    CandidateLimitExceeded,
    InvalidPathCertificateLimit,
    UnsupportedPathCertificateLimit,
    IssuerSelection(X509NamePolicyIssuerCandidateSelectionErrorKind),
    CertificateCycle,
    PathCertificateLimitExceeded,
}

/// A redacted policy-backed certificate-path construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509NamePolicyCertificatePathConstructionError {
    kind: X509NamePolicyCertificatePathConstructionErrorKind,
}

impl X509NamePolicyCertificatePathConstructionError {
    const fn new(kind: X509NamePolicyCertificatePathConstructionErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509NamePolicyCertificatePathConstructionErrorKind {
        self.kind
    }
}

impl fmt::Display for X509NamePolicyCertificatePathConstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 Name-policy certificate-path construction error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509NamePolicyCertificatePathConstructionError {}

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

/// Non-copyable construction evidence retaining one exact Name-value policy.
///
/// This type is deliberately distinct from [`ConstructedX509CertificatePath`].
/// Only the policy-backed constructor can create it, so a path built with the
/// generic Boolean callback cannot substitute where later code requires the
/// exact validated policy that governed every issuer-selection step.
pub struct ConstructedX509CertificatePathWithNamePolicy<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
> {
    constructed_path:
        ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded>,
    name_policy: X509NameValuePolicy<'policy>,
}

impl<'target, 'target_encoded, 'candidates, 'candidate_encoded, 'policy>
    ConstructedX509CertificatePathWithNamePolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
    >
{
    /// Borrow the exact constructed path governed by the retained policy.
    pub const fn constructed_path(
        &self,
    ) -> &ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded>
    {
        &self.constructed_path
    }

    /// Return the exact validated Name-value policy used for construction.
    pub const fn name_policy(&self) -> X509NameValuePolicy<'policy> {
        self.name_policy
    }
}

impl fmt::Debug for ConstructedX509CertificatePathWithNamePolicy<'_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ConstructedX509CertificatePathWithNamePolicy")
    }
}

/// Non-copyable evidence for one exact adjacent link in a constructed path.
///
/// `issuer_depth` addresses the same selected-issuer position as
/// [`ConstructedX509CertificatePath::issuer_certificate`]. At depth zero the
/// child is the target. At every later depth the child is the preceding
/// selected candidate. Both endpoints are derived before this value is
/// returned, and the complete construction remains borrowed so the pair cannot
/// outlive or be detached from its retained ordering evidence.
///
/// This is adjacency evidence only. It does not verify the child's signature,
/// validate either certificate, establish trust, or apply path policy.
pub struct ConstructedX509CertificatePathAdjacentLink<
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
> {
    constructed_path: &'path ConstructedX509CertificatePath<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >,
    issuer_depth: usize,
    child_target_certificate: Option<&'target X509Certificate<'target_encoded>>,
    child_candidate_certificate: Option<&'candidates X509Certificate<'candidate_encoded>>,
    issuer_certificate: &'candidates X509Certificate<'candidate_encoded>,
}

impl<'path, 'target, 'target_encoded, 'candidates, 'candidate_encoded>
    ConstructedX509CertificatePathAdjacentLink<
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    >
{
    /// Return the exact complete construction from which this pair was derived.
    pub const fn constructed_path(
        &self,
    ) -> &'path ConstructedX509CertificatePath<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
    > {
        self.constructed_path
    }

    /// Return this pair's selected-issuer depth.
    pub const fn issuer_depth(&self) -> usize {
        self.issuer_depth
    }

    /// Return the target when it is this pair's child.
    ///
    /// This is `Some` exactly at issuer depth zero.
    pub const fn child_target_certificate(
        &self,
    ) -> Option<&'target X509Certificate<'target_encoded>> {
        self.child_target_certificate
    }

    /// Return the preceding selected candidate when it is this pair's child.
    ///
    /// This is `Some` exactly when the issuer depth is greater than zero.
    pub const fn child_candidate_certificate(
        &self,
    ) -> Option<&'candidates X509Certificate<'candidate_encoded>> {
        self.child_candidate_certificate
    }

    /// Return the exact selected candidate serving as this pair's issuer.
    pub const fn issuer_certificate(&self) -> &'candidates X509Certificate<'candidate_encoded> {
        self.issuer_certificate
    }
}

impl fmt::Debug for ConstructedX509CertificatePathAdjacentLink<'_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ConstructedX509CertificatePathAdjacentLink")
    }
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

    /// Project one exact child/issuer pair from the retained path order.
    ///
    /// No Name or terminal-policy callback is rerun. An out-of-range depth,
    /// including every depth for a terminal target, returns `None` without
    /// exposing partial evidence.
    pub fn adjacent_link<'path>(
        &'path self,
        issuer_depth: usize,
    ) -> Option<
        ConstructedX509CertificatePathAdjacentLink<
            'path,
            'target,
            'target_encoded,
            'candidates,
            'candidate_encoded,
        >,
    > {
        let issuer_certificate = self.issuer_certificate(issuer_depth)?;
        let (child_target_certificate, child_candidate_certificate) = if issuer_depth == 0 {
            (Some(self.target_certificate), None)
        } else {
            (None, Some(self.issuer_certificate(issuer_depth - 1)?))
        };

        Some(ConstructedX509CertificatePathAdjacentLink {
            constructed_path: self,
            issuer_depth,
            child_target_certificate,
            child_candidate_certificate,
            issuer_certificate,
        })
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
    construct_unique_x509_certificate_path_with_selector(
        target_certificate,
        candidate_certificates,
        maximum_candidates,
        maximum_path_certificates,
        &mut is_terminal,
        |certificate| {
            select_unique_x509_issuer_candidate_by(
                certificate,
                candidate_certificates,
                maximum_candidates,
                |issuer, subject| same_name(issuer, subject),
            )
            .map(|selected| selected.candidate_index())
            .map_err(|error| error.kind())
        },
    )
    .map_err(|error| {
        let kind = match error {
            PathConstructionFailure::InvalidCandidateLimit => {
                X509CertificatePathConstructionErrorKind::InvalidCandidateLimit
            }
            PathConstructionFailure::CandidateLimitExceeded => {
                X509CertificatePathConstructionErrorKind::CandidateLimitExceeded
            }
            PathConstructionFailure::InvalidPathCertificateLimit => {
                X509CertificatePathConstructionErrorKind::InvalidPathCertificateLimit
            }
            PathConstructionFailure::UnsupportedPathCertificateLimit => {
                X509CertificatePathConstructionErrorKind::UnsupportedPathCertificateLimit
            }
            PathConstructionFailure::IssuerSelection(kind) => {
                X509CertificatePathConstructionErrorKind::IssuerSelection(kind)
            }
            PathConstructionFailure::CertificateCycle => {
                X509CertificatePathConstructionErrorKind::CertificateCycle
            }
            PathConstructionFailure::PathCertificateLimitExceeded => {
                X509CertificatePathConstructionErrorKind::PathCertificateLimitExceeded
            }
        };
        X509CertificatePathConstructionError::new(kind)
    })
}

/// Construct one deterministic chain under one validated Name-value policy.
///
/// Candidate and path bounds are checked before terminal policy or Name policy
/// is invoked. Every non-terminal step delegates to the policy-backed unique
/// issuer selector, so the first Name-comparison failure aborts construction
/// while every successful comparison still scans the complete accepted pool
/// to reject ambiguity. Exact certificate-DER cycles and path-depth exhaustion
/// retain the same fail-closed semantics as the generic constructor.
///
/// Success retains both the constructed path and the exact validated policy.
/// The available value rules do not implement complete RFC 5280 Name equality,
/// and this evidence does not authenticate a link, validate a path, recognize
/// trust, check revocation, or grant TLS, HTTP, OAuth, or network authority.
pub fn construct_unique_x509_certificate_path_with_name_policy<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
>(
    target_certificate: &'target X509Certificate<'target_encoded>,
    candidate_certificates: &'candidates [X509Certificate<'candidate_encoded>],
    maximum_candidates: usize,
    maximum_path_certificates: usize,
    mut is_terminal: impl for<'certificate, 'encoded> FnMut(
        &'certificate X509Certificate<'encoded>,
    ) -> bool,
    name_policy: X509NameValuePolicy<'policy>,
) -> Result<
    ConstructedX509CertificatePathWithNamePolicy<
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
    >,
    X509NamePolicyCertificatePathConstructionError,
> {
    let constructed_path = construct_unique_x509_certificate_path_with_selector(
        target_certificate,
        candidate_certificates,
        maximum_candidates,
        maximum_path_certificates,
        &mut is_terminal,
        |certificate| {
            select_unique_x509_issuer_candidate_with_name_policy(
                certificate,
                candidate_certificates,
                maximum_candidates,
                name_policy,
            )
            .map(|selected| selected.candidate_index())
            .map_err(|error| error.kind())
        },
    )
    .map_err(|error| {
        let kind = match error {
            PathConstructionFailure::InvalidCandidateLimit => {
                X509NamePolicyCertificatePathConstructionErrorKind::InvalidCandidateLimit
            }
            PathConstructionFailure::CandidateLimitExceeded => {
                X509NamePolicyCertificatePathConstructionErrorKind::CandidateLimitExceeded
            }
            PathConstructionFailure::InvalidPathCertificateLimit => {
                X509NamePolicyCertificatePathConstructionErrorKind::InvalidPathCertificateLimit
            }
            PathConstructionFailure::UnsupportedPathCertificateLimit => {
                X509NamePolicyCertificatePathConstructionErrorKind::UnsupportedPathCertificateLimit
            }
            PathConstructionFailure::IssuerSelection(kind) => {
                X509NamePolicyCertificatePathConstructionErrorKind::IssuerSelection(kind)
            }
            PathConstructionFailure::CertificateCycle => {
                X509NamePolicyCertificatePathConstructionErrorKind::CertificateCycle
            }
            PathConstructionFailure::PathCertificateLimitExceeded => {
                X509NamePolicyCertificatePathConstructionErrorKind::PathCertificateLimitExceeded
            }
        };
        X509NamePolicyCertificatePathConstructionError::new(kind)
    })?;

    Ok(ConstructedX509CertificatePathWithNamePolicy {
        constructed_path,
        name_policy,
    })
}

enum PathConstructionFailure<SelectionFailure> {
    InvalidCandidateLimit,
    CandidateLimitExceeded,
    InvalidPathCertificateLimit,
    UnsupportedPathCertificateLimit,
    IssuerSelection(SelectionFailure),
    CertificateCycle,
    PathCertificateLimitExceeded,
}

fn construct_unique_x509_certificate_path_with_selector<
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    SelectionFailure,
>(
    target_certificate: &'target X509Certificate<'target_encoded>,
    candidate_certificates: &'candidates [X509Certificate<'candidate_encoded>],
    maximum_candidates: usize,
    maximum_path_certificates: usize,
    mut is_terminal: impl for<'certificate, 'encoded> FnMut(
        &'certificate X509Certificate<'encoded>,
    ) -> bool,
    mut select_issuer: impl for<'certificate, 'encoded> FnMut(
        &'certificate X509Certificate<'encoded>,
    ) -> Result<usize, SelectionFailure>,
) -> Result<
    ConstructedX509CertificatePath<'target, 'target_encoded, 'candidates, 'candidate_encoded>,
    PathConstructionFailure<SelectionFailure>,
> {
    if maximum_candidates == 0 {
        return Err(PathConstructionFailure::InvalidCandidateLimit);
    }
    if candidate_certificates.len() > maximum_candidates {
        return Err(PathConstructionFailure::CandidateLimitExceeded);
    }
    if maximum_path_certificates == 0 {
        return Err(PathConstructionFailure::InvalidPathCertificateLimit);
    }
    if maximum_path_certificates > MAX_X509_CONSTRUCTED_PATH_CERTIFICATES {
        return Err(PathConstructionFailure::UnsupportedPathCertificateLimit);
    }

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
            return Err(PathConstructionFailure::PathCertificateLimitExceeded);
        }

        let candidate_index = match current {
            CurrentCertificate::Target(certificate) => select_issuer(certificate),
            CurrentCertificate::Candidate(certificate) => select_issuer(certificate),
        }
        .map_err(PathConstructionFailure::IssuerSelection)?;

        let candidate = &candidate_certificates[candidate_index];
        if repeats_certificate(
            candidate,
            target_certificate,
            candidate_certificates,
            &candidate_indices,
            issuer_count,
        ) {
            return Err(PathConstructionFailure::CertificateCycle);
        }

        candidate_indices[issuer_count] = Some(candidate_index);
        issuer_count += 1;
        current = CurrentCertificate::Candidate(candidate);
    }
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
