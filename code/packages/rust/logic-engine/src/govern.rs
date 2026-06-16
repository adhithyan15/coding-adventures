//! ADJ73 — defeasible rule precedence: resolve *conflicting* derivations by priority.
//!
//! [`enumerate_all`](crate::enumerate::enumerate_all) collects EVERY proof of a query. That
//! is correct for monotonic knowledge but wrong for the dominant real-world rulebook shape —
//! **defaults with exceptions**, where two rules derive conclusions that cannot both hold and
//! a priority decides which one *governs*. This module adds that resolution as a **post-pass**
//! over the proofs `enumerate_all` already produces: SLD search is untouched, so any query
//! over predicates that are not declared functional is byte-identical to before.
//!
//! ## The two ingredients (see ADJ73 §2)
//!
//! 1. **Conflict** — when can two answers compete? In PR-1 the relation is *functional
//!    predicates*: a predicate declared via [`KnowledgeBase::declare_functional`] admits at
//!    most one value on its **last argument**, keyed by the preceding arguments. So for
//!    `timing/1` (key = `()`), `timing(await)` and `timing(treat_now)` conflict; for a
//!    hypothetical `means(term, reading, ctx)` declared functional, two answers conflict only
//!    when they share `(term, ctx)` but differ on `reading`. A predicate that is NOT declared
//!    functional never conflicts → every answer governs (back-compat).
//! 2. **Priority** — which competitor wins? Each answer's priority is the **maximum**
//!    [`Rule::priority`](crate::Rule) over the proofs that derive it (a fact-derived answer is
//!    [`i64::MAX`] — an asserted truth is never defeated by a rule). Among a conflict group,
//!    the unique maximum **governs**; the rest are **defeated**. A tie at the maximum is a
//!    genuine **conflict** — both are surfaced as [`GovernStatus::ConflictPeer`], never
//!    silently resolved (mirrors the engine's `INDETERMINATE/CONFLICT` stance).
//!
//! Defeated/peer answers are *kept and tagged*, not discarded — the audit trail shows exactly
//! what was overridden and by what (`feedback_nothing_human_authored` / provenance-first).

use logic_core::{Substitution, Term};

use crate::enumerate::enumerate_all;
use crate::proof_dag::{DerivationOrigin, ProofDAG};
use crate::KnowledgeBase;

/// The governance verdict for one distinct answer term.
#[derive(Debug, Clone, PartialEq)]
pub enum GovernStatus {
    /// This answer governs — no conflicting answer outranks it (it is the unique maximum of
    /// its conflict group, or its predicate is non-functional / the group is a singleton).
    Governing,
    /// A higher-priority conflicting answer defeated this one. `by` is the governing term.
    Defeated { by: Term },
    /// This answer ties at the maximum priority with one or more conflicting answers — an
    /// unresolved conflict. Both peers are surfaced; the caller decides (abstain / ask).
    ConflictPeer,
}

/// One distinct answer to the query, with its governance verdict.
#[derive(Debug, Clone, PartialEq)]
pub struct GovernedAnswer {
    /// The ground answer term (the query with this proof's bindings applied).
    pub term: Term,
    /// Max [`Rule::priority`](crate::Rule) over the proofs deriving this answer
    /// (`i64::MAX` if any derivation is a bare fact).
    pub priority: i64,
    /// Indices into [`GovernedResult::dag`]`.proofs` that derive this answer.
    pub proof_indices: Vec<usize>,
    /// Whether this answer governs, was defeated, or is an unresolved conflict peer.
    pub status: GovernStatus,
}

/// The result of a governing query: the full proof DAG (unchanged from `enumerate_all`) plus
/// one [`GovernedAnswer`] per distinct answer term, each tagged with its verdict.
#[derive(Debug, Clone)]
pub struct GovernedResult {
    pub dag: ProofDAG,
    pub answers: Vec<GovernedAnswer>,
}

impl GovernedResult {
    /// The answers that govern (the engine's actual conclusions). Excludes defeated answers
    /// and conflict peers — for the conflict case the caller should inspect [`Self::conflicts`].
    pub fn governing(&self) -> impl Iterator<Item = &GovernedAnswer> {
        self.answers
            .iter()
            .filter(|a| a.status == GovernStatus::Governing)
    }

    /// `true` iff some conflict group had no unique maximum (an unresolved tie). When set, the
    /// caller should abstain or ask rather than pick — there is no governing answer there.
    pub fn has_conflict(&self) -> bool {
        self.answers
            .iter()
            .any(|a| a.status == GovernStatus::ConflictPeer)
    }
}

/// Deep-resolve a term under a substitution (the engine's `Substitution::walk` only resolves
/// the top level; an answer term may have bound variables nested inside compound arguments).
fn resolve_deep(term: &Term, subst: &Substitution) -> Term {
    match subst.walk(term) {
        Term::Compound { functor, args } => Term::Compound {
            functor,
            args: args.iter().map(|a| resolve_deep(a, subst)).collect(),
        },
        other => other,
    }
}

/// The conflict KEY for a functional answer: `(functor, args[..last])`. Two answers conflict
/// iff they share a key but differ as whole terms. Returns `None` if the term's predicate is
/// not functional (or is not a compound), meaning the answer never conflicts.
fn conflict_key(term: &Term, kb: &KnowledgeBase) -> Option<(String, Vec<Term>)> {
    if !kb.is_functional(term) {
        return None;
    }
    match term {
        // Functional on the LAST argument → key is the functor + all preceding args.
        Term::Compound { functor, args } if !args.is_empty() => {
            Some((functor.clone(), args[..args.len() - 1].to_vec()))
        }
        _ => None,
    }
}

/// The priority a single proof confers on its answer: the priority of the rule that derived the
/// query head (the proof's first step). A fact-derived head is `i64::MAX` — an asserted truth
/// outranks any rule. An empty/odd proof falls back to `0` (the default rule priority).
fn proof_priority(dag: &ProofDAG, proof_index: usize, kb: &KnowledgeBase) -> i64 {
    match dag.proofs[proof_index].steps.first().map(|s| &s.origin) {
        Some(DerivationOrigin::FromRule(id)) => {
            kb.find_rule_by_id(*id).map(|r| r.priority).unwrap_or(0)
        }
        Some(DerivationOrigin::FromFact(_)) => i64::MAX,
        _ => 0,
    }
}

/// Enumerate all proofs of `query`, then resolve conflicting answers by defeasible precedence
/// (ADJ73). Returns every distinct answer tagged [`GovernStatus`]. For a query over predicates
/// none of which are declared functional, every answer is [`GovernStatus::Governing`] and the
/// result is just `enumerate_all` grouped by distinct answer.
pub fn enumerate_governing(query: &Term, kb: &KnowledgeBase) -> GovernedResult {
    let dag = enumerate_all(query, kb);

    // 1. Collapse proofs into distinct answer terms (Term has no Hash/Eq — linear scan, as the
    //    KB does for priors). Each answer accumulates its proof indices + its max priority.
    let mut answers: Vec<GovernedAnswer> = Vec::new();
    for (i, proof) in dag.proofs.iter().enumerate() {
        let term = resolve_deep(query, &proof.bindings);
        let pri = proof_priority(&dag, i, kb);
        if let Some(a) = answers.iter_mut().find(|a| a.term == term) {
            a.proof_indices.push(i);
            a.priority = a.priority.max(pri);
        } else {
            answers.push(GovernedAnswer {
                term,
                priority: pri,
                proof_indices: vec![i],
                status: GovernStatus::Governing, // provisional; resolved below
            });
        }
    }

    // 2. Resolve each functional conflict group. We compute verdicts first (immutable borrow),
    //    then apply them, to keep the borrow checker happy and the logic readable.
    let keys: Vec<Option<(String, Vec<Term>)>> =
        answers.iter().map(|a| conflict_key(&a.term, kb)).collect();

    let mut verdicts: Vec<GovernStatus> = vec![GovernStatus::Governing; answers.len()];
    for (i, key_i) in keys.iter().enumerate() {
        let Some(key_i) = key_i else { continue }; // non-functional → always governs
                                                   // The conflict group = every answer sharing this key (including i itself).
        let group: Vec<usize> = keys
            .iter()
            .enumerate()
            .filter(|(_, k)| k.as_ref() == Some(key_i))
            .map(|(j, _)| j)
            .collect();
        if group.len() < 2 {
            continue; // singleton → no contest
        }
        let max_pri = group.iter().map(|&j| answers[j].priority).max().unwrap();
        let winners: Vec<usize> = group
            .iter()
            .copied()
            .filter(|&j| answers[j].priority == max_pri)
            .collect();
        verdicts[i] = if answers[i].priority < max_pri {
            // Defeated — cite a governing winner. With a unique winner that is THE governor;
            // with a tie, any peer is a valid "defeated by" witness.
            GovernStatus::Defeated {
                by: answers[winners[0]].term.clone(),
            }
        } else if winners.len() == 1 {
            GovernStatus::Governing
        } else {
            GovernStatus::ConflictPeer // tied at the top with another answer
        };
    }
    for (a, v) in answers.iter_mut().zip(verdicts) {
        a.status = v;
    }

    GovernedResult { dag, answers }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BodyLiteral, Fact, KnowledgeBase, Rule};
    use logic_core::Term;

    fn atom(s: &str) -> Term {
        Term::Atom(s.to_string())
    }
    fn comp(f: &str, args: Vec<Term>) -> Term {
        Term::Compound {
            functor: f.to_string(),
            args,
        }
    }
    fn var(name: &str) -> Term {
        Term::Var(logic_core::LogicVar::fresh(Some(name)))
    }

    /// A higher-priority rule defeats the lower default for a functional predicate.
    #[test]
    fn higher_priority_rule_governs_and_default_is_defeated() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("timing", 1);
        // active_context fact gates the specific rule.
        kb.add_fact(Fact::certain(atom("stable_routine_pending")));
        // specific: timing(await) when stable_routine_pending — priority 10
        kb.add_rule(
            Rule::certain(
                comp("timing", vec![atom("await")]),
                vec![BodyLiteral::Pos(atom("stable_routine_pending"))],
            )
            .with_priority(10),
        );
        // default: timing(treat_now) unconditionally — priority 0
        kb.add_rule(Rule::certain(
            comp("timing", vec![atom("treat_now")]),
            vec![],
        ));

        let res = enumerate_governing(&comp("timing", vec![var("D")]), &kb);
        let governing: Vec<&Term> = res.governing().map(|a| &a.term).collect();
        assert_eq!(governing, vec![&comp("timing", vec![atom("await")])]);
        assert!(!res.has_conflict());
        // the default is present but defeated, citing the winner.
        let default = res
            .answers
            .iter()
            .find(|a| a.term == comp("timing", vec![atom("treat_now")]))
            .unwrap();
        assert_eq!(
            default.status,
            GovernStatus::Defeated {
                by: comp("timing", vec![atom("await")])
            }
        );
    }

    /// Two equal-priority conflicting rules → an unresolved conflict (both peers, no governor).
    #[test]
    fn equal_priority_conflict_yields_peers_not_a_silent_pick() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("timing", 1);
        kb.add_rule(Rule::certain(comp("timing", vec![atom("await")]), vec![]).with_priority(5));
        kb.add_rule(
            Rule::certain(comp("timing", vec![atom("treat_now")]), vec![]).with_priority(5),
        );

        let res = enumerate_governing(&comp("timing", vec![var("D")]), &kb);
        assert!(res.has_conflict());
        assert_eq!(res.governing().count(), 0); // nothing governs — the caller must abstain
        assert!(res
            .answers
            .iter()
            .all(|a| a.status == GovernStatus::ConflictPeer));
    }

    /// A non-functional predicate keeps every derivation (back-compat: no defeat at all).
    #[test]
    fn non_functional_predicate_keeps_every_answer() {
        let mut kb = KnowledgeBase::new();
        // contraindicated/2 is NOT declared functional — many may hold.
        kb.add_rule(
            Rule::certain(comp("contra", vec![atom("moxi"), atom("preg")]), vec![])
                .with_priority(1),
        );
        kb.add_rule(
            Rule::certain(comp("contra", vec![atom("tmp"), atom("preg")]), vec![]).with_priority(9),
        );

        let res = enumerate_governing(&comp("contra", vec![var("D"), var("C")]), &kb);
        assert_eq!(res.governing().count(), 2); // both govern despite differing priority
        assert!(!res.has_conflict());
    }

    /// An asserted fact outranks a conflicting rule-derived default (fact priority = i64::MAX).
    #[test]
    fn a_fact_defeats_a_conflicting_rule() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("timing", 1);
        kb.add_fact(Fact::certain(comp("timing", vec![atom("targeted")])));
        kb.add_rule(
            Rule::certain(comp("timing", vec![atom("treat_now")]), vec![]).with_priority(50),
        );

        let res = enumerate_governing(&comp("timing", vec![var("D")]), &kb);
        let governing: Vec<&Term> = res.governing().map(|a| &a.term).collect();
        assert_eq!(governing, vec![&comp("timing", vec![atom("targeted")])]);
    }

    /// Functional on the LAST arg, keyed by the rest: answers with different keys do NOT
    /// conflict, but two values under the same key do.
    #[test]
    fn functional_keys_isolate_independent_conflicts() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("means", 2); // means(term, reading) functional on reading per term
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("waters"), atom("broad")]), vec![])
                .with_priority(2),
        );
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("waters"), atom("narrow")]), vec![])
                .with_priority(1),
        );
        // a different key (term=person) — independent, should govern on its own.
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("person"), atom("natural")]), vec![])
                .with_priority(1),
        );

        let res = enumerate_governing(&comp("means", vec![var("T"), var("R")]), &kb);
        let mut governing: Vec<Term> = res.governing().map(|a| a.term.clone()).collect();
        governing.sort_by_key(|t| format!("{t:?}"));
        assert_eq!(
            governing,
            vec![
                comp("means", vec![atom("person"), atom("natural")]),
                comp("means", vec![atom("waters"), atom("broad")]),
            ]
        );
        assert!(!res.has_conflict());
    }
}
