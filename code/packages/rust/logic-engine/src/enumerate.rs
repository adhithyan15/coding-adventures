//! Enumerate every successful proof of a query, building a `ProofDAG`.
//!
//! This is the search path the engine takes when at least one Fact or
//! Rule carries a non-`Certain` probability. It is also reachable
//! directly via [`SearchMode::EnumerateAll`] when the caller wants
//! complete proof enumeration regardless of probability content (for
//! example, when computing diagnostic explanations).
//!
//! ## Algorithm sketch
//!
//! At each goal:
//!
//! 1. Try every Fact whose head matches the goal's functor/arity. For
//!    each successful unification, emit a complete proof leaf with the
//!    resulting substitution.
//! 2. Try every Rule whose head matches. For each successful
//!    unification of the rule's head with the goal:
//!    - Rename the rule's variables so distinct clause instantiations
//!      don't share variable identity (same machinery as the
//!      deterministic search).
//!    - Recursively solve the rule's body literals, collecting every
//!      combination of successful body proofs.
//!
//! The result is a flat list of `(Substitution, Vec<ProofStep>)` pairs
//! — one per successful proof. Each is then packaged into a `Proof`
//! with deduplicated via_facts / via_rules.
//!
//! Negation-as-failure inside a rule body is the same as in
//! `find_first`: `Neg(t)` succeeds iff `t` cannot be proved. In the
//! enumeration setting this means we run `solve` on `t` and check
//! whether the result is empty. For the deterministic subset this is
//! straightforward; for the probabilistic case it follows LP19's
//! Sato distribution semantics specialized to stratified well-founded
//! programs (LP19 §"Negation in the Probabilistic Setting").

use std::collections::HashMap;

use logic_core::{LogicVar, Substitution, Term, unify};

use crate::proof_dag::{collect_ids, DerivationOrigin, Proof, ProofDAG, ProofStep};
use crate::{BodyLiteral, KnowledgeBase};

/// Walk `term` and replace every `Var` with a fresh variable, sharing
/// renames so that two occurrences of the same variable in the input
/// map to the same fresh variable in the output. Same helper the
/// deterministic search uses.
fn rename_term(term: &Term, renames: &mut HashMap<u64, LogicVar>) -> Term {
    match term {
        Term::Var(v) => {
            let fresh = renames
                .entry(v.id)
                .or_insert_with(|| LogicVar::fresh(v.display_name.as_deref()))
                .clone();
            Term::Var(fresh)
        }
        Term::Compound { functor, args } => Term::Compound {
            functor: functor.clone(),
            args: args.iter().map(|a| rename_term(a, renames)).collect(),
        },
        other => other.clone(),
    }
}

fn rename_literal(lit: &BodyLiteral, renames: &mut HashMap<u64, LogicVar>) -> BodyLiteral {
    match lit {
        BodyLiteral::Pos(t) => BodyLiteral::Pos(rename_term(t, renames)),
        BodyLiteral::Neg(t) => BodyLiteral::Neg(rename_term(t, renames)),
    }
}

/// Enumerate every successful proof of `query` against `kb`. Returns a
/// `ProofDAG` containing one `Proof` per successful derivation.
pub fn enumerate_all(query: &Term, kb: &KnowledgeBase) -> ProofDAG {
    let raw = solve(query, kb, &Substitution::empty());
    let proofs = raw
        .into_iter()
        .map(|(bindings, steps)| {
            let (via_facts, via_rules) = collect_ids(&steps);
            Proof {
                bindings,
                steps,
                via_facts,
                via_rules,
            }
        })
        .collect();
    ProofDAG {
        root_query: query.clone(),
        proofs,
    }
}

/// Solve a single goal, returning every successful (substitution,
/// derivation-steps) pair. The substitution applies to the *current*
/// goal in the calling context; the calling context is responsible for
/// composing further substitutions.
fn solve(
    goal: &Term,
    kb: &KnowledgeBase,
    subst: &Substitution,
) -> Vec<(Substitution, Vec<ProofStep>)> {
    let resolved = subst.walk(goal);
    let mut results: Vec<(Substitution, Vec<ProofStep>)> = Vec::new();

    // Try every matching Fact.
    for fact in kb.facts_for(&resolved) {
        let mut renames = HashMap::new();
        let renamed = rename_term(&fact.term, &mut renames);
        if let Some(s) = unify(&resolved, &renamed, subst) {
            let step = ProofStep {
                goal: resolved.clone(),
                origin: DerivationOrigin::FromFact(fact.id),
            };
            results.push((s, vec![step]));
        }
    }

    // Try every matching Rule.
    for rule in kb.rules_for(&resolved) {
        let mut renames = HashMap::new();
        let renamed_head = rename_term(&rule.head, &mut renames);
        let renamed_body: Vec<BodyLiteral> = rule
            .body
            .iter()
            .map(|lit| rename_literal(lit, &mut renames))
            .collect();

        if let Some(s) = unify(&resolved, &renamed_head, subst) {
            for (body_subst, body_steps) in solve_body(&renamed_body, kb, &s) {
                let mut steps = Vec::with_capacity(1 + body_steps.len());
                steps.push(ProofStep {
                    goal: resolved.clone(),
                    origin: DerivationOrigin::FromRule(rule.id),
                });
                steps.extend(body_steps);
                results.push((body_subst, steps));
            }
        }
    }

    results
}

/// Prove every literal in `body`, threading substitutions forward and
/// enumerating every combination of body-literal proofs. Returns the
/// full list of (final-substitution, accumulated-steps) pairs.
fn solve_body(
    body: &[BodyLiteral],
    kb: &KnowledgeBase,
    subst: &Substitution,
) -> Vec<(Substitution, Vec<ProofStep>)> {
    if body.is_empty() {
        return vec![(subst.clone(), Vec::new())];
    }

    let (first, rest) = body.split_first().unwrap();
    let mut results = Vec::new();

    match first {
        BodyLiteral::Pos(t) => {
            // Find every way to prove `t`; for each, recurse on `rest`.
            for (after_first, steps_first) in solve(t, kb, subst) {
                for (after_rest, steps_rest) in solve_body(rest, kb, &after_first) {
                    let mut all_steps = Vec::with_capacity(steps_first.len() + steps_rest.len());
                    all_steps.extend(steps_first.iter().cloned());
                    all_steps.extend(steps_rest);
                    results.push((after_rest, all_steps));
                }
            }
        }
        BodyLiteral::Neg(t) => {
            // Negation-as-failure: succeed iff `t` has zero proofs.
            // Substitution and steps are unchanged on success.
            if solve(t, kb, subst).is_empty() {
                results.extend(solve_body(rest, kb, subst));
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fact, KnowledgeBase, Rule};
    use logic_core::{atom, compound, var};

    #[test]
    fn enumerate_all_finds_every_fact_match() {
        // Three father/2 facts; ?- father(homer, X). should produce 3 proofs.
        let mut kb = KnowledgeBase::new();
        kb.add_fact(Fact::certain(compound(
            "father",
            vec![atom("homer"), atom("bart")],
        )));
        kb.add_fact(Fact::certain(compound(
            "father",
            vec![atom("homer"), atom("lisa")],
        )));
        kb.add_fact(Fact::certain(compound(
            "father",
            vec![atom("homer"), atom("maggie")],
        )));

        let x = var("X");
        let query = compound("father", vec![atom("homer"), Term::Var(x.clone())]);
        let dag = enumerate_all(&query, &kb);

        assert_eq!(dag.proofs.len(), 3, "should find three children of homer");

        // Check that the bindings cover all three children.
        let mut children: Vec<Term> = dag.proofs.iter().map(|p| p.bindings.walk_var(&x)).collect();
        children.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        assert_eq!(
            children,
            vec![atom("bart"), atom("lisa"), atom("maggie")]
        );
    }

    #[test]
    fn enumerate_all_returns_empty_on_failure() {
        let kb = KnowledgeBase::new();
        let dag = enumerate_all(&atom("nope"), &kb);
        assert!(dag.proofs.is_empty());
        assert!(!dag.has_proof());
    }

    #[test]
    fn enumerate_all_traverses_rules_and_facts() {
        // parent(X, Y) :- father(X, Y).
        // father(homer, bart).
        // father(homer, lisa).
        // ?- parent(homer, Who).  -> two proofs
        let mut kb = KnowledgeBase::new();
        kb.add_fact(Fact::certain(compound(
            "father",
            vec![atom("homer"), atom("bart")],
        )));
        kb.add_fact(Fact::certain(compound(
            "father",
            vec![atom("homer"), atom("lisa")],
        )));

        let x = var("X");
        let y = var("Y");
        kb.add_rule(Rule::certain(
            compound("parent", vec![Term::Var(x.clone()), Term::Var(y.clone())]),
            vec![BodyLiteral::Pos(compound(
                "father",
                vec![Term::Var(x.clone()), Term::Var(y.clone())],
            ))],
        ));

        let who = var("Who");
        let query = compound("parent", vec![atom("homer"), Term::Var(who.clone())]);
        let dag = enumerate_all(&query, &kb);

        assert_eq!(dag.proofs.len(), 2);
    }

    #[test]
    fn enumerate_all_path_edge_example_has_two_proofs() {
        // edge(a, b). edge(b, c). edge(a, c).
        // path(X, Y) :- edge(X, Y).
        // path(X, Y) :- edge(X, Z), path(Z, Y).
        // ?- path(a, c).  -> two proofs (direct + via b)
        let mut kb = KnowledgeBase::new();
        kb.add_fact(Fact::certain(compound("edge", vec![atom("a"), atom("b")])));
        kb.add_fact(Fact::certain(compound("edge", vec![atom("b"), atom("c")])));
        kb.add_fact(Fact::certain(compound("edge", vec![atom("a"), atom("c")])));

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
            "path(a, c) has exactly two derivations: direct edge and via b"
        );
    }
}
