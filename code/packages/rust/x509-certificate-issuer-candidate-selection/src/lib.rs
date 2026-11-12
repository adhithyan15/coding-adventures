//! # Bounded X.509 issuer-candidate selection
//!
//! This crate owns one small path-construction rule: select exactly one decoded
//! candidate whose subject Name a validated caller-owned policy equates with
//! the child's issuer Name. A generic callback entry point remains available,
//! but later certificate code can use the data-driven repository-owned Name
//! policy directly. This crate deliberately owns neither complete Name
//! semantics nor certificate authentication and trust.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate::X509Certificate;
use x509_name::X509Name;
use x509_name_value_policy::{X509NameValueComparisonErrorKind, X509NameValuePolicy};

/// Stable, payload-free issuer-candidate selection failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509IssuerCandidateSelectionErrorKind {
    InvalidCandidateLimit,
    CandidateLimitExceeded,
    NoMatch,
    AmbiguousMatch,
}

/// A redacted issuer-candidate selection failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509IssuerCandidateSelectionError {
    kind: X509IssuerCandidateSelectionErrorKind,
}

impl X509IssuerCandidateSelectionError {
    const fn new(kind: X509IssuerCandidateSelectionErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509IssuerCandidateSelectionErrorKind {
        self.kind
    }
}

impl fmt::Display for X509IssuerCandidateSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 issuer-candidate selection error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509IssuerCandidateSelectionError {}

/// Stable failures from policy-backed issuer-candidate selection.
///
/// Nested categories preserve whether the selection boundary or Name-value
/// policy rejected the operation without retaining certificate or OID data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509NamePolicyIssuerCandidateSelectionErrorKind {
    Selection(X509IssuerCandidateSelectionErrorKind),
    NameComparison(X509NameValueComparisonErrorKind),
}

/// A redacted policy-backed issuer-candidate selection failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509NamePolicyIssuerCandidateSelectionError {
    kind: X509NamePolicyIssuerCandidateSelectionErrorKind,
}

impl X509NamePolicyIssuerCandidateSelectionError {
    const fn selection(kind: X509IssuerCandidateSelectionErrorKind) -> Self {
        Self {
            kind: X509NamePolicyIssuerCandidateSelectionErrorKind::Selection(kind),
        }
    }

    const fn name_comparison(kind: X509NameValueComparisonErrorKind) -> Self {
        Self {
            kind: X509NamePolicyIssuerCandidateSelectionErrorKind::NameComparison(kind),
        }
    }

    pub const fn kind(self) -> X509NamePolicyIssuerCandidateSelectionErrorKind {
        self.kind
    }
}

impl fmt::Display for X509NamePolicyIssuerCandidateSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 Name-policy issuer-candidate selection error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509NamePolicyIssuerCandidateSelectionError {}

/// Non-copyable evidence that caller policy matched exactly one candidate.
pub struct SelectedX509IssuerCandidate<'child, 'child_encoded, 'candidates, 'candidate_encoded> {
    child_certificate: &'child X509Certificate<'child_encoded>,
    candidate_certificate: &'candidates X509Certificate<'candidate_encoded>,
    candidate_index: usize,
}

impl<'child, 'child_encoded, 'candidates, 'candidate_encoded>
    SelectedX509IssuerCandidate<'child, 'child_encoded, 'candidates, 'candidate_encoded>
{
    pub const fn child_certificate(&self) -> &'child X509Certificate<'child_encoded> {
        self.child_certificate
    }

    pub const fn candidate_certificate(&self) -> &'candidates X509Certificate<'candidate_encoded> {
        self.candidate_certificate
    }

    pub const fn candidate_index(&self) -> usize {
        self.candidate_index
    }
}

impl fmt::Debug for SelectedX509IssuerCandidate<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SelectedX509IssuerCandidate")
    }
}

/// Non-copyable evidence that one exact Name policy matched one candidate.
///
/// This type is deliberately distinct from [`SelectedX509IssuerCandidate`],
/// which the generic callback API returns. Retaining the validated policy
/// prevents callback-created selection evidence from being substituted where
/// later code requires policy-backed provenance.
pub struct SelectedX509IssuerCandidateWithNamePolicy<
    'child,
    'child_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
> {
    child_certificate: &'child X509Certificate<'child_encoded>,
    candidate_certificate: &'candidates X509Certificate<'candidate_encoded>,
    candidate_index: usize,
    name_policy: X509NameValuePolicy<'policy>,
}

impl<'child, 'child_encoded, 'candidates, 'candidate_encoded, 'policy>
    SelectedX509IssuerCandidateWithNamePolicy<
        'child,
        'child_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
    >
{
    pub const fn child_certificate(&self) -> &'child X509Certificate<'child_encoded> {
        self.child_certificate
    }

    pub const fn candidate_certificate(&self) -> &'candidates X509Certificate<'candidate_encoded> {
        self.candidate_certificate
    }

    pub const fn candidate_index(&self) -> usize {
        self.candidate_index
    }

    pub const fn name_policy(&self) -> X509NameValuePolicy<'policy> {
        self.name_policy
    }
}

impl fmt::Debug for SelectedX509IssuerCandidateWithNamePolicy<'_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SelectedX509IssuerCandidateWithNamePolicy")
    }
}

/// Select exactly one candidate issuer under caller-owned Name policy.
///
/// `maximum_candidates` must be nonzero. Pools larger than that ceiling are
/// rejected before `same_name` is invoked. Every candidate in an accepted pool
/// is compared exactly once, in caller order, with the child's issuer Name as
/// the first argument and the candidate's subject Name as the second argument.
/// The complete pool is scanned so a later duplicate match fails closed.
///
/// Success proves only unique selection under this callback. It does not prove
/// complete RFC 5280 Name equality, a CA role, an authenticated issuer link, a
/// valid path, or trust.
pub fn select_unique_x509_issuer_candidate_by<
    'child,
    'child_encoded,
    'candidates,
    'candidate_encoded,
>(
    child_certificate: &'child X509Certificate<'child_encoded>,
    candidate_certificates: &'candidates [X509Certificate<'candidate_encoded>],
    maximum_candidates: usize,
    mut same_name: impl FnMut(&X509Name<'child_encoded>, &X509Name<'candidate_encoded>) -> bool,
) -> Result<
    SelectedX509IssuerCandidate<'child, 'child_encoded, 'candidates, 'candidate_encoded>,
    X509IssuerCandidateSelectionError,
> {
    if maximum_candidates == 0 {
        return Err(X509IssuerCandidateSelectionError::new(
            X509IssuerCandidateSelectionErrorKind::InvalidCandidateLimit,
        ));
    }
    if candidate_certificates.len() > maximum_candidates {
        return Err(X509IssuerCandidateSelectionError::new(
            X509IssuerCandidateSelectionErrorKind::CandidateLimitExceeded,
        ));
    }

    let child_issuer = child_certificate.tbs_certificate().issuer();
    let mut selected = None;
    let mut ambiguous = false;

    for (candidate_index, candidate_certificate) in candidate_certificates.iter().enumerate() {
        if same_name(
            child_issuer,
            candidate_certificate.tbs_certificate().subject(),
        ) {
            if selected.is_some() {
                ambiguous = true;
            } else {
                selected = Some((candidate_index, candidate_certificate));
            }
        }
    }

    if ambiguous {
        return Err(X509IssuerCandidateSelectionError::new(
            X509IssuerCandidateSelectionErrorKind::AmbiguousMatch,
        ));
    }

    let (candidate_index, candidate_certificate) = selected.ok_or_else(|| {
        X509IssuerCandidateSelectionError::new(X509IssuerCandidateSelectionErrorKind::NoMatch)
    })?;

    Ok(SelectedX509IssuerCandidate {
        child_certificate,
        candidate_certificate,
        candidate_index,
    })
}

/// Select exactly one candidate issuer under validated Name-value policy.
///
/// Candidate limits are checked before policy evaluation. Each accepted
/// candidate is compared in caller order with
/// [`X509NameValuePolicy::compare_names`]. The first policy error aborts the
/// scan and is returned as a payload-free category; successful comparisons
/// still scan the complete pool so a later duplicate match fails closed.
///
/// Success proves only unique selection under the exact supplied policy. The
/// currently available rules do not implement complete RFC 5280 Name equality:
/// in particular they provide no RFC 4518 normalization, case or whitespace
/// folding, schema-specific matching, or TeletexString mapping. This function
/// does not prove a CA role, authenticate a link, construct or validate a path,
/// recognize trust, or perform any I/O.
pub fn select_unique_x509_issuer_candidate_with_name_policy<
    'child,
    'child_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
>(
    child_certificate: &'child X509Certificate<'child_encoded>,
    candidate_certificates: &'candidates [X509Certificate<'candidate_encoded>],
    maximum_candidates: usize,
    name_policy: X509NameValuePolicy<'policy>,
) -> Result<
    SelectedX509IssuerCandidateWithNamePolicy<
        'child,
        'child_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
    >,
    X509NamePolicyIssuerCandidateSelectionError,
> {
    if maximum_candidates == 0 {
        return Err(X509NamePolicyIssuerCandidateSelectionError::selection(
            X509IssuerCandidateSelectionErrorKind::InvalidCandidateLimit,
        ));
    }
    if candidate_certificates.len() > maximum_candidates {
        return Err(X509NamePolicyIssuerCandidateSelectionError::selection(
            X509IssuerCandidateSelectionErrorKind::CandidateLimitExceeded,
        ));
    }

    let child_issuer = child_certificate.tbs_certificate().issuer();
    let mut selected = None;
    let mut ambiguous = false;

    for (candidate_index, candidate_certificate) in candidate_certificates.iter().enumerate() {
        let same_name = name_policy
            .compare_names(
                child_issuer,
                candidate_certificate.tbs_certificate().subject(),
            )
            .map_err(|error| {
                X509NamePolicyIssuerCandidateSelectionError::name_comparison(error.kind())
            })?;
        if same_name {
            if selected.is_some() {
                ambiguous = true;
            } else {
                selected = Some((candidate_index, candidate_certificate));
            }
        }
    }

    if ambiguous {
        return Err(X509NamePolicyIssuerCandidateSelectionError::selection(
            X509IssuerCandidateSelectionErrorKind::AmbiguousMatch,
        ));
    }

    let (candidate_index, candidate_certificate) = selected.ok_or_else(|| {
        X509NamePolicyIssuerCandidateSelectionError::selection(
            X509IssuerCandidateSelectionErrorKind::NoMatch,
        )
    })?;

    Ok(SelectedX509IssuerCandidateWithNamePolicy {
        child_certificate,
        candidate_certificate,
        candidate_index,
        name_policy,
    })
}
