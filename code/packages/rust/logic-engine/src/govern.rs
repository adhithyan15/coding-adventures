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
//! 2. **Priority** — which competitor wins? Each answer's [`Standing`] is the **maximum** over
//!    the proofs that derive it: the rule [`Priority`] tier, or [`Standing::Asserted`] for a
//!    fact-derived answer (asserted truth outranks any rule tier). Among a conflict group, the
//!    unique maximum **governs**; the rest are **defeated**. A tie at the maximum is a genuine
//!    **conflict** — both are surfaced as [`GovernStatus::ConflictPeer`], never silently
//!    resolved (mirrors the engine's `INDETERMINATE/CONFLICT` stance). Named tiers, not raw
//!    integers (ADJ73 decision 1); richer grounded precedence is PR-B.
//!
//! Defeated/peer answers are *kept and tagged*, not discarded — the audit trail shows exactly
//! what was overridden and by what (`feedback_nothing_human_authored` / provenance-first).

use logic_core::{Substitution, Term};

use crate::enumerate::enumerate_all;
use crate::proof_dag::{DerivationOrigin, ProofDAG};
use crate::{KnowledgeBase, Priority};

/// The precedence STANDING of a derived answer (ADJ73 decision 1: named tiers, not integers).
/// Either the priority tier of the rule that derived it, or [`Standing::Asserted`] when it
/// rests on a ground fact — an asserted truth that outranks every rule tier. Totally ordered
/// by `derive(Ord)`: `Rule` is declared first so `Asserted` is the greatest, and `Rule(p)`
/// compares by its [`Priority`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Standing {
    /// Derived by a rule carrying this priority tier.
    Rule(Priority),
    /// Derived from a ground fact — outranks every rule tier (asserted truth).
    Asserted,
}

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
    /// The highest [`Standing`] over the proofs deriving this answer ([`Standing::Asserted`]
    /// if any derivation rests on a ground fact, else the max rule [`Priority`] tier).
    pub priority: Standing,
    /// ADJ73 PR-B: the CONTEXT this answer is grounded in (the context of its highest-standing
    /// deriving rule) — `None` for a context-free derivation. When two conflicting answers
    /// carry contexts ordered by [`KnowledgeBase::add_context_outranks`], the one in the
    /// greater context defeats the other *before* the [`Standing`] tier is consulted.
    pub context: Option<String>,
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

    /// `true` iff the underlying proof search **stopped early** and therefore
    /// enumerated only some of the answers.
    ///
    /// Any caller about to conclude something NEGATIVE from this result — "no
    /// conflict", "no rival answer", "nothing defeats this" — must check here
    /// first. Those conclusions are drawn by *failing to find* a counterexample,
    /// which proves nothing if the search gave up before looking.
    pub fn truncated(&self) -> bool {
        self.dag.truncated
    }

    /// `true` iff some conflict group had no unique maximum (an unresolved tie). When set, the
    /// caller should abstain or ask rather than pick — there is no governing answer there.
    ///
    /// **`false` is only meaningful when [`truncated`](Self::truncated) is
    /// false.** This method answers "did I *see* a conflict?", and on a
    /// truncated search that is not the same question as "is there one?" —
    /// see [`conflict_status`](Self::conflict_status) for the honest three-way
    /// answer.
    pub fn has_conflict(&self) -> bool {
        self.answers
            .iter()
            .any(|a| a.status == GovernStatus::ConflictPeer)
    }

    /// The honest three-way answer to "is there a conflict?".
    ///
    /// `has_conflict()` collapses two very different situations into `false`:
    /// *I looked and there is none*, and *I never finished looking*. Only the
    /// first licenses acting on the governing answer.
    pub fn conflict_status(&self) -> ConflictStatus {
        if self.has_conflict() {
            ConflictStatus::Conflict
        } else if self.truncated() {
            ConflictStatus::Unknown
        } else {
            ConflictStatus::NoConflict
        }
    }
}

/// Whether a conflict exists among the governed answers — with "I don't know"
/// as a first-class third case rather than an absence silently reported as a
/// negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStatus {
    /// The search completed and found no unresolved tie.
    NoConflict,
    /// The search completed and found a genuine split — abstain or ask.
    Conflict,
    /// The search did NOT complete, so the absence of an observed conflict is
    /// not evidence that none exists.
    Unknown,
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

/// The [`Standing`] a single proof confers on its answer: the priority tier of the rule that
/// derived the query head (the proof's first step), or [`Standing::Asserted`] when the head is
/// a ground fact (asserted truth outranks any rule). An empty/odd proof falls back to the
/// `Default` tier.
fn proof_priority(dag: &ProofDAG, proof_index: usize, kb: &KnowledgeBase) -> Standing {
    match dag.proofs[proof_index].steps.first().map(|s| &s.origin) {
        Some(DerivationOrigin::FromRule(id)) => Standing::Rule(
            kb.find_rule_by_id(*id)
                .map(|r| r.priority)
                .unwrap_or(Priority::Default),
        ),
        Some(DerivationOrigin::FromFact(_)) => Standing::Asserted,
        _ => Standing::Rule(Priority::Default),
    }
}

/// ADJ73 PR-B: the CONTEXT a single proof confers — the `context` of the rule that derived the
/// query head (the proof's first step). Fact-derived heads have no context.
fn proof_context(dag: &ProofDAG, proof_index: usize, kb: &KnowledgeBase) -> Option<String> {
    match dag.proofs[proof_index].steps.first().map(|s| &s.origin) {
        Some(DerivationOrigin::FromRule(id)) => {
            kb.find_rule_by_id(*id).and_then(|r| r.context.clone())
        }
        _ => None,
    }
}

/// ADJ73 PR-B: does answer `a` DEFEAT answer `b` in a conflict group? Context precedence is
/// primary (lex superior): if `a`'s context outranks `b`'s, `a` defeats `b` regardless of tier;
/// if `b`'s outranks `a`'s, it does not. When the contexts are equal / incomparable / absent,
/// the [`Standing`] tier decides. This generalizes the pure-tier rule (with no contexts it
/// reduces to "higher tier defeats lower"), so a rulebook with no `context_order` is unchanged.
fn defeats(a: &GovernedAnswer, b: &GovernedAnswer, kb: &KnowledgeBase) -> bool {
    if let (Some(ca), Some(cb)) = (&a.context, &b.context) {
        if kb.context_outranks(ca, cb) {
            return true;
        }
        if kb.context_outranks(cb, ca) {
            return false;
        }
    }
    a.priority > b.priority
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
        let ctx = proof_context(&dag, i, kb);
        if let Some(a) = answers.iter_mut().find(|a| a.term == term) {
            a.proof_indices.push(i);
            // Track the standing AND the context of the highest-standing derivation, so a
            // context-precedence comparison uses the context of the rule that gave the answer
            // its strongest footing.
            if pri > a.priority {
                a.priority = pri;
                a.context = ctx;
            }
        } else {
            answers.push(GovernedAnswer {
                term,
                priority: pri,
                context: ctx,
                proof_indices: vec![i],
                status: GovernStatus::Governing, // provisional; resolved below
            });
        }
    }

    // 2. Resolve each functional conflict group via the `defeats` relation (context precedence
    //    primary, tier secondary). An answer GOVERNS iff no other answer in its group defeats
    //    it; a sole undefeated answer governs, multiple undefeated answers are conflict peers
    //    (a genuine split), and a defeated answer cites a defeating witness. We compute verdicts
    //    first (immutable borrow), then apply them.
    let keys: Vec<Option<(String, Vec<Term>)>> =
        answers.iter().map(|a| conflict_key(&a.term, kb)).collect();

    // STRICT domination (ADJ73 §4.3): `j` defeats `i` AND `i` does not defeat `j` back. A merely
    // *mutual* defeat — each context outranks the other, e.g. lex superior says federal > state
    // while lex specialis says state > federal — is NOT a clean defeat of either: it is a genuine
    // collision of canons. Resolving it as "both defeated" would silently crown nothing while
    // reporting no conflict; instead a mutually-defeated answer stays UNDEFEATED here, so the group
    // surfaces as `ConflictPeer` (abstain) — the honest "else CONFLICT" the spec promises. (The
    // context order is a partial order + a total tier, so only 2-cycles of mutual defeat arise, not
    // strict Condorcet cycles — transitivity makes any longer cycle mutual everywhere.)
    let dominates = |j: usize, i: usize| -> bool {
        j != i && defeats(&answers[j], &answers[i], kb) && !defeats(&answers[i], &answers[j], kb)
    };
    let undefeated =
        |idx: usize, group: &[usize]| -> bool { !group.iter().any(|&j| dominates(j, idx)) };

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
        verdicts[i] = if !undefeated(i, &group) {
            // Defeated — cite a witness that STRICTLY dominates i (defeats it and isn't defeated
            // back). A merely mutual defeat does not land here; it leaves i undefeated → conflict.
            let by = group
                .iter()
                .find(|&&j| dominates(j, i))
                .map(|&j| answers[j].term.clone())
                .unwrap();
            GovernStatus::Defeated { by }
        } else {
            // i is undefeated. The unique undefeated answer governs; if several are undefeated
            // (incomparable at the top — a true split of authority) they are conflict peers.
            let undefeated_count = group.iter().filter(|&&j| undefeated(j, &group)).count();
            if undefeated_count == 1 {
                GovernStatus::Governing
            } else {
                GovernStatus::ConflictPeer
            }
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
    use crate::{BodyLiteral, Fact, KnowledgeBase, Priority, Provenance, Rule, TrustTier};
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
            .with_priority(Priority::Specific),
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
        kb.add_rule(
            Rule::certain(comp("timing", vec![atom("await")]), vec![])
                .with_priority(Priority::Specific),
        );
        kb.add_rule(
            Rule::certain(comp("timing", vec![atom("treat_now")]), vec![])
                .with_priority(Priority::Specific),
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
                .with_priority(Priority::Specific),
        );
        kb.add_rule(
            Rule::certain(comp("contra", vec![atom("tmp"), atom("preg")]), vec![])
                .with_priority(Priority::Authoritative),
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
            Rule::certain(comp("timing", vec![atom("treat_now")]), vec![])
                .with_priority(Priority::Authoritative),
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
                .with_priority(Priority::Authoritative),
        );
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("waters"), atom("narrow")]), vec![])
                .with_priority(Priority::Specific),
        );
        // a different key (term=person) — independent, should govern on its own.
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("person"), atom("natural")]), vec![])
                .with_priority(Priority::Specific),
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

    // ---- ADJ73 PR-B: grounded context precedence (lex superior) ----

    /// A rule grounded in a higher context governs a conflicting one in a lower context — the
    /// north-star case. ninth_circuit > district_court, so the broad reading governs.
    #[test]
    fn higher_context_governs_the_lower_context() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("means", 2);
        kb.add_context_outranks("ninth_circuit", "district_court");
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("waters"), atom("broad")]), vec![])
                .with_context("ninth_circuit"),
        );
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("waters"), atom("narrow")]), vec![])
                .with_context("district_court"),
        );
        let res = enumerate_governing(&comp("means", vec![atom("waters"), var("R")]), &kb);
        let gov: Vec<&Term> = res.governing().map(|a| &a.term).collect();
        assert_eq!(
            gov,
            vec![&comp("means", vec![atom("waters"), atom("broad")])]
        );
        assert!(!res.has_conflict());
    }

    /// Context precedence is PRIMARY: a higher-context rule with a LOWER tier still defeats a
    /// lower-context rule carrying a higher tier (lex superior beats the explicit tier).
    #[test]
    fn context_precedence_outranks_the_tier() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("rule_on", 1);
        kb.add_context_outranks("federal", "state");
        // federal rule at the LOWEST tier...
        kb.add_rule(
            Rule::certain(comp("rule_on", vec![atom("permitted")]), vec![]).with_context("federal"),
        );
        // ...vs a state rule at the HIGHEST tier — federal still governs.
        kb.add_rule(
            Rule::certain(comp("rule_on", vec![atom("forbidden")]), vec![])
                .with_context("state")
                .with_priority(Priority::Mandatory),
        );
        let res = enumerate_governing(&comp("rule_on", vec![var("X")]), &kb);
        let gov: Vec<&Term> = res.governing().map(|a| &a.term).collect();
        assert_eq!(gov, vec![&comp("rule_on", vec![atom("permitted")])]);
    }

    /// Incomparable contexts (no order between them) fall back to the priority tier.
    #[test]
    fn incomparable_contexts_fall_back_to_tier() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("rule_on", 1);
        // two unrelated contexts — no edge between them.
        kb.add_rule(
            Rule::certain(comp("rule_on", vec![atom("a")]), vec![])
                .with_context("oregon")
                .with_priority(Priority::Authoritative),
        );
        kb.add_rule(
            Rule::certain(comp("rule_on", vec![atom("b")]), vec![])
                .with_context("nevada")
                .with_priority(Priority::Specific),
        );
        let res = enumerate_governing(&comp("rule_on", vec![var("X")]), &kb);
        let gov: Vec<&Term> = res.governing().map(|a| &a.term).collect();
        assert_eq!(gov, vec![&comp("rule_on", vec![atom("a")])]); // higher tier wins the tie
    }

    /// A cyclic context order (a > b, b > a) is detectable and never silently picks a winner.
    #[test]
    fn cyclic_context_order_is_detected_and_governs_nothing() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("means", 2);
        kb.add_context_outranks("a", "b");
        kb.add_context_outranks("b", "a");
        assert!(kb.context_order_has_cycle());
        assert!(kb.context_outranks("a", "b") && kb.context_outranks("b", "a"));
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("t"), atom("x")]), vec![]).with_context("a"),
        );
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("t"), atom("y")]), vec![]).with_context("b"),
        );
        let res = enumerate_governing(&comp("means", vec![atom("t"), var("R")]), &kb);
        assert_eq!(
            res.governing().count(),
            0,
            "a cycle must not crown a winner"
        );
    }

    /// Back-compat: with NO context order declared, context-free rules resolve purely by tier
    /// exactly as before PR-B.
    #[test]
    fn no_context_order_is_pure_tier_resolution() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("timing", 1);
        kb.add_rule(
            Rule::certain(comp("timing", vec![atom("await")]), vec![])
                .with_priority(Priority::Specific),
        );
        kb.add_rule(Rule::certain(
            comp("timing", vec![atom("treat_now")]),
            vec![],
        ));
        let res = enumerate_governing(&comp("timing", vec![var("D")]), &kb);
        let gov: Vec<&Term> = res.governing().map(|a| &a.term).collect();
        assert_eq!(gov, vec![&comp("timing", vec![atom("await")])]);
        assert!(!res.has_conflict());
    }

    /// ADJ73 PR-B-2 — END TO END: the precedence edge is a GROUNDED FACT, not a bare
    /// `add_context_outranks` call. A `relate outranks_context(federal, state)` clause (here a
    /// Fact carrying the Supremacy Clause as provenance) drives the same lex-superior resolution:
    /// the federal reading governs the state reading even though state carries the higher tier,
    /// AND the citation that justifies the precedence is retrievable for the audit trail.
    #[test]
    fn grounded_context_edge_fact_drives_lex_superior_with_provenance() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("means", 2);

        // The PRECEDENCE itself is grounded: federal outranks state *because of* the Supremacy
        // Clause. The reason rides on the edge (one CAS edit away from correctable), not in code.
        let edge_id = kb.add_fact(
            Fact::certain(comp(
                "outranks_context",
                vec![atom("federal"), atom("state")],
            ))
            .with_provenance(Provenance::new(
                "U.S. Const. art. VI, cl. 2 (Supremacy Clause)",
                Some("cl. 2".to_string()),
                TrustTier::Authoritative,
            )),
        );

        // A federal reading at the LOWEST tier...
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("waters"), atom("broad")]), vec![])
                .with_context("federal"),
        );
        // ...vs a state reading at the HIGHEST tier — federal still governs (context is primary).
        kb.add_rule(
            Rule::certain(comp("means", vec![atom("waters"), atom("narrow")]), vec![])
                .with_context("state")
                .with_priority(Priority::Mandatory),
        );

        let res = enumerate_governing(&comp("means", vec![atom("waters"), var("R")]), &kb);
        let gov: Vec<&Term> = res.governing().map(|a| &a.term).collect();
        assert_eq!(
            gov,
            vec![&comp("means", vec![atom("waters"), atom("broad")])],
            "the federal reading governs purely because of the grounded outranks_context edge"
        );
        assert!(!res.has_conflict());

        // The precedence is auditable: the edge fact (and its citation) is recoverable.
        let edge = kb
            .fact(edge_id)
            .expect("the grounded precedence edge is a queryable fact");
        assert_eq!(
            edge.provenance.source,
            "U.S. Const. art. VI, cl. 2 (Supremacy Clause)"
        );
    }

    /// ADJ73 §4.3 — when two canons point OPPOSITE ways (each context outranks the other: lex
    /// superior says federal > state, lex specialis says state > federal), the defeat is MUTUAL.
    /// Neither answer is strictly dominated, so the group is an unresolved CONFLICT (abstain) —
    /// NOT a silent "both defeated, nothing flagged". This is the honest "else CONFLICT" branch.
    #[test]
    fn mutually_outranking_contexts_yield_conflict_not_silent_double_defeat() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("rule_on", 1);
        // Contradictory context order: federal outranks state AND state outranks federal.
        kb.add_context_outranks("federal", "state");
        kb.add_context_outranks("state", "federal");
        kb.add_rule(
            Rule::certain(comp("rule_on", vec![atom("permitted")]), vec![]).with_context("federal"),
        );
        kb.add_rule(
            Rule::certain(comp("rule_on", vec![atom("prohibited")]), vec![]).with_context("state"),
        );
        let res = enumerate_governing(&comp("rule_on", vec![var("X")]), &kb);
        assert!(
            res.has_conflict(),
            "mutual outranking is an honest CONFLICT, not a silent pick"
        );
        assert_eq!(
            res.governing().count(),
            0,
            "nothing governs under contradictory precedence — the caller must abstain"
        );
        // Both are surfaced as peers (neither silently 'defeated').
        assert!(res
            .answers
            .iter()
            .all(|a| a.status == GovernStatus::ConflictPeer));
    }

    /// Guard the common path is unchanged: a ONE-WAY context edge still cleanly defeats (the
    /// strict-domination refinement must not regress the ordinary lex-superior case).
    #[test]
    fn one_way_context_edge_still_cleanly_governs() {
        let mut kb = KnowledgeBase::new();
        kb.declare_functional("rule_on", 1);
        kb.add_context_outranks("federal", "state"); // one direction only
        kb.add_rule(
            Rule::certain(comp("rule_on", vec![atom("permitted")]), vec![]).with_context("federal"),
        );
        kb.add_rule(
            Rule::certain(comp("rule_on", vec![atom("forbidden")]), vec![])
                .with_context("state")
                .with_priority(Priority::Mandatory),
        );
        let res = enumerate_governing(&comp("rule_on", vec![var("X")]), &kb);
        let gov: Vec<&Term> = res.governing().map(|a| &a.term).collect();
        assert_eq!(gov, vec![&comp("rule_on", vec![atom("permitted")])]);
        assert!(!res.has_conflict());
    }
}
