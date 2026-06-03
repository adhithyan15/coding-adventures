//! # LR aggregation — likelihood-ratio Bayesian posterior inference.
//!
//! This module implements [LP19e](../../../specs/LP19e-likelihood-ratio-aggregation.md):
//! a separate-from-WMC inference algorithm for the
//! `prior + Σ contributes(LR, …)` rulebook shape that
//! [ADJ14](../../../specs/ADJ14-probabilistic-ir-semantics.md) commits
//! the adjudication framework to.
//!
//! ## What LR aggregation is, in one paragraph
//!
//! Each conclusion carries one Bayesian prior `prior(p, c)`. Every
//! independent piece of evidence carries a likelihood ratio
//! `contributes(LR, evidence_term, c)`. When the KB *observes* an
//! evidence term (via a Certain Fact, see [`KnowledgeBase::observed_evidence`]),
//! the contribution's `log(LR)` is added to the running log-odds.
//! Joint evidence interactions — synergy or explaining-away — are
//! handled by `contributes_jointly(LR, [e1, …, en], c)`, which adds
//! `log(joint LR)` to the log-odds iff *every* term in `evidence_set`
//! is observed. The posterior is `sigmoid(prior_logit + Σ deltas)`.
//!
//! ## Why this exists alongside WMC
//!
//! The weighted-model-counting engine in [`crate::weighted_model_count`]
//! is the right answer for joint conjunctive probabilistic programs:
//! a clause body fires iff every literal is independently true, and
//! the answer is a sum over possible-worlds probability mass. That is
//! the wrong answer for `prior + contributes` rulebooks, where the
//! inference shape is fundamentally Bayesian log-odds composition and
//! the math collapses to a linear-time sum rather than a 2ⁿ-world
//! enumeration. ADJ46's awkwardness catalogue (item A6) found this
//! by hand: the engine returned the WMC posterior and the
//! ACS-rulebook demo had to throw it away and aggregate again in
//! user code. LP19e is the engine paying that aggregation cost itself
//! at linear time, and emitting a proof DAG that names every
//! contribution.

use logic_core::{Substitution, Term};

use crate::{
    ContributionClauseId, FactId, JointContributionClauseId, KnowledgeBase, PriorClauseId,
    Provenance,
};
use crate::proof_dag::{DerivationOrigin, Proof, ProofDAG, ProofStep};

// ---------------------------------------------------------------------------
// Clause types
// ---------------------------------------------------------------------------

/// A Bayesian prior probability for a conclusion atom.
///
/// Required at most once per conclusion that participates in LR
/// aggregation. Adding a second prior for the same conclusion is
/// rejected by [`KnowledgeBase::add_prior`] with a
/// [`KbError::ConflictingPriors`] error.
///
/// The `prior_logit` field stores `log(p / (1 - p))`. This is
/// numerically friendlier than the probability for two reasons:
/// 1. Sums of log-odds compose evidence multiplicatively in
///    odds-space, which is exactly the LR aggregation operation.
/// 2. It avoids underflow / overflow at the extremes of the
///    probability scale; LP19e's [`sigmoid`] back-converts at the
///    end with a branch that's numerically stable for very large
///    negative inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct PriorClause {
    pub id: PriorClauseId,
    pub conclusion: Term,
    pub prior_logit: f64,
    /// LP19e + ADJ47-B: citation for this prior. Default is
    /// [`Provenance::unattributed`] when constructed via
    /// `PriorClause::new` / `from_probability`; use
    /// [`PriorClause::with_provenance`] to attach a citation.
    pub provenance: Provenance,
}

impl PriorClause {
    /// Construct a prior with an explicit log-odds value, no
    /// provenance. The `id` is a sentinel `PriorClauseId(u64::MAX)`
    /// that [`KnowledgeBase::add_prior`] overwrites on insert. This
    /// mirrors the `Fact::certain` / `Rule::certain` pattern in
    /// [`crate::lib`] so that all clause types behave identically at
    /// construction time.
    pub fn new(conclusion: Term, prior_logit: f64) -> Self {
        Self {
            id: PriorClauseId(u64::MAX),
            conclusion,
            prior_logit,
            provenance: Provenance::unattributed(),
        }
    }

    /// Construct a prior from a probability `p ∈ (0, 1)`. Panics on
    /// `p ≤ 0.0` or `p ≥ 1.0` because the resulting logit would be
    /// infinite — that's a modeller error worth catching at
    /// construction, not silently producing an `inf`.
    pub fn from_probability(conclusion: Term, p: f64) -> Self {
        assert!(
            p > 0.0 && p < 1.0,
            "PriorClause::from_probability requires p ∈ (0.0, 1.0); got {p}"
        );
        Self::new(conclusion, (p / (1.0 - p)).ln())
    }

    /// Builder-style: attach a citation. Returns a new value with
    /// `provenance` set, leaving the original unchanged.
    ///
    /// Idiomatic usage:
    /// ```ignore
    /// kb.add_prior(
    ///     PriorClause::from_probability(atom("acs"), 0.10)
    ///         .with_provenance(Provenance::cited("Pope JH et al., NEJM 1995;342(16):1163-70"))
    /// )?;
    /// ```
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }
}

/// A single-source likelihood-ratio contribution.
///
/// "When `evidence_term` is observed, multiply the conclusion's odds
/// by `exp(logit_delta)`." Multiple contributions per
/// `(conclusion, evidence_term)` pair sum in log-odds — that is the
/// LP19e semantics for combining LR sources (e.g., one LR per
/// reviewing physician for the same finding, or one LR per cited
/// study).
#[derive(Debug, Clone, PartialEq)]
pub struct ContributionClause {
    pub id: ContributionClauseId,
    pub conclusion: Term,
    pub evidence_term: Term,
    pub logit_delta: f64,
    /// LP19e + ADJ47-B: citation for this contribution.
    pub provenance: Provenance,
}

impl ContributionClause {
    /// Construct a contribution with an explicit log(LR) value and
    /// unattributed provenance.
    pub fn new(conclusion: Term, evidence_term: Term, logit_delta: f64) -> Self {
        Self {
            id: ContributionClauseId(u64::MAX),
            conclusion,
            evidence_term,
            logit_delta,
            provenance: Provenance::unattributed(),
        }
    }

    /// Construct a contribution from an LR magnitude. Panics on
    /// `lr ≤ 0.0` — LR is by definition strictly positive (it's a
    /// ratio of probabilities).
    pub fn from_lr(conclusion: Term, evidence_term: Term, lr: f64) -> Self {
        assert!(
            lr > 0.0,
            "ContributionClause::from_lr requires lr > 0.0; got {lr}"
        );
        Self::new(conclusion, evidence_term, lr.ln())
    }

    /// Builder-style: attach a citation.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }
}

/// A joint-evidence interaction term.
///
/// Active iff *every* term in `evidence_set` is observed in the
/// current KB. The semantics is "synergy / explaining-away beyond
/// the product of atomic LRs" — `joint_logit_delta > 0` means the
/// combination is more diagnostic than independent atomic LRs would
/// predict; `< 0` means the combination is less diagnostic (the
/// evidence atoms partly explain each other away).
#[derive(Debug, Clone, PartialEq)]
pub struct JointContributionClause {
    pub id: JointContributionClauseId,
    pub conclusion: Term,
    pub evidence_set: Vec<Term>,
    pub joint_logit_delta: f64,
    /// LP19e + ADJ47-B: citation for this joint contribution.
    pub provenance: Provenance,
}

impl JointContributionClause {
    /// Construct a joint contribution with explicit log(joint LR)
    /// and unattributed provenance.
    pub fn new(conclusion: Term, evidence_set: Vec<Term>, joint_logit_delta: f64) -> Self {
        Self {
            id: JointContributionClauseId(u64::MAX),
            conclusion,
            evidence_set,
            joint_logit_delta,
            provenance: Provenance::unattributed(),
        }
    }

    /// Construct a joint contribution from a joint LR magnitude.
    /// Panics on `joint_lr ≤ 0.0`.
    pub fn from_lr(conclusion: Term, evidence_set: Vec<Term>, joint_lr: f64) -> Self {
        assert!(
            joint_lr > 0.0,
            "JointContributionClause::from_lr requires joint_lr > 0.0; got {joint_lr}"
        );
        Self::new(conclusion, evidence_set, joint_lr.ln())
    }

    /// Builder-style: attach a citation.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }
}

// ---------------------------------------------------------------------------
// KB-construction error type
// ---------------------------------------------------------------------------

/// Errors that can arise when adding clauses to a KB.
#[derive(Debug, Clone, PartialEq)]
pub enum KbError {
    /// A prior for the given conclusion already exists. Per LP19e,
    /// at most one prior per conclusion is permitted — this is how
    /// the modeller declares the Bayesian baseline rather than
    /// silently averaging or last-writer-wins.
    ConflictingPriors {
        conclusion: Term,
        existing: PriorClauseId,
    },
}

// ---------------------------------------------------------------------------
// The aggregation algorithm
// ---------------------------------------------------------------------------

/// A numerically stable sigmoid. The textbook `1 / (1 + exp(-x))`
/// underflows to NaN for very negative x because `exp(-x)` overflows
/// to infinity. The branch on the sign of x keeps both halves of the
/// curve within representable floats.
pub fn sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        let z = (-x).exp();
        1.0 / (1.0 + z)
    } else {
        let z = x.exp();
        z / (1.0 + z)
    }
}

/// Inverse of [`sigmoid`]: convert a probability `p ∈ (0, 1)` to its
/// log-odds. Useful for callers that want to interpret a posterior
/// in either representation. Returns `±∞` at the extremes.
pub fn logit(p: f64) -> f64 {
    (p / (1.0 - p)).ln()
}

/// The result of an LR aggregation. Mirrors the public
/// `SearchResult::LRAggregateResult` variant; this type is what the
/// `lr_aggregate` function itself returns, and what `lib::search`
/// wraps before handing to the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct LRAggregateResult {
    pub dag: ProofDAG,
    /// `sigmoid(posterior_logit)`. Materialised here too so the
    /// caller doesn't have to invoke the sigmoid themselves.
    pub posterior: f64,
    /// The final running log-odds after applying the prior and every
    /// active contribution.
    pub posterior_logit: f64,
    /// Diagnostics from the aggregation. Empty in the common case.
    /// Non-empty when the algorithm took a defensible-but-warnable
    /// branch — e.g. "no prior declared," "no contributions active,"
    /// "a contribution with LR=1.0 was silently a no-op."
    pub warnings: Vec<LrAggregateWarning>,
}

/// Conditions worth surfacing to the audit trail without hard-failing
/// the query. These mirror the LP19e §"Edge cases" enumeration.
#[derive(Debug, Clone, PartialEq)]
pub enum LrAggregateWarning {
    /// No `PriorClause` was found for the queried conclusion. The
    /// algorithm proceeded with `prior_logit = 0` (P = 0.5). The
    /// audit trail should flag this loudly — the resulting posterior
    /// is not Bayesian-grounded; it's a uniform-prior default.
    NoPriorDeclared { conclusion: Term },
    /// No contribution clause was active for the queried conclusion.
    /// The posterior equals the prior. Not necessarily an error; it
    /// is the right answer when no evidence has been observed yet.
    NoContributionsActive { conclusion: Term },
    /// A contribution with `logit_delta == 0.0` (equivalently
    /// `LR == 1.0`) fired. Permitted but a no-op; emitted once per
    /// such clause to surface likely modeller intent errors.
    DegenerateContribution { clause_id: ContributionClauseId },
}

/// Run LR aggregation for `query` against `kb`.
///
/// **Algorithm** (LP19e §"The inference algorithm"):
///
/// 1. Look up the prior on `query`. If absent, proceed with
///    `prior_logit = 0` and emit [`LrAggregateWarning::NoPriorDeclared`].
/// 2. For every single-source contribution naming `query` as its
///    conclusion, check whether the evidence term is observed
///    (`kb.observed_evidence`). If so, add `logit_delta` to the
///    running log-odds and record a `FromContribution` step.
/// 3. For every joint contribution naming `query`, check whether
///    *every* term in `evidence_set` is observed. If so, add
///    `joint_logit_delta` and record a `FromJointContribution` step.
/// 4. Convert the final log-odds to a probability via [`sigmoid`].
///
/// **Complexity**: `O(C + J·E)` where C is the number of single
/// contributions naming `query`, J is the number of joint
/// contributions, and E is the maximum joint evidence-set size.
/// Linear in clause count — no 2ⁿ-world enumeration is needed
/// because conditional independence makes the math collapse to a
/// sum.
pub fn lr_aggregate(query: &Term, kb: &KnowledgeBase) -> LRAggregateResult {
    let mut warnings: Vec<LrAggregateWarning> = Vec::new();
    let mut steps: Vec<ProofStep> = Vec::new();
    let mut via_facts: Vec<FactId> = Vec::new();

    // Step 1: prior (or warned absence).
    let prior_logit = match kb.prior_for(query) {
        Some(prior) => {
            steps.push(ProofStep {
                goal: query.clone(),
                origin: DerivationOrigin::FromPrior {
                    clause_id: prior.id,
                    prior_logit: prior.prior_logit,
                },
            });
            prior.prior_logit
        }
        None => {
            warnings.push(LrAggregateWarning::NoPriorDeclared {
                conclusion: query.clone(),
            });
            0.0
        }
    };
    let mut running_logit = prior_logit;

    // Step 2: single-source contributions.
    let mut any_contribution_active = false;
    let contributions = kb.contributions_for(query);
    for contrib in &contributions {
        if let Some(observed_facts) = kb.observed_evidence(&contrib.evidence_term) {
            any_contribution_active = true;
            if contrib.logit_delta == 0.0 {
                warnings.push(LrAggregateWarning::DegenerateContribution {
                    clause_id: contrib.id,
                });
            }
            steps.push(ProofStep {
                goal: query.clone(),
                origin: DerivationOrigin::FromContribution {
                    clause_id: contrib.id,
                    evidence_fact_ids: observed_facts.clone(),
                    logit_delta: contrib.logit_delta,
                },
            });
            running_logit += contrib.logit_delta;
            via_facts.extend(observed_facts);
        }
    }

    // Step 3: joint contributions.
    let joints = kb.joint_contributions_for(query);
    for joint in &joints {
        let mut every_evidence_observed = true;
        let mut joint_evidence_facts: Vec<FactId> = Vec::new();
        for ev_term in &joint.evidence_set {
            match kb.observed_evidence(ev_term) {
                Some(ids) => joint_evidence_facts.extend(ids),
                None => {
                    every_evidence_observed = false;
                    break;
                }
            }
        }
        if every_evidence_observed {
            any_contribution_active = true;
            steps.push(ProofStep {
                goal: query.clone(),
                origin: DerivationOrigin::FromJointContribution {
                    clause_id: joint.id,
                    evidence_fact_ids: joint_evidence_facts.clone(),
                    joint_logit_delta: joint.joint_logit_delta,
                },
            });
            running_logit += joint.joint_logit_delta;
            via_facts.extend(joint_evidence_facts);
        }
    }

    if !any_contribution_active {
        warnings.push(LrAggregateWarning::NoContributionsActive {
            conclusion: query.clone(),
        });
    }

    // Step 4: package the proof.
    via_facts.sort();
    via_facts.dedup();
    let posterior = sigmoid(running_logit);
    LRAggregateResult {
        dag: ProofDAG {
            root_query: query.clone(),
            proofs: vec![Proof {
                bindings: Substitution::empty(),
                steps,
                via_facts,
                via_rules: Vec::new(),
                posterior_logit: Some(running_logit),
                posterior_probability: Some(posterior),
            }],
        },
        posterior,
        posterior_logit: running_logit,
        warnings,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fact, KnowledgeBase};
    use logic_core::{atom, compound};

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn sigmoid_is_stable_at_large_magnitudes() {
        // At ±50, the textbook 1/(1+exp(-x)) underflows. Our branch
        // keeps both ends finite and within representable floats.
        assert!(sigmoid(50.0) > 0.9999999);
        assert!(sigmoid(-50.0) < 0.0000001);
        assert!(sigmoid(50.0).is_finite());
        assert!(sigmoid(-50.0).is_finite());
    }

    #[test]
    fn from_probability_round_trips_through_logit() {
        for p in &[0.01, 0.1, 0.25, 0.5, 0.75, 0.9, 0.99] {
            let lo = logit(*p);
            assert!(approx_eq(sigmoid(lo), *p, 1e-12), "p={p}");
        }
    }

    #[test]
    #[should_panic(expected = "lr > 0.0")]
    fn contribution_with_negative_lr_panics_at_construction() {
        let _ = ContributionClause::from_lr(atom("acs"), atom("sym"), -1.0);
    }

    #[test]
    #[should_panic(expected = "p ∈ (0.0, 1.0)")]
    fn prior_at_zero_panics() {
        let _ = PriorClause::from_probability(atom("acs"), 0.0);
    }

    #[test]
    fn no_prior_falls_through_with_warning_and_uniform_default() {
        let kb = KnowledgeBase::new();
        let result = lr_aggregate(&atom("acs"), &kb);
        assert!(approx_eq(result.posterior, 0.5, 1e-12));
        assert_eq!(result.posterior_logit, 0.0);
        assert_eq!(result.warnings.len(), 2);
        assert!(matches!(
            result.warnings[0],
            LrAggregateWarning::NoPriorDeclared { .. }
        ));
        assert!(matches!(
            result.warnings[1],
            LrAggregateWarning::NoContributionsActive { .. }
        ));
    }

    #[test]
    fn prior_only_returns_prior() {
        // 10% prior, no observations: posterior == prior.
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
            .unwrap();
        let result = lr_aggregate(&atom("acs"), &kb);
        assert!(approx_eq(result.posterior, 0.10, 1e-12));
        // One warning: no contributions active.
        assert_eq!(result.warnings.len(), 1);
        assert!(matches!(
            result.warnings[0],
            LrAggregateWarning::NoContributionsActive { .. }
        ));
        // Proof has one step: the prior.
        assert_eq!(result.dag.proofs.len(), 1);
        assert_eq!(result.dag.proofs[0].steps.len(), 1);
    }

    #[test]
    fn single_contribution_shifts_posterior_correctly() {
        // 10% prior + LR 2.5 on observed symptom → posterior 22%
        // (sanity: logit(0.10) + ln(2.5) ≈ -1.094, sigmoid → 0.217)
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
            .unwrap();
        kb.add_contribution(ContributionClause::from_lr(
            atom("acs"),
            compound("symptom", vec![atom("pressure")]),
            2.5,
        ));
        kb.add_fact(Fact::certain(compound("symptom", vec![atom("pressure")])));
        let result = lr_aggregate(&atom("acs"), &kb);
        assert!(
            approx_eq(result.posterior, 0.2174, 1e-3),
            "got {}",
            result.posterior
        );
        assert!(result.warnings.is_empty());
        // Proof: prior step + one contribution step.
        assert_eq!(result.dag.proofs[0].steps.len(), 2);
    }

    #[test]
    fn unobserved_evidence_does_not_contribute() {
        // 10% prior + LR 2.5 on UNOBSERVED symptom → posterior 10%.
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
            .unwrap();
        kb.add_contribution(ContributionClause::from_lr(
            atom("acs"),
            compound("symptom", vec![atom("pressure")]),
            2.5,
        ));
        // Note: no fact for symptom(pressure). The contribution is
        // skipped.
        let result = lr_aggregate(&atom("acs"), &kb);
        assert!(approx_eq(result.posterior, 0.10, 1e-12));
        // The unobserved contribution did not fire, so we warn.
        assert!(matches!(
            result.warnings[0],
            LrAggregateWarning::NoContributionsActive { .. }
        ));
    }

    #[test]
    fn joint_contribution_fires_only_when_all_evidence_observed() {
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
            .unwrap();
        // A synergy term: when BOTH pressure AND diaphoresis are
        // observed, add log(1.3) to the log-odds.
        kb.add_joint_contribution(JointContributionClause::from_lr(
            atom("acs"),
            vec![
                compound("symptom", vec![atom("pressure")]),
                compound("symptom", vec![atom("diaphoresis")]),
            ],
            1.3,
        ));
        // Observe only pressure: joint does not fire.
        kb.add_fact(Fact::certain(compound("symptom", vec![atom("pressure")])));
        let result = lr_aggregate(&atom("acs"), &kb);
        assert!(approx_eq(result.posterior, 0.10, 1e-12));
        // Now observe diaphoresis too: joint fires, posterior shifts.
        kb.add_fact(Fact::certain(compound(
            "symptom",
            vec![atom("diaphoresis")],
        )));
        let result = lr_aggregate(&atom("acs"), &kb);
        // logit(0.10) + ln(1.3) ≈ -1.935; sigmoid → 0.126
        assert!(
            approx_eq(result.posterior, 0.126, 1e-3),
            "got {}",
            result.posterior
        );
    }

    #[test]
    fn conflicting_priors_are_rejected() {
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
            .unwrap();
        let err = kb
            .add_prior(PriorClause::from_probability(atom("acs"), 0.20))
            .unwrap_err();
        assert!(matches!(err, KbError::ConflictingPriors { .. }));
    }

    #[test]
    fn degenerate_contribution_lr_one_warns() {
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
            .unwrap();
        kb.add_contribution(ContributionClause::from_lr(
            atom("acs"),
            atom("ev"),
            1.0,
        ));
        kb.add_fact(Fact::certain(atom("ev")));
        let result = lr_aggregate(&atom("acs"), &kb);
        // Posterior == prior (LR=1.0 is a no-op multiplier on odds).
        assert!(approx_eq(result.posterior, 0.10, 1e-12));
        assert!(result.warnings.iter().any(|w| matches!(
            w,
            LrAggregateWarning::DegenerateContribution { .. }
        )));
    }
}
