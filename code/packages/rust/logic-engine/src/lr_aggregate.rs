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
//! evidence term (via a Certain Fact or an SLD proof, see
//! [`KnowledgeBase::observed_evidence`]), the contribution's `log(LR)`
//! is added to the running log-odds. Rule-derived evidence attenuates
//! that delta by the confidence of the proof that established it.
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

use crate::compute::{compute, ComputeExpr, ExactRational};
use crate::proof_dag::{DerivationOrigin, Proof, ProofDAG, ProofStep};
use crate::{
    ContributionClauseId, FactId, JointContributionClauseId, KnowledgeBase,
    PredicateContributionClauseId, PriorClauseId, Provenance, UncertaintyMarkerId,
};

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
/// by `exp(logit_delta)`." When the evidence is derived rather than directly
/// observed, the applied delta is attenuated by the proof confidence. Multiple
/// contributions per
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

/// A numeric comparison operator for a predicate-gated contribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Ge,
    Le,
    Gt,
    Lt,
    Eq,
}

impl CmpOp {
    /// Evaluate `lhs <op> rhs` over f64. `Eq` uses an absolute tolerance
    /// so a fact extracted as `14600` matches a threshold `14600.0`.
    pub fn eval(&self, lhs: f64, rhs: f64) -> bool {
        match self {
            CmpOp::Ge => lhs >= rhs,
            CmpOp::Le => lhs <= rhs,
            CmpOp::Gt => lhs > rhs,
            CmpOp::Lt => lhs < rhs,
            CmpOp::Eq => (lhs - rhs).abs() < 1e-9,
        }
    }

    /// Evaluate with exact rational sidecars when both sides have them.
    /// This keeps legacy f64 behaviour for ordinary thresholds while making
    /// fraction equality such as `1 / 10 + 2 / 10 == 3 / 10` exact.
    pub fn eval_values(
        &self,
        lhs: f64,
        rhs: f64,
        lhs_exact: Option<ExactRational>,
        rhs_exact: Option<ExactRational>,
    ) -> bool {
        match (lhs_exact, rhs_exact) {
            (Some(a), Some(b)) => match self {
                CmpOp::Ge => cmp_exact(&a, &b) >= 0,
                CmpOp::Le => cmp_exact(&a, &b) <= 0,
                CmpOp::Gt => cmp_exact(&a, &b) > 0,
                CmpOp::Lt => cmp_exact(&a, &b) < 0,
                CmpOp::Eq => a == b,
            },
            _ => self.eval(lhs, rhs),
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            CmpOp::Ge => ">=",
            CmpOp::Le => "<=",
            CmpOp::Gt => ">",
            CmpOp::Lt => "<",
            CmpOp::Eq => "==",
        }
    }
}

/// Exact ordering of two rationals. `BigRational` is unbounded and totally ordered, so this is
/// an exact comparison with no cross-multiplication overflow to guard against (the old `i128`
/// sidecar needed an `f64` fallback; this never does).
fn cmp_exact(lhs: &ExactRational, rhs: &ExactRational) -> i8 {
    ordering_i8(lhs.as_ratio().cmp(rhs.as_ratio()))
}

fn ordering_i8(ordering: std::cmp::Ordering) -> i8 {
    match ordering {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// A **predicate-gated** likelihood-ratio contribution — the bridge that lets
/// adj-lang express a DETERMINISTIC rule as the saturating limit of a
/// probabilistic one. "When the observed numeric value of `slot` satisfies
/// `slot <op> value`, multiply the conclusion's odds by `exp(logit_delta)`."
/// A deterministic rule is simply a very large `logit_delta` (a saturating LR);
/// DETERMINATE / INDETERMINATE / CONFLICT then fall out of the differential
/// (leader / insufficient-evidence / kickback) — no separate engine.
///
/// The predicate is evaluated on CPU against the observed valued fact
/// `slot(V)` (V a `Term::Num`), so the model never does the comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct PredicateContributionClause {
    pub id: PredicateContributionClauseId,
    pub conclusion: Term,
    pub slot: String,
    pub op: CmpOp,
    pub rhs: ComputeExpr,
    pub logit_delta: f64,
    pub provenance: Provenance,
}

impl PredicateContributionClause {
    pub fn new(
        conclusion: Term,
        slot: impl Into<String>,
        op: CmpOp,
        value: f64,
        logit_delta: f64,
    ) -> Self {
        Self {
            id: PredicateContributionClauseId(u64::MAX),
            conclusion,
            slot: slot.into(),
            op,
            rhs: ComputeExpr::Lit(value),
            logit_delta,
            provenance: Provenance::unattributed(),
        }
    }

    pub fn new_expr(
        conclusion: Term,
        slot: impl Into<String>,
        op: CmpOp,
        rhs: ComputeExpr,
        logit_delta: f64,
    ) -> Self {
        Self {
            id: PredicateContributionClauseId(u64::MAX),
            conclusion,
            slot: slot.into(),
            op,
            rhs,
            logit_delta,
            provenance: Provenance::unattributed(),
        }
    }

    /// Construct from an LR magnitude (panics on `lr <= 0`).
    pub fn from_lr(
        conclusion: Term,
        slot: impl Into<String>,
        op: CmpOp,
        value: f64,
        lr: f64,
    ) -> Self {
        assert!(
            lr > 0.0,
            "PredicateContributionClause::from_lr requires lr > 0.0; got {lr}"
        );
        Self::new(conclusion, slot, op, value, lr.ln())
    }

    /// Construct a predicate contribution whose right-hand side is a full
    /// compute expression (`from answer == 3 / 10 to opt_a`).
    pub fn from_lr_expr(
        conclusion: Term,
        slot: impl Into<String>,
        op: CmpOp,
        rhs: ComputeExpr,
        lr: f64,
    ) -> Self {
        assert!(
            lr > 0.0,
            "PredicateContributionClause::from_lr_expr requires lr > 0.0; got {lr}"
        );
        Self::new_expr(conclusion, slot, op, rhs, lr.ln())
    }

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
// Uncertainty markers — ADJ47-D, addresses ADJ46 awkwardness A5
// ---------------------------------------------------------------------------

/// An explicit "we don't know which value of X applies here" annotation.
///
/// Dissolves ADJ46 awkwardness item **A5**. When a patient case
/// says "no clear precipitator," the IR pipeline lowers it to an
/// `UncertaintyMarker` whose `domain` is the candidate precipitator
/// terms. The aggregator detects markers whose domain is entirely
/// unobserved and emits an [`UncertaintyReport`] in the result, so
/// the framework's user-facing output can say "if you can determine
/// the precipitator, the posterior could shift by up to X."
///
/// This is the engine-layer enabler of VOI (ADJ18). The full VOI
/// computation lives in [`LRAggregateResult::uncertainties`], which
/// the user-facing layer can rank, render, or act on.
#[derive(Debug, Clone, PartialEq)]
pub struct UncertaintyMarker {
    pub id: UncertaintyMarkerId,
    pub conclusion: Term,
    /// Candidate evidence terms. Treated as mutually exclusive in
    /// v0.1: the aggregator computes the maximum log-odds swing
    /// across the domain, not the expected posterior under a prior
    /// over the domain. Richer treatment is a follow-up.
    pub domain: Vec<Term>,
    pub provenance: Provenance,
}

impl UncertaintyMarker {
    pub fn new(conclusion: Term, domain: Vec<Term>) -> Self {
        Self {
            id: UncertaintyMarkerId(u64::MAX),
            conclusion,
            domain,
            provenance: Provenance::unattributed(),
        }
    }

    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }
}

/// A user-facing report on an active uncertainty in the
/// LR-aggregation result.
///
/// Active means: the marker's domain has zero observed evidence
/// terms in the KB, so the aggregator skipped every related
/// contribution. The report tells the audit reader what each domain
/// value would have contributed if observed, plus the VOI summary
/// — the log-odds range that resolving the uncertainty could
/// produce.
#[derive(Debug, Clone, PartialEq)]
pub struct UncertaintyReport {
    pub marker_id: UncertaintyMarkerId,
    pub conclusion: Term,
    pub domain: Vec<Term>,
    /// One entry per domain term, in `domain` order. The f64 is the
    /// log(LR) that would be added to the running log-odds if that
    /// particular value were observed. `0.0` when no
    /// [`ContributionClause`] names the (conclusion, value) pair —
    /// i.e. the rulebook covers the uncertainty marker but not the
    /// individual value, which is a modeller signal worth surfacing.
    pub if_observed_logit_delta: Vec<f64>,
    /// `max(if_observed_logit_delta) - min(if_observed_logit_delta)`.
    /// The simple v0.1 VOI proxy: "knowing this value could swing
    /// the running log-odds by up to this much." Higher is more
    /// informative.
    pub voi_logit_range: f64,
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
    /// Active uncertainty reports. One entry per
    /// [`UncertaintyMarker`] on the query whose domain has zero
    /// observed evidence — exactly the markers worth showing the
    /// user as "if you can determine this, the answer would shift."
    pub uncertainties: Vec<UncertaintyReport>,
}

// ---------------------------------------------------------------------------
// Kickback — ADJ47-E, addresses ADJ46 awkwardness A7
// ---------------------------------------------------------------------------

/// Summary of why and how the framework would recommend the user
/// resolve some uncertainty before committing to the posterior.
///
/// Surfaces only when the *plausible posterior range* induced by the
/// current uncertainties straddles a decision threshold. Built so
/// the user-facing layer (an EMR widget, a brief-review tool, a
/// deal-room dashboard) can render "the system isn't confident; here
/// are the things to resolve" without having to recompute the band
/// itself.
#[derive(Debug, Clone, PartialEq)]
pub struct KickbackReport {
    pub posterior: f64,
    pub posterior_logit: f64,
    /// Worst-case posterior assuming every uncertainty resolves
    /// in the direction that *lowers* the conclusion's probability.
    pub posterior_lo: f64,
    /// Best-case posterior assuming every uncertainty resolves
    /// in the direction that *raises* the conclusion's probability.
    pub posterior_hi: f64,
    /// The decision threshold the band straddles.
    pub decision_threshold: f64,
    /// Recommend the user resolve these markers, ranked by
    /// individual VOI (largest range first).
    pub recommended_resolutions: Vec<UncertaintyMarkerId>,
}

// ---------------------------------------------------------------------------
// Source disagreement — ADJ47-E, addresses ADJ46 awkwardness A9
// ---------------------------------------------------------------------------

/// A flag that two or more `ContributionClause`s with the same
/// `(conclusion, evidence_term)` have substantially different
/// `logit_delta` values — i.e. the rulebook's sources disagree
/// about the LR for this piece of evidence.
///
/// Useful for the audit layer to surface "AHA 2021 says LR=2.5 for
/// this finding; ESC 2023 says LR=4.0 — your posterior is sensitive
/// to which authority you trust" without the user having to mine
/// the rulebook by hand.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceDisagreementReport {
    pub conclusion: Term,
    pub evidence_term: Term,
    /// Per-source records in clause-insertion order.
    pub source_logit_deltas: Vec<SourceLogitDelta>,
    /// `max(deltas) - min(deltas)`. Larger means the sources
    /// disagree more.
    pub disagreement_logit_range: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceLogitDelta {
    pub clause_id: ContributionClauseId,
    pub logit_delta: f64,
    pub provenance: Provenance,
}

impl LRAggregateResult {
    /// Suggest a kickback to the user if the posterior, accounting
    /// for active [`UncertaintyReport`]s, could end up on either side
    /// of `decision_threshold`.
    ///
    /// Returns `None` when:
    /// - no uncertainties are active (posterior is robust by
    ///   construction), or
    /// - the worst-case and best-case posterior both lie on the
    ///   *same* side of `decision_threshold` (the answer doesn't
    ///   depend on resolving any uncertainty).
    ///
    /// Returns `Some(KickbackReport)` when the band straddles the
    /// threshold — that's when the framework's recommended action
    /// is to escalate rather than commit.
    pub fn suggest_kickback(&self, decision_threshold: f64) -> Option<KickbackReport> {
        if self.uncertainties.is_empty() {
            return None;
        }
        // Sum of "if-observed" worst-case and best-case shifts the
        // current log-odds would receive from resolving every
        // uncertainty simultaneously. This is a *bounding* analysis
        // (each marker independently optimized) — it does not assume
        // the resolutions are jointly achievable, so the band is a
        // conservative outer envelope on the true plausible posterior.
        let mut lo_shift = 0.0;
        let mut hi_shift = 0.0;
        for u in &self.uncertainties {
            if u.if_observed_logit_delta.is_empty() {
                continue;
            }
            let mn = u
                .if_observed_logit_delta
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);
            let mx = u
                .if_observed_logit_delta
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            lo_shift += mn;
            hi_shift += mx;
        }
        let posterior_lo = sigmoid(self.posterior_logit + lo_shift);
        let posterior_hi = sigmoid(self.posterior_logit + hi_shift);
        if posterior_lo <= decision_threshold && decision_threshold <= posterior_hi {
            // Rank markers by their individual VOI (largest first).
            let mut ranked: Vec<&UncertaintyReport> = self.uncertainties.iter().collect();
            ranked.sort_by(|a, b| {
                b.voi_logit_range
                    .partial_cmp(&a.voi_logit_range)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            Some(KickbackReport {
                posterior: self.posterior,
                posterior_logit: self.posterior_logit,
                posterior_lo,
                posterior_hi,
                decision_threshold,
                recommended_resolutions: ranked.into_iter().map(|u| u.marker_id).collect(),
            })
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Counterfactual — ADJ47-E, addresses ADJ46 awkwardness A8
// ---------------------------------------------------------------------------

/// Rerun [`lr_aggregate`] on a copy of `kb` with `assumed_facts`
/// added as Certain Facts. Used to answer "what would the posterior
/// be if X were true?" without disturbing the caller's KB.
///
/// The implementation clones the KB, inserts each assumed fact, and
/// reruns aggregation. Linear in KB size + assumed_facts.len(); fine
/// at current scale, and the clone semantics is the right contract
/// — the caller's KB is unchanged.
pub fn counterfactual(
    query: &Term,
    kb: &KnowledgeBase,
    assumed_facts: &[Term],
) -> LRAggregateResult {
    let mut perturbed = kb.clone();
    for fact in assumed_facts {
        perturbed.add_fact(crate::Fact::certain(fact.clone()));
    }
    lr_aggregate(query, &perturbed)
}

/// Walk a KB's contributions for `conclusion`, group by
/// `evidence_term`, and report every group whose members have a
/// non-zero spread in `logit_delta`. Threshold parameter is the
/// minimum spread (in absolute log-odds) to surface; the default
/// in `kb_source_disagreements` is `1e-9` — i.e. any non-trivial
/// floating-point difference.
pub fn source_disagreements_with_threshold(
    kb: &KnowledgeBase,
    conclusion: &Term,
    min_spread: f64,
) -> Vec<SourceDisagreementReport> {
    let contributions = kb.contributions_for(conclusion);
    // Group by evidence_term (linear scan, small per-conclusion lists).
    let mut groups: Vec<(Term, Vec<SourceLogitDelta>)> = Vec::new();
    for c in &contributions {
        let entry = groups.iter_mut().find(|(t, _)| t == &c.evidence_term);
        let record = SourceLogitDelta {
            clause_id: c.id,
            logit_delta: c.logit_delta,
            provenance: c.provenance.clone(),
        };
        match entry {
            Some((_, list)) => list.push(record),
            None => groups.push((c.evidence_term.clone(), vec![record])),
        }
    }
    let mut out = Vec::new();
    for (evidence_term, deltas) in groups {
        if deltas.len() < 2 {
            continue;
        }
        let max = deltas
            .iter()
            .map(|d| d.logit_delta)
            .fold(f64::NEG_INFINITY, f64::max);
        let min = deltas
            .iter()
            .map(|d| d.logit_delta)
            .fold(f64::INFINITY, f64::min);
        let range = max - min;
        if range > min_spread {
            out.push(SourceDisagreementReport {
                conclusion: conclusion.clone(),
                evidence_term,
                source_logit_deltas: deltas,
                disagreement_logit_range: range,
            });
        }
    }
    out
}

/// Convenience wrapper over [`source_disagreements_with_threshold`]
/// with `min_spread = 1e-9`. Surfaces every group of two or more
/// `ContributionClause`s on the same `(conclusion, evidence)` whose
/// `logit_delta`s are not bit-for-bit equal.
pub fn source_disagreements(
    kb: &KnowledgeBase,
    conclusion: &Term,
) -> Vec<SourceDisagreementReport> {
    source_disagreements_with_threshold(kb, conclusion, 1e-9)
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
///    conclusion, check whether the evidence term is observed or provable
///    (`kb.observed_evidence`). If so, add the possibly attenuated
///    `logit_delta` to the running log-odds and record a
///    `FromContribution` step.
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
    let mut via_rules = Vec::new();

    // Step 1: prior (or warned absence).
    let prior_logit = match kb.prior_for(query) {
        Some(prior) => {
            steps.push(ProofStep {
                goal: query.clone(),
                origin: DerivationOrigin::FromPrior {
                    clause_id: prior.id,
                    prior_logit: prior.prior_logit,
                },
                depth: 0,
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
        if let Some(observed) = kb.observed_evidence(&contrib.evidence_term) {
            any_contribution_active = true;
            let logit_delta = contrib.logit_delta * observed.confidence;
            if logit_delta == 0.0 {
                warnings.push(LrAggregateWarning::DegenerateContribution {
                    clause_id: contrib.id,
                });
            }
            steps.push(ProofStep {
                goal: query.clone(),
                origin: DerivationOrigin::FromContribution {
                    clause_id: contrib.id,
                    evidence_fact_ids: observed.fact_ids.clone(),
                    evidence_proof: observed.proof.clone(),
                    logit_delta,
                },
                depth: 0,
            });
            running_logit += logit_delta;
            via_facts.extend(observed.fact_ids);
            via_rules.extend(observed.rule_ids);
        }
    }

    // Step 3: joint contributions.
    let joints = kb.joint_contributions_for(query);
    for joint in &joints {
        let mut every_evidence_observed = true;
        let mut joint_evidence_facts: Vec<FactId> = Vec::new();
        let mut joint_evidence_rules = Vec::new();
        let mut joint_evidence_proofs = Vec::new();
        let mut joint_confidence = 1.0;
        for ev_term in &joint.evidence_set {
            match kb.observed_evidence(ev_term) {
                Some(observed) => {
                    joint_confidence *= observed.confidence;
                    joint_evidence_facts.extend(observed.fact_ids);
                    joint_evidence_rules.extend(observed.rule_ids);
                    if let Some(proof) = observed.proof {
                        joint_evidence_proofs.push(*proof);
                    }
                }
                None => {
                    every_evidence_observed = false;
                    break;
                }
            }
        }
        if every_evidence_observed {
            any_contribution_active = true;
            let joint_logit_delta = joint.joint_logit_delta * joint_confidence;
            steps.push(ProofStep {
                goal: query.clone(),
                origin: DerivationOrigin::FromJointContribution {
                    clause_id: joint.id,
                    evidence_fact_ids: joint_evidence_facts.clone(),
                    evidence_proofs: joint_evidence_proofs,
                    joint_logit_delta,
                },
                depth: 0,
            });
            running_logit += joint_logit_delta;
            via_facts.extend(joint_evidence_facts);
            via_rules.extend(joint_evidence_rules);
        }
    }

    // Step 3b: predicate-gated contributions. For each, read the observed numeric
    // value of its slot and evaluate the predicate on CPU; if true, apply its
    // logit_delta. A saturating logit_delta makes this a deterministic rule.
    for pc in kb.predicate_contributions_for(query) {
        if let Some((observed, observed_exact)) = kb.observed_numeric(&pc.slot) {
            let Ok(rhs) = compute("__predicate_rhs", &pc.rhs, kb) else {
                continue;
            };
            if pc
                .op
                .eval_values(observed, rhs.value, observed_exact, rhs.exact)
            {
                any_contribution_active = true;
                steps.push(ProofStep {
                    goal: query.clone(),
                    origin: DerivationOrigin::FromPredicateContribution {
                        clause_id: pc.id,
                        slot: pc.slot.clone(),
                        op: pc.op,
                        threshold: rhs.value,
                        observed,
                        logit_delta: pc.logit_delta,
                    },
                    depth: 0,
                });
                running_logit += pc.logit_delta;
            }
        }
    }

    if !any_contribution_active {
        warnings.push(LrAggregateWarning::NoContributionsActive {
            conclusion: query.clone(),
        });
    }

    // Step 4: uncertainty reports — for every marker on `query`
    // whose domain has zero observed evidence, build a report
    // listing what each domain value would contribute.
    let mut uncertainties: Vec<UncertaintyReport> = Vec::new();
    for marker in kb.uncertainty_markers_for(query) {
        let any_observed = marker
            .domain
            .iter()
            .any(|ev| kb.observed_evidence(ev).is_some());
        if any_observed {
            // The marker is "resolved" — one of its domain values
            // was observed; the aggregator already counted that
            // contribution above, so no report.
            continue;
        }
        let deltas: Vec<f64> = marker
            .domain
            .iter()
            .map(|ev| {
                // Find the ContributionClause(s) for (query, ev) and
                // sum their logit_delta; 0.0 if no clause names this
                // pair.
                kb.contributions_for(query)
                    .iter()
                    .filter(|c| &c.evidence_term == ev)
                    .map(|c| c.logit_delta)
                    .sum::<f64>()
            })
            .collect();
        let voi = if deltas.is_empty() {
            0.0
        } else {
            let max = deltas.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let min = deltas.iter().copied().fold(f64::INFINITY, f64::min);
            max - min
        };
        uncertainties.push(UncertaintyReport {
            marker_id: marker.id,
            conclusion: marker.conclusion.clone(),
            domain: marker.domain.clone(),
            if_observed_logit_delta: deltas,
            voi_logit_range: voi,
        });
    }

    // Step 5: package the proof.
    via_facts.sort();
    via_facts.dedup();
    via_rules.sort();
    via_rules.dedup();
    let posterior = sigmoid(running_logit);
    LRAggregateResult {
        dag: ProofDAG {
            root_query: query.clone(),
            proofs: vec![Proof {
                bindings: Substitution::empty(),
                steps,
                via_facts,
                via_rules,
                posterior_logit: Some(running_logit),
                posterior_probability: Some(posterior),
            }],
            // LR aggregation walks a FIXED clause list rather than searching,
            // so it has no budget to exhaust and can never be truncated.
            truncated: false,
        },
        posterior,
        posterior_logit: running_logit,
        warnings,
        uncertainties,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComputeOp, Fact, KnowledgeBase};
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
        kb.add_contribution(ContributionClause::from_lr(atom("acs"), atom("ev"), 1.0));
        kb.add_fact(Fact::certain(atom("ev")));
        let result = lr_aggregate(&atom("acs"), &kb);
        // Posterior == prior (LR=1.0 is a no-op multiplier on odds).
        assert!(approx_eq(result.posterior, 0.10, 1e-12));
        assert!(result
            .warnings
            .iter()
            .any(|w| matches!(w, LrAggregateWarning::DegenerateContribution { .. })));
    }

    // ---- predicate-gated contributions (deterministic = saturating) ----

    use logic_core::int;

    #[test]
    fn cmpop_evaluates_each_operator() {
        assert!(CmpOp::Ge.eval(14600.0, 14600.0));
        assert!(CmpOp::Ge.eval(18000.0, 14600.0));
        assert!(!CmpOp::Ge.eval(14000.0, 14600.0));
        assert!(CmpOp::Le.eval(10.0, 20.0));
        assert!(CmpOp::Gt.eval(2.0, 1.0));
        assert!(!CmpOp::Gt.eval(1.0, 1.0));
        assert!(CmpOp::Lt.eval(1.0, 2.0));
        assert!(CmpOp::Eq.eval(5.0, 5.0));
        assert!(!CmpOp::Eq.eval(5.0, 5.1));
        assert_eq!(CmpOp::Ge.symbol(), ">=");
        assert_eq!(CmpOp::Eq.symbol(), "==");
    }

    #[test]
    #[should_panic(expected = "lr > 0.0")]
    fn predicate_contribution_with_negative_lr_panics() {
        let _ = PredicateContributionClause::from_lr(
            atom("required_to_file"),
            "gross_income",
            CmpOp::Ge,
            14600.0,
            -1.0,
        );
    }

    #[test]
    fn predicate_fires_when_threshold_met_and_saturates() {
        // Deterministic rule via a huge LR: "income >= 14600 ⇒ required to
        // file". Observe a valued slot above threshold; verdict saturates.
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(
            atom("required_to_file"),
            0.10,
        ))
        .unwrap();
        kb.add_predicate_contribution(PredicateContributionClause::from_lr(
            atom("required_to_file"),
            "gross_income",
            CmpOp::Ge,
            14600.0,
            1e6,
        ));
        kb.add_fact(Fact::certain(compound("gross_income", vec![int(18000)])));

        let result = lr_aggregate(&atom("required_to_file"), &kb);
        assert!(result.posterior > 0.9999, "got {}", result.posterior);

        // The proof carries the literal comparison that fired — the model
        // never computed it.
        let step = result.dag.proofs[0]
            .steps
            .iter()
            .find_map(|s| match &s.origin {
                DerivationOrigin::FromPredicateContribution {
                    slot,
                    op,
                    threshold,
                    observed,
                    ..
                } => Some((slot.clone(), op.symbol(), *threshold, *observed)),
                _ => None,
            })
            .expect("a predicate step should be present");
        assert_eq!(step, ("gross_income".to_string(), ">=", 14600.0, 18000.0));
    }

    #[test]
    fn predicate_expression_rhs_uses_exact_fraction_equality() {
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(atom("opt_a"), 0.10))
            .unwrap();
        let answer = crate::compute(
            "answer",
            &ComputeExpr::Bin(
                ComputeOp::Add,
                Box::new(ComputeExpr::Bin(
                    ComputeOp::Div,
                    Box::new(ComputeExpr::Lit(1.0)),
                    Box::new(ComputeExpr::Lit(10.0)),
                )),
                Box::new(ComputeExpr::Bin(
                    ComputeOp::Div,
                    Box::new(ComputeExpr::Lit(2.0)),
                    Box::new(ComputeExpr::Lit(10.0)),
                )),
            ),
            &kb,
        )
        .unwrap();
        assert_eq!(answer.exact, ExactRational::new(3, 10));
        kb.add_derived(answer);
        kb.add_predicate_contribution(PredicateContributionClause::from_lr_expr(
            atom("opt_a"),
            "answer",
            CmpOp::Eq,
            ComputeExpr::Bin(
                ComputeOp::Div,
                Box::new(ComputeExpr::Lit(3.0)),
                Box::new(ComputeExpr::Lit(10.0)),
            ),
            1e6,
        ));

        let result = lr_aggregate(&atom("opt_a"), &kb);
        assert!(result.posterior > 0.9999, "got {}", result.posterior);
        assert!(result.dag.proofs[0]
            .steps
            .iter()
            .any(|s| matches!(s.origin, DerivationOrigin::FromPredicateContribution { .. })));
    }

    #[test]
    fn predicate_does_not_fire_below_threshold() {
        // Income under threshold: the dispositive contribution never fires,
        // so the verdict stays at the prior (INDETERMINATE territory — the
        // evidence-sufficiency guard upstream turns this into abstention).
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(
            atom("required_to_file"),
            0.10,
        ))
        .unwrap();
        kb.add_predicate_contribution(PredicateContributionClause::from_lr(
            atom("required_to_file"),
            "gross_income",
            CmpOp::Ge,
            14600.0,
            1e6,
        ));
        kb.add_fact(Fact::certain(compound("gross_income", vec![int(9000)])));

        let result = lr_aggregate(&atom("required_to_file"), &kb);
        assert!(
            approx_eq(result.posterior, 0.10, 1e-9),
            "got {}",
            result.posterior
        );
        assert!(!result.dag.proofs[0]
            .steps
            .iter()
            .any(|s| matches!(s.origin, DerivationOrigin::FromPredicateContribution { .. })));
    }

    #[test]
    fn predicate_fires_over_typed_quantity_wrapper() {
        // Step 2: a typed value `quantity(18000, usd)` carries its unit;
        // the predicate compares against the leading magnitude (18000)
        // while `usd` stays attached to the fact for the audit gate.
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(
            atom("required_to_file"),
            0.10,
        ))
        .unwrap();
        kb.add_predicate_contribution(PredicateContributionClause::from_lr(
            atom("required_to_file"),
            "gross_income",
            CmpOp::Ge,
            14600.0,
            1e6,
        ));
        kb.add_fact(Fact::certain(compound(
            "gross_income",
            vec![compound("quantity", vec![int(18000), atom("usd")])],
        )));

        let result = lr_aggregate(&atom("required_to_file"), &kb);
        assert!(result.posterior > 0.9999, "got {}", result.posterior);
    }

    #[test]
    fn numeric_magnitude_reads_bare_and_wrapped_values() {
        use crate::numeric_magnitude;
        assert_eq!(numeric_magnitude(&int(42)), Some(42.0));
        assert_eq!(numeric_magnitude(&logic_core::float(3.5)), Some(3.5));
        assert_eq!(
            numeric_magnitude(&compound("quantity", vec![int(18000), atom("usd")])),
            Some(18000.0)
        );
        assert_eq!(
            numeric_magnitude(&compound("percentage", vec![int(40)])),
            Some(40.0)
        );
        // No leading number → no magnitude.
        assert_eq!(numeric_magnitude(&atom("usd")), None);
        assert_eq!(
            numeric_magnitude(&compound("pair", vec![atom("a"), int(1)])),
            None
        );
    }

    #[test]
    fn predicate_uses_latest_observation_of_slot() {
        // A later `observe` of the same slot supersedes the earlier value.
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(
            atom("required_to_file"),
            0.10,
        ))
        .unwrap();
        kb.add_predicate_contribution(PredicateContributionClause::from_lr(
            atom("required_to_file"),
            "gross_income",
            CmpOp::Ge,
            14600.0,
            1e6,
        ));
        kb.add_fact(Fact::certain(compound("gross_income", vec![int(9000)])));
        kb.add_fact(Fact::certain(compound("gross_income", vec![int(20000)])));

        let result = lr_aggregate(&atom("required_to_file"), &kb);
        assert!(
            result.posterior > 0.9999,
            "latest value should fire; got {}",
            result.posterior
        );
    }
}
