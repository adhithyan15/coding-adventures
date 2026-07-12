//! Proof DAG types — what the engine returns when running in
//! `EnumerateAll` mode (or when `AutoDetect` selects it because at
//! least one clause carries a non-`Certain` probability).
//!
//! The full LP19 grammar describes a true DAG where multiple proofs may
//! share subproofs through `lowered_from` edges. This module starts with
//! the simpler representation that captures every successful proof as
//! an independent path with its own sequence of derivation steps.
//! Shared-subproof DAG compression is a later optimization; for
//! correctness, the proofs-as-paths form is sufficient because the
//! weighted-model-counting backend operates over the *set of clauses
//! used per proof*, not over the topological structure.

use logic_core::{Substitution, Term};

use crate::lr_aggregate::CmpOp;
use crate::{
    ContributionClauseId, FactId, JointContributionClauseId, PredicateContributionClauseId,
    PriorClauseId, RuleId,
};

/// Why a particular step succeeded.
///
/// The first two variants — [`FromFact`](Self::FromFact) and
/// [`FromRule`](Self::FromRule) — describe SLD-resolution-style steps
/// produced by [`crate::enumerate_all`].
///
/// The remaining three — [`FromPrior`](Self::FromPrior),
/// [`FromContribution`](Self::FromContribution), and
/// [`FromJointContribution`](Self::FromJointContribution) — describe
/// likelihood-ratio aggregation steps produced by
/// [`crate::lr_aggregate`] (LP19e). They are *additive*: the
/// pre-existing two variants continue to mean exactly what they
/// always have. The LR variants carry the *running log-odds delta*
/// the step contributed, so an audit reader can reconstruct the
/// posterior arithmetic by walking the proof in evaluation order.
#[derive(Debug, Clone, PartialEq)]
pub enum DerivationOrigin {
    /// The step was satisfied by a Fact.
    FromFact(FactId),
    /// The step was satisfied by a Rule (head unification + body proof).
    FromRule(RuleId),
    /// The step is the "seed" of an LR aggregation — the prior on the
    /// conclusion. Per LP19e there is at most one such step per
    /// aggregation proof, and it appears first.
    FromPrior {
        clause_id: PriorClauseId,
        /// log(p / (1 - p)) for the prior probability p. Carried
        /// inline so the audit reader doesn't have to consult the KB
        /// to reconstruct running log-odds.
        prior_logit: f64,
    },
    /// The step applied a single-source LR contribution to the
    /// running log-odds.
    FromContribution {
        clause_id: ContributionClauseId,
        /// FactIds for every Fact that satisfied the evidence term.
        /// Directly observed facts appear here, and rule-derived
        /// evidence also repeats the proof's leaf facts so the
        /// aggregate proof can expose a flat fact index.
        evidence_fact_ids: Vec<FactId>,
        /// If the evidence term was not directly observed but was
        /// proved by SLD resolution, this is the derivation that
        /// licensed the contribution.
        evidence_proof: Option<Box<Proof>>,
        /// log(LR) for this contribution. Inline for the same audit
        /// reason as `prior_logit`.
        logit_delta: f64,
    },
    /// The step applied a joint-evidence interaction term — synergy
    /// (positive delta) or explaining-away (negative delta) beyond
    /// the product of atomic LRs.
    FromJointContribution {
        clause_id: JointContributionClauseId,
        /// Union of FactIds satisfying every evidence term in the
        /// joint set.
        evidence_fact_ids: Vec<FactId>,
        /// SLD derivations for any joint evidence terms that were
        /// rule-derived rather than directly observed.
        evidence_proofs: Vec<Proof>,
        /// log(joint LR). Inline.
        joint_logit_delta: f64,
    },
    /// The step applied a **predicate-gated** contribution: the observed
    /// numeric value of `slot` satisfied `slot <op> threshold`, so the
    /// clause's `logit_delta` was added to the running log-odds. This is
    /// how a DETERMINISTIC rule expresses itself — a saturating
    /// `logit_delta` over a CPU-evaluated numeric predicate. The audit
    /// reader sees the literal comparison that fired (`observed`,
    /// `op.symbol()`, `threshold`), never a model-computed number.
    FromPredicateContribution {
        clause_id: PredicateContributionClauseId,
        /// The valued slot whose observation was compared.
        slot: String,
        /// The comparison operator (`>=`, `<=`, `>`, `<`, `==`).
        op: CmpOp,
        /// The right-hand threshold the clause was written against.
        threshold: f64,
        /// The observed numeric value read from the valued fact `slot(V)`.
        observed: f64,
        /// log(LR) applied because the predicate held. Inline.
        logit_delta: f64,
    },
}

/// A single step inside a proof.
#[derive(Debug, Clone, PartialEq)]
pub struct ProofStep {
    /// The goal this step proved (after applying its parent's substitution).
    pub goal: Term,
    /// What clause this step was derived from.
    pub origin: DerivationOrigin,
}

/// One complete proof of the root query.
///
/// `bindings` is the substitution that, when applied to the root query,
/// produces the proved instance. `steps` records the derivation in
/// depth-first order — useful for audit-trail reconstruction.
///
/// `via_facts` and `via_rules` are de-duplicated lists of the FactIds
/// and RuleIds used anywhere in this proof. They are the propositional
/// variables this proof's Boolean conjunct depends on for weighted
/// model counting.
#[derive(Debug, Clone, PartialEq)]
pub struct Proof {
    pub bindings: Substitution,
    pub steps: Vec<ProofStep>,
    pub via_facts: Vec<FactId>,
    pub via_rules: Vec<RuleId>,
    /// The running log-odds *after* applying every LR step in
    /// `steps`. `Some(x)` after an LR aggregation; `None` after
    /// `FindFirst`, `EnumerateAll`, or `AutoDetect → WMC`.
    ///
    /// These two fields are additive per LP19e §"Proof DAG
    /// integration": they let an LR-aware reader recover the
    /// posterior arithmetic from a single Proof, but they do not
    /// disturb the SLD-resolution / WMC paths that never set them.
    pub posterior_logit: Option<f64>,
    /// `sigmoid(posterior_logit)` — the posterior probability of the
    /// root query. Redundant with `posterior_logit` but materialised
    /// so callers don't all reinvent the sigmoid.
    pub posterior_probability: Option<f64>,
}

/// Collection of all proofs of a query against a knowledge base.
///
/// For deterministic queries, `proofs.len()` is 1 (or 0 on failure)
/// when the engine is in `FindFirst` mode. For probabilistic queries
/// in `EnumerateAll` mode, `proofs.len()` is every successful
/// derivation — possibly several, the input to weighted model counting.
#[derive(Debug, Clone, PartialEq)]
pub struct ProofDAG {
    pub root_query: Term,
    pub proofs: Vec<Proof>,
}

impl ProofDAG {
    /// `true` iff at least one proof of the root query exists.
    pub fn has_proof(&self) -> bool {
        !self.proofs.is_empty()
    }

    /// The complete set of probabilistic facts referenced by any proof.
    pub fn all_fact_ids(&self) -> Vec<FactId> {
        let mut out: Vec<FactId> = self
            .proofs
            .iter()
            .flat_map(|p| p.via_facts.iter().copied())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The complete set of probabilistic rules referenced by any proof.
    pub fn all_rule_ids(&self) -> Vec<RuleId> {
        let mut out: Vec<RuleId> = self
            .proofs
            .iter()
            .flat_map(|p| p.via_rules.iter().copied())
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// Helper for the enumeration module: de-duplicate fact and rule ids
/// from a step list. Public within the crate so the enumeration code
/// can keep `Proof` construction trivial.
pub(crate) fn collect_ids(steps: &[ProofStep]) -> (Vec<FactId>, Vec<RuleId>) {
    let mut facts: Vec<FactId> = Vec::new();
    let mut rules: Vec<RuleId> = Vec::new();
    for s in steps {
        match &s.origin {
            DerivationOrigin::FromFact(f) => facts.push(*f),
            DerivationOrigin::FromRule(r) => rules.push(*r),
            // The LR-aggregation variants carry their direct evidence
            // Fact ids inline. If evidence was rule-derived, the nested
            // SLD proof carries the facts and rules that licensed it.
            DerivationOrigin::FromPrior { .. } => {}
            DerivationOrigin::FromContribution {
                evidence_fact_ids,
                evidence_proof,
                ..
            } => {
                facts.extend(evidence_fact_ids.iter().copied());
                if let Some(proof) = evidence_proof {
                    facts.extend(proof.via_facts.iter().copied());
                    rules.extend(proof.via_rules.iter().copied());
                }
            }
            DerivationOrigin::FromJointContribution {
                evidence_fact_ids,
                evidence_proofs,
                ..
            } => {
                facts.extend(evidence_fact_ids.iter().copied());
                for proof in evidence_proofs {
                    facts.extend(proof.via_facts.iter().copied());
                    rules.extend(proof.via_rules.iter().copied());
                }
            }
            // Predicate-gated contributions read a Certain valued slot
            // on CPU; the fired comparison (observed/op/threshold) is the
            // provenance carried inline on the step. They contribute no
            // probabilistic propositional variable to WMC, so they add no
            // FactId here.
            DerivationOrigin::FromPredicateContribution { .. } => {}
        }
    }
    facts.sort();
    facts.dedup();
    rules.sort();
    rules.dedup();
    (facts, rules)
}
