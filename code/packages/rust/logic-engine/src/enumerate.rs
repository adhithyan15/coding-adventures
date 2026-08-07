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

use logic_core::{unify, LogicVar, Substitution, Term};

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

/// The deepest chain of rule applications the resolver will follow before
/// giving up.
///
/// # Why a cap is required at all
///
/// `solve` and `solve_body` are mutually recursive, and a self-recursive rule
/// (`p(X) :- p(X)`) has no base case — the resolver descends until the process
/// **overflows its stack and aborts**. That abort is a `SIGABRT`: it cannot be
/// caught, so an embedding process dies with it, not just the CLI.
///
/// Every other recursive descent in the stack is already capped —
/// `adj_lang::MAX_RULE_DEPTH` in the parser, `compute::MAX_EVAL_DEPTH` in the
/// arithmetic evaluator. The resolver was the remaining hole. 128 sits above
/// the parser's own 90-deep rule limit, so no program the frontend accepts can
/// reach it by legitimate nesting.
pub const MAX_SLD_DEPTH: usize = 128;

/// The most conjuncts a single rule body may contain.
///
/// `solve_body` recurses over the body's *remaining* literals, and that
/// recursion is a **different axis** from `MAX_SLD_DEPTH`: `depth` is
/// deliberately held constant across a body (all conjuncts of one rule sit at
/// the same nesting level), so it cannot bound body length. A rule with ~14,000
/// conjuncts overflows the stack even though its `depth` never exceeds 1.
///
/// Capping the length at entry bounds the recursion directly, since the slice
/// shrinks by one per frame. 1024 is far past any hand-written or generated
/// rule and an order of magnitude below the observed failure point.
pub const MAX_BODY_CONJUNCTS: usize = 1024;

/// The resolver abandoned the search because it hit [`MAX_SLD_DEPTH`].
///
/// This is deliberately an **error, not an empty result**. The distinction is
/// the whole point: "I found no proof" and "I stopped looking" are different
/// claims, and conflating them is exactly the accounting failure the audit
/// trail exists to prevent. It matters most under negation — see `solve_body`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolutionLimitExceeded;

/// Enumerate every successful proof of `query` against `kb`. Returns a
/// `ProofDAG` containing one `Proof` per successful derivation.
///
/// If the search hits [`MAX_SLD_DEPTH`], this returns a DAG with **no proofs**
/// — i.e. the caller abstains. Reporting the proofs found before the cap would
/// present a truncated search as a complete one, which is the failure mode the
/// whole audit-trail effort is aimed at.
pub fn enumerate_all(query: &Term, kb: &KnowledgeBase) -> ProofDAG {
    let outcome = solve(query, kb, &Substitution::empty(), 0);
    // The limit is RECORDED on the DAG, not silently swallowed. Callers that
    // draw a negative conclusion from an empty result set must be able to see
    // that the search stopped early — see `ProofDAG::truncated`.
    let truncated = outcome.is_err();
    let raw = outcome.unwrap_or_default();
    let proofs = raw
        .into_iter()
        .map(|(bindings, steps)| {
            let (via_facts, via_rules) = collect_ids(&steps);
            Proof {
                bindings,
                steps,
                via_facts,
                via_rules,
                // SLD-resolution proofs leave the LP19e LR-aggregation
                // fields empty. Setting them to None is the documented
                // signal that the proof was produced by the
                // WMC/EnumerateAll path, not LRAggregate.
                posterior_logit: None,
                posterior_probability: None,
            }
        })
        .collect();
    ProofDAG {
        root_query: query.clone(),
        proofs,
        truncated,
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
    depth: usize,
) -> Result<Vec<(Substitution, Vec<ProofStep>)>, ResolutionLimitExceeded> {
    // The guard that turns an infinite descent into an honest abstention.
    if depth >= MAX_SLD_DEPTH {
        return Err(ResolutionLimitExceeded);
    }
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
                depth,
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
            // The body is proved one level DEEPER than the rule step that
            // introduces it. That single `+ 1` is what turns the flat step
            // vector into a reconstructable tree (see `ProofStep::depth`).
            for (body_subst, body_steps) in solve_body(&renamed_body, kb, &s, depth + 1)? {
                let mut steps = Vec::with_capacity(1 + body_steps.len());
                steps.push(ProofStep {
                    goal: resolved.clone(),
                    origin: DerivationOrigin::FromRule(rule.id),
                    depth,
                });
                steps.extend(body_steps);
                results.push((body_subst, steps));
            }
        }
    }

    Ok(results)
}

/// Prove every literal in `body`, threading substitutions forward and
/// enumerating every combination of body-literal proofs. Returns the
/// full list of (final-substitution, accumulated-steps) pairs.
fn solve_body(
    body: &[BodyLiteral],
    kb: &KnowledgeBase,
    subst: &Substitution,
    depth: usize,
) -> Result<Vec<(Substitution, Vec<ProofStep>)>, ResolutionLimitExceeded> {
    if body.is_empty() {
        return Ok(vec![(subst.clone(), Vec::new())]);
    }
    // Bounds the `rest` recursion below — see MAX_BODY_CONJUNCTS. `Err`, not an
    // empty vec, for the same reason as the depth cap: a body we refused to
    // evaluate must never be observable as a goal that failed.
    if body.len() > MAX_BODY_CONJUNCTS {
        return Err(ResolutionLimitExceeded);
    }

    let (first, rest) = body.split_first().unwrap();
    let mut results = Vec::new();

    match first {
        BodyLiteral::Pos(t) => {
            // Find every way to prove `t`; for each, recurse on `rest`.
            for (after_first, steps_first) in solve(t, kb, subst, depth)? {
                for (after_rest, steps_rest) in solve_body(rest, kb, &after_first, depth)? {
                    let mut all_steps = Vec::with_capacity(steps_first.len() + steps_rest.len());
                    all_steps.extend(steps_first.iter().cloned());
                    all_steps.extend(steps_rest);
                    results.push((after_rest, all_steps));
                }
            }
        }
        BodyLiteral::Neg(t) => {
            // Negation-as-failure: succeed iff `t` has zero proofs.
            //
            // This RECORDS A STEP. It previously recorded none, which meant
            // a rule guarded by `not contraindicated(D)` would fire and the
            // audit trail would never mention the guard — a reader could not
            // distinguish "we checked, and found no contraindication" from
            // "nobody checked." The absence IS the justification, so it has
            // to appear in the trail like any other justification.
            //
            // The substitution is unchanged: NAF binds nothing (it succeeded
            // precisely because there was no proof to bind from).
            //
            // THE CAP MUST NOT BE READ AS ABSENCE. `?` here is load-bearing:
            // if the negated subgoal's own search hit MAX_SLD_DEPTH we
            // propagate the error instead of observing an empty result set.
            // Swallowing it would let a truncated search masquerade as "no
            // proof exists", and this function would then emit a
            // `FromNegation` step asserting a guard held that was never
            // actually established — a fabricated justification in the audit
            // trail, which is worse than the crash it replaces.
            if solve(t, kb, subst, depth + 1)?.is_empty() {
                let neg_goal = subst.walk(t);
                for (after_rest, steps_rest) in solve_body(rest, kb, subst, depth)? {
                    let mut all_steps = Vec::with_capacity(1 + steps_rest.len());
                    all_steps.push(ProofStep {
                        goal: neg_goal.clone(),
                        origin: DerivationOrigin::FromNegation {
                            goal: neg_goal.clone(),
                        },
                        depth,
                    });
                    all_steps.extend(steps_rest);
                    results.push((after_rest, all_steps));
                }
            }
        }
    }

    Ok(results)
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
        children.sort_by_key(|a| a.to_string());
        assert_eq!(children, vec![atom("bart"), atom("lisa"), atom("maggie")]);
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
