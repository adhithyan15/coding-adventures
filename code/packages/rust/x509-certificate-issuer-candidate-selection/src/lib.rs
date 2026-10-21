//! # Bounded X.509 issuer-candidate selection
//!
//! This crate owns one small path-construction rule: select exactly one decoded
//! candidate whose subject Name a caller-owned policy equates with the child's
//! issuer Name. It deliberately owns neither Name semantics nor certificate
//! authentication and trust.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate::X509Certificate;
use x509_name::X509Name;

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
