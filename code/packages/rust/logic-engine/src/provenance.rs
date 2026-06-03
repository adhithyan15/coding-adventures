//! # Provenance — first-class citations for every clause.
//!
//! Addresses ADJ46 awkwardness item **A2**: "Provenance is not a
//! clause field." Each prior, contribution, and joint contribution
//! now carries a [`Provenance`] value that names the authority the
//! clause is grounded in. The proof DAG threads this through every
//! step's [`crate::DerivationOrigin`] so an audit-trail consumer can
//! print, for every contribution that fired, the exact citation it
//! came from — without consulting a side-table.
//!
//! The shape is intentionally small. ADJ44's recursive-rulebook
//! derivation produces a richer object; this is the minimum the
//! engine needs to be able to *cite* a clause back to its origin.
//! Richer provenance (recursion depth, derivation tree, content-match
//! verification status) lives at the rulebook-acquisition layer and
//! is referenced from here by `source` + `locator`.

/// Citation + trust signal for a single clause.
///
/// Designed so the common case is a one-liner — `Provenance::cited("AHA 2021
/// chest-pain guideline §3.2")` — while still carrying enough
/// structure that an audit reader can sort, filter, and aggregate
/// across clauses by trust tier.
///
/// `Provenance::unattributed()` is the explicit sentinel for "this
/// clause has no citation." It is preserved as a value rather than
/// `None` so the audit trail can distinguish "no one bothered to
/// attribute" from "the modeller explicitly committed to a fact
/// without external grounding" at proof-display time.
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    /// Human-readable citation. Typically a journal reference, a
    /// guideline name + year, a statute, a clinical trial id, etc.
    /// Empty string only when `trust_tier == TrustTier::Unattributed`.
    pub source: String,
    /// Optional locator within the source — page, section, paragraph,
    /// figure number, court-opinion page-line.
    pub locator: Option<String>,
    /// How much weight a reviewer should place on this clause.
    pub trust_tier: TrustTier,
}

/// A coarse trust-tier rank. The variants are ordered from highest
/// (consensus across multiple authoritative sources) to lowest
/// (modeller intuition with no external grounding).
///
/// `Eq + Ord` is derived deliberately: tools that aggregate proofs
/// can sort by trust tier without re-implementing the ordering, and
/// the order in the source determines the order — `Consensus <
/// Authoritative < Empirical < Inferred < Unattributed` by `PartialOrd`,
/// which corresponds to "more trustworthy is smaller" so a min-tier
/// reduction picks the strongest provenance in a proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TrustTier {
    /// Multi-society / cross-authority consensus. E.g. AHA + ESC
    /// + ACC agreeing on a treatment recommendation.
    Consensus,
    /// Single authoritative source — a guideline, a statute, a
    /// peer-reviewed meta-analysis. The "default reasonable" tier
    /// for most clauses.
    Authoritative,
    /// Cohort study, case series, retrospective analysis. Lower
    /// power than `Authoritative` but still empirically grounded.
    Empirical,
    /// Derived by an LLM or by a knowledge engineer from training
    /// data, not from an external citation. ADJ44's recursive
    /// rulebook derivation marks rules at this tier when no
    /// external authority has been resolved.
    Inferred,
    /// No citation. The clause is asserted by the modeller without
    /// external grounding. Distinct from "citation not yet
    /// recorded" — the modeller explicitly committed to no source.
    Unattributed,
}

impl Provenance {
    /// Construct a `Provenance` with all three fields explicit.
    pub fn new(
        source: impl Into<String>,
        locator: Option<String>,
        trust_tier: TrustTier,
    ) -> Self {
        Self {
            source: source.into(),
            locator,
            trust_tier,
        }
    }

    /// The common case: a single-line citation at
    /// [`TrustTier::Authoritative`]. Used for the bulk of
    /// guideline-derived clauses.
    pub fn cited(source: impl Into<String>) -> Self {
        Self::new(source, None, TrustTier::Authoritative)
    }

    /// A citation at consensus tier.
    pub fn consensus(source: impl Into<String>) -> Self {
        Self::new(source, None, TrustTier::Consensus)
    }

    /// Empirical observation; the bulk of LR magnitudes in the
    /// HEART-score / Panju 1998 literature land here.
    pub fn empirical(source: impl Into<String>) -> Self {
        Self::new(source, None, TrustTier::Empirical)
    }

    /// No external grounding. Use this when adapting a rulebook
    /// from a non-citing source or when the modeller is explicitly
    /// committing to a fact on their own authority.
    pub fn unattributed() -> Self {
        Self::new(String::new(), None, TrustTier::Unattributed)
    }

    /// Convenience: attach a locator to an existing provenance.
    /// Returns a new value, leaves the receiver unchanged.
    pub fn with_locator(mut self, locator: impl Into<String>) -> Self {
        self.locator = Some(locator.into());
        self
    }
}

impl Default for Provenance {
    /// The default is [`Provenance::unattributed`], not a panic.
    /// Defaulting to "no citation" is the right behaviour for tests
    /// and small examples where citing every clause would be noise;
    /// real rulebooks should set this explicitly via the constructor.
    fn default() -> Self {
        Self::unattributed()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cited_lands_at_authoritative_tier() {
        let p = Provenance::cited("AHA 2021 chest-pain guideline");
        assert_eq!(p.trust_tier, TrustTier::Authoritative);
        assert!(p.locator.is_none());
        assert_eq!(p.source, "AHA 2021 chest-pain guideline");
    }

    #[test]
    fn unattributed_has_empty_source() {
        let p = Provenance::unattributed();
        assert_eq!(p.trust_tier, TrustTier::Unattributed);
        assert!(p.source.is_empty());
    }

    #[test]
    fn trust_tier_ord_puts_consensus_lowest_value() {
        // "Lower variant => stronger trust" is the convention so
        // that min(...) over a proof's contributions picks the
        // strongest provenance in evidence.
        assert!(TrustTier::Consensus < TrustTier::Authoritative);
        assert!(TrustTier::Authoritative < TrustTier::Empirical);
        assert!(TrustTier::Empirical < TrustTier::Inferred);
        assert!(TrustTier::Inferred < TrustTier::Unattributed);
    }

    #[test]
    fn with_locator_threads_immutably() {
        let p = Provenance::cited("ACOG bulletin").with_locator("§4.2");
        assert_eq!(p.locator.as_deref(), Some("§4.2"));
        // Original `cited` still has no locator if reused:
        let q = Provenance::cited("ACOG bulletin");
        assert!(q.locator.is_none());
    }

    #[test]
    fn default_is_unattributed() {
        assert_eq!(Provenance::default(), Provenance::unattributed());
    }
}
