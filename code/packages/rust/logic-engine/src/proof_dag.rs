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

use crate::{FactId, RuleId};

/// Why a particular step succeeded.
#[derive(Debug, Clone, PartialEq)]
pub enum DerivationOrigin {
    /// The step was satisfied by a Fact.
    FromFact(FactId),
    /// The step was satisfied by a Rule (head unification + body proof).
    FromRule(RuleId),
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
        match s.origin {
            DerivationOrigin::FromFact(f) => facts.push(f),
            DerivationOrigin::FromRule(r) => rules.push(r),
        }
    }
    facts.sort();
    facts.dedup();
    rules.sort();
    rules.dedup();
    (facts, rules)
}
