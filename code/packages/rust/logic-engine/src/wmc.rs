//! Weighted Model Counting over a `ProofDAG`.
//!
//! Given a proof DAG produced by [`enumerate_all`], this module computes
//! `P(query)` by summing the probability mass of possible worlds in
//! which the query is provable.
//!
//! ## Semantics (LP19 §"Weighted Model Counting")
//!
//! A possible world is an assignment of Boolean truth values to every
//! probabilistic clause (Fact or Rule) in the knowledge base. The
//! probability of a world is the product of:
//!
//! - `p` for each prob clause assigned TRUE, where `p` is its probability,
//! - `1 - p` for each prob clause assigned FALSE.
//!
//! Probabilistic clauses are treated as independent random variables
//! unless higher-level modeling (Bayesian-network structure encoded as
//! additional rules) says otherwise.
//!
//! `Certain` clauses are *always* TRUE; they do not contribute degrees
//! of freedom to the world enumeration.
//!
//! The query is provable in a world iff at least one proof in the DAG
//! has all of its `via_facts` and `via_rules` TRUE in that world.
//!
//! `P(query) = Σ P(world) for worlds where query is provable`.
//!
//! ## This implementation
//!
//! Naïve enumeration over `2^n` worlds, where `n` is the count of
//! distinct probabilistic clauses used across all proofs. For `n ≤ 20`
//! this is fine (~1M worlds, sub-second). For larger `n`, the LP19a
//! sub-spec adds d-DNNF compilation, which evaluates in time linear in
//! the diagram's size rather than exponential in `n`. d-DNNF is future
//! work; the naïve path is correct and is what the first paper's
//! evaluation runs on.

use crate::proof_dag::{Proof, ProofDAG};
use crate::{FactId, KnowledgeBase, Probability, RuleId};

/// One indicator: either a Fact or a Rule, with its Bernoulli parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Indicator {
    Fact(FactId, f64),
    Rule(RuleId, f64),
}

impl Indicator {
    fn probability(&self) -> f64 {
        match self {
            Indicator::Fact(_, p) | Indicator::Rule(_, p) => *p,
        }
    }
}

/// `P(query)` for the given DAG against the KB it was built from.
///
/// Returns `0.0` if the DAG has no proofs. Returns `1.0` if every proof
/// uses only `Certain` clauses (the all-certain short-circuit:
/// equivalent to find-first succeeding deterministically).
pub fn weighted_model_count(dag: &ProofDAG, kb: &KnowledgeBase) -> f64 {
    if !dag.has_proof() {
        return 0.0;
    }

    // Collect the indicators — only probabilistic (non-Certain) clauses
    // contribute degrees of freedom.
    let mut indicators: Vec<Indicator> = Vec::new();
    for fid in dag.all_fact_ids() {
        if let Some(fact) = kb.find_fact_by_id(fid) {
            if let Probability::Value(p) = fact.probability {
                indicators.push(Indicator::Fact(fid, p));
            }
        }
    }
    for rid in dag.all_rule_ids() {
        if let Some(rule) = kb.find_rule_by_id(rid) {
            if let Probability::Value(p) = rule.probability {
                indicators.push(Indicator::Rule(rid, p));
            }
        }
    }

    if indicators.is_empty() {
        // Every clause used is Certain — query is provable with probability 1.
        return 1.0;
    }

    let n = indicators.len();
    let mut total: f64 = 0.0;

    // Enumerate 2^n possible worlds.
    for world_bits in 0u64..(1u64 << n) {
        // Compute this world's probability and its set of TRUE indicators.
        let mut world_prob: f64 = 1.0;
        let mut true_facts: Vec<FactId> = Vec::new();
        let mut true_rules: Vec<RuleId> = Vec::new();

        for (i, ind) in indicators.iter().enumerate() {
            let is_true = (world_bits >> i) & 1 == 1;
            let p = ind.probability();
            world_prob *= if is_true { p } else { 1.0 - p };
            if is_true {
                match ind {
                    Indicator::Fact(f, _) => true_facts.push(*f),
                    Indicator::Rule(r, _) => true_rules.push(*r),
                }
            }
        }

        // The query is provable in this world iff at least one proof
        // has all of its prob clauses TRUE in this world. Certain
        // clauses are always satisfied; we check them by consulting
        // the KB for each clause used in the proof.
        if dag
            .proofs
            .iter()
            .any(|p| proof_satisfied(p, &true_facts, &true_rules, kb))
        {
            total += world_prob;
        }
    }

    total
}

/// A proof is satisfied in a world iff every probabilistic clause it
/// uses is TRUE in the world. Certain clauses pass automatically (they
/// are not in the indicator set and their truth value is implicitly 1).
fn proof_satisfied(
    proof: &Proof,
    true_facts: &[FactId],
    true_rules: &[RuleId],
    kb: &KnowledgeBase,
) -> bool {
    proof.via_facts.iter().all(|f| match kb.find_fact_by_id(*f) {
        Some(fact) if fact.probability == Probability::Certain => true,
        Some(_) => true_facts.contains(f),
        None => false, // Unknown fact id is treated as not provable.
    }) && proof.via_rules.iter().all(|r| match kb.find_rule_by_id(*r) {
        Some(rule) if rule.probability == Probability::Certain => true,
        Some(_) => true_rules.contains(r),
        None => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enumerate::enumerate_all;
    use crate::{BodyLiteral, Fact, KnowledgeBase, Rule};
    use logic_core::{atom, compound, var, Term};

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn empty_dag_has_zero_probability() {
        let kb = KnowledgeBase::new();
        let dag = enumerate_all(&atom("nope"), &kb);
        assert_eq!(weighted_model_count(&dag, &kb), 0.0);
    }

    #[test]
    fn all_certain_query_has_probability_one() {
        let mut kb = KnowledgeBase::new();
        kb.add_fact(Fact::certain(atom("a")));
        let dag = enumerate_all(&atom("a"), &kb);
        assert_eq!(weighted_model_count(&dag, &kb), 1.0);
    }

    #[test]
    fn single_probabilistic_fact_returns_its_probability() {
        let mut kb = KnowledgeBase::new();
        kb.add_fact(Fact::with_probability(atom("a"), 0.7));
        let dag = enumerate_all(&atom("a"), &kb);
        assert!(approx_eq(weighted_model_count(&dag, &kb), 0.7));
    }

    #[test]
    fn two_independent_facts_disjunct_via_one_query_each() {
        // We cannot OR two facts in one query without a rule, so the
        // simpler test is to verify that a single probabilistic fact
        // gives its probability and that the engine doesn't double-count.
        let mut kb = KnowledgeBase::new();
        kb.add_fact(Fact::with_probability(compound("p", vec![atom("a")]), 0.4));
        // Add an unused fact to ensure the engine isn't confused by it.
        kb.add_fact(Fact::with_probability(compound("q", vec![atom("a")]), 0.9));
        let dag = enumerate_all(&compound("p", vec![atom("a")]), &kb);
        assert!(approx_eq(weighted_model_count(&dag, &kb), 0.4));
    }

    #[test]
    fn conjunction_via_rule_multiplies_independent_probabilities() {
        // 0.6 :: p.
        // 0.8 :: q.
        // r :- p, q.
        // ?- r.   P(r) = 0.6 * 0.8 = 0.48
        let mut kb = KnowledgeBase::new();
        kb.add_fact(Fact::with_probability(atom("p"), 0.6));
        kb.add_fact(Fact::with_probability(atom("q"), 0.8));
        kb.add_rule(Rule::certain(
            atom("r"),
            vec![BodyLiteral::Pos(atom("p")), BodyLiteral::Pos(atom("q"))],
        ));

        let dag = enumerate_all(&atom("r"), &kb);
        assert!(approx_eq(weighted_model_count(&dag, &kb), 0.6 * 0.8));
    }

    #[test]
    fn graph_reachability_with_independent_paths_uses_inclusion_exclusion() {
        // The canonical worked example from LP19:
        //   0.9 :: edge(a, b).
        //   0.8 :: edge(b, c).
        //   0.5 :: edge(a, c).
        //   path(X, Y) :- edge(X, Y).
        //   path(X, Y) :- edge(X, Z), path(Z, Y).
        //   ?- path(a, c).
        //
        // Two proofs: direct (edge(a,c)) and via b (edge(a,b), edge(b,c)).
        // They use disjoint probabilistic facts, so:
        //   P = 1 - (1 - 0.5) * (1 - 0.9 * 0.8)
        //     = 1 - 0.5 * 0.28
        //     = 0.86

        let mut kb = KnowledgeBase::new();
        kb.add_fact(Fact::with_probability(
            compound("edge", vec![atom("a"), atom("b")]),
            0.9,
        ));
        kb.add_fact(Fact::with_probability(
            compound("edge", vec![atom("b"), atom("c")]),
            0.8,
        ));
        kb.add_fact(Fact::with_probability(
            compound("edge", vec![atom("a"), atom("c")]),
            0.5,
        ));

        let x = var("X");
        let y = var("Y");
        kb.add_rule(Rule::certain(
            compound("path", vec![Term::Var(x.clone()), Term::Var(y.clone())]),
            vec![BodyLiteral::Pos(compound(
                "edge",
                vec![Term::Var(x.clone()), Term::Var(y.clone())],
            ))],
        ));

        let x = var("X");
        let y = var("Y");
        let z = var("Z");
        kb.add_rule(Rule::certain(
            compound("path", vec![Term::Var(x.clone()), Term::Var(y.clone())]),
            vec![
                BodyLiteral::Pos(compound(
                    "edge",
                    vec![Term::Var(x.clone()), Term::Var(z.clone())],
                )),
                BodyLiteral::Pos(compound(
                    "path",
                    vec![Term::Var(z.clone()), Term::Var(y.clone())],
                )),
            ],
        ));

        let dag = enumerate_all(&compound("path", vec![atom("a"), atom("c")]), &kb);
        assert_eq!(
            dag.proofs.len(),
            2,
            "should find both direct and via-b proofs"
        );
        let p = weighted_model_count(&dag, &kb);
        assert!(
            approx_eq(p, 0.86),
            "expected P(path(a,c)) = 0.86, got {}",
            p
        );
    }

    #[test]
    fn shared_fact_is_not_double_counted() {
        // 0.5 :: a.
        // p :- a.
        // q :- a.
        // r :- p.
        // r :- q.
        // ?- r.
        //
        // Two proofs of r exist (via p and via q), but both depend on
        // the same fact `a`. P(r) must be 0.5, NOT 0.5 + 0.5 - 0.25.
        // (i.e., NOT inclusion-exclusion over independent paths — the
        // paths are NOT independent.)

        let mut kb = KnowledgeBase::new();
        kb.add_fact(Fact::with_probability(atom("a"), 0.5));
        kb.add_rule(Rule::certain(atom("p"), vec![BodyLiteral::Pos(atom("a"))]));
        kb.add_rule(Rule::certain(atom("q"), vec![BodyLiteral::Pos(atom("a"))]));
        kb.add_rule(Rule::certain(atom("r"), vec![BodyLiteral::Pos(atom("p"))]));
        kb.add_rule(Rule::certain(atom("r"), vec![BodyLiteral::Pos(atom("q"))]));

        let dag = enumerate_all(&atom("r"), &kb);
        assert_eq!(dag.proofs.len(), 2);
        let p = weighted_model_count(&dag, &kb);
        assert!(
            approx_eq(p, 0.5),
            "shared-fact case: expected 0.5, got {} (would be 0.75 if inclusion-exclusion mistakenly applied to dependent paths)",
            p
        );
    }
}
