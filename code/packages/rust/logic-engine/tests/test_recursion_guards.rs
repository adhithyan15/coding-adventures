//! Recursion guards for both resolvers (RS-4 PR-B security follow-up).
//!
//! Two entry points into the engine were mutually recursive with no
//! termination guard, so a self-recursive rule drove them until the process
//! **overflowed its stack and aborted**. That abort is a `SIGABRT`: it cannot
//! be caught, so a host embedding this crate as a library dies with it.
//!
//! Both are now capped. The tests below are written as **guards against
//! regression of a crash**, so each one is meaningful only if it would
//! previously have aborted — they are not merely asserting a return value.
//!
//! The subtle part is *what the cap returns*. Reporting "no proof" at the cap
//! looks harmless and is not: negation-as-failure succeeds precisely when the
//! proof set is empty, so a truncated search would be read as **absence**, and
//! the engine would assert that a guard held which it never actually checked.
//! A resource limit would have been silently converted into a claim about the
//! world. Hence the caps raise an error that propagates, and the query
//! abstains.

use logic_core::{atom, compound, var, Term};
use logic_engine::{enumerate_all, find_first, BodyLiteral, Fact, KnowledgeBase, Rule};

/// `var()` builds a `LogicVar`; a term argument needs it wrapped.
fn v(name: &str) -> Term {
    Term::Var(var(name))
}

/// `p(X, Y) :- p(X, Y)` — no base case. Plus one real fact, so the KB has a
/// perfectly good answer available; the point is that the *divergent branch*
/// must not take the process down.
fn self_recursive_kb() -> KnowledgeBase {
    let mut kb = KnowledgeBase::new();
    kb.add_fact(Fact::certain(compound("p", vec![atom("a"), atom("b")])));
    kb.add_rule(Rule::certain(
        compound("p", vec![v("X"), v("Y")]),
        vec![BodyLiteral::Pos(compound("p", vec![v("X"), v("Y")]))],
    ));
    kb
}

#[test]
fn enumeration_resolver_survives_a_self_recursive_rule() {
    let kb = self_recursive_kb();
    // Before the guard: "fatal runtime error: stack overflow", SIGABRT.
    let dag = enumerate_all(&compound("p", vec![v("A"), v("B")]), &kb);
    // It abstains rather than reporting the proofs found before the cap —
    // a truncated search presented as a complete one is the accounting
    // failure the whole audit-trail effort exists to prevent.
    assert!(
        dag.proofs.is_empty(),
        "a capped search must abstain, not report a partial result set"
    );
}

#[test]
fn deterministic_resolver_survives_a_self_recursive_rule() {
    let kb = self_recursive_kb();
    // This is the OTHER resolver — reachable via `search(.., AutoDetect)`
    // whenever the KB is all-`Certain`, which is the mode the adjudication
    // connector uses unconditionally. It had the identical defect.
    let answer = find_first(&compound("p", vec![v("A"), v("B")]), &kb);
    // The only requirement is that we got here at all: before the guard this
    // aborted the test process. `find_first` answers a yes/no question, so
    // either outcome is a legitimate answer — what matters is that it is an
    // ANSWER and not a signal.
    let _ = answer;
}

#[test]
fn a_capped_search_under_negation_never_claims_the_goal_is_absent() {
    // THE CRITICAL PROPERTY. `q` is guarded by `not p(X, Y)`, and proving
    // `p` diverges. If the cap were reported as "no proof", the guard would
    // read as satisfied and `q` would be derived — the engine asserting an
    // absence it never established.
    let mut kb = self_recursive_kb();
    kb.add_rule(Rule::certain(
        compound("q", vec![v("X"), v("Y")]),
        vec![BodyLiteral::Neg(compound("p", vec![v("X"), v("Y")]))],
    ));

    let dag = enumerate_all(&compound("q", vec![v("A"), v("B")]), &kb);
    assert!(
        dag.proofs.is_empty(),
        "a goal guarded by a negation whose search hit the cap must NOT be \
         derived — that would fabricate the guard"
    );

    let det = find_first(&compound("q", vec![v("A"), v("B")]), &kb);
    assert!(
        det.is_none(),
        "the deterministic resolver must not fabricate the guard either"
    );
}

#[test]
fn a_pathologically_wide_rule_body_is_rejected_rather_than_overflowing() {
    // A DIFFERENT AXIS from nesting depth. `solve_body` recurses over the
    // body's remaining literals, and `depth` is deliberately constant across a
    // body (all conjuncts of one rule sit at the same level), so the depth cap
    // cannot bound this. Observed to abort at ~14,000 conjuncts.
    let mut kb = KnowledgeBase::new();
    kb.add_fact(Fact::certain(compound("t", vec![atom("x")])));
    let body: Vec<BodyLiteral> = (0..5_000)
        .map(|_| BodyLiteral::Pos(compound("t", vec![atom("x")])))
        .collect();
    kb.add_rule(Rule::certain(compound("wide", vec![]), body));

    let dag = enumerate_all(&Term::Atom("wide".into()), &kb);
    assert!(
        dag.proofs.is_empty(),
        "an over-wide body is refused, not evaluated into a stack overflow"
    );
}

#[test]
fn ordinary_programs_below_the_caps_are_completely_unaffected() {
    // The guards must not cost anything for real programs. A two-level rule
    // chain over a normal body resolves exactly as before.
    let mut kb = KnowledgeBase::new();
    kb.add_fact(Fact::certain(compound("edge", vec![atom("a"), atom("b")])));
    kb.add_rule(Rule::certain(
        compound("path", vec![v("X"), v("Y")]),
        vec![BodyLiteral::Pos(compound("edge", vec![v("X"), v("Y")]))],
    ));
    kb.add_rule(Rule::certain(
        compound("reaches", vec![v("X"), v("Y")]),
        vec![BodyLiteral::Pos(compound("path", vec![v("X"), v("Y")]))],
    ));

    let dag = enumerate_all(&compound("reaches", vec![v("A"), v("B")]), &kb);
    assert_eq!(
        dag.proofs.len(),
        1,
        "the ordinary derivation still resolves"
    );
    // And the nesting the audit trail depends on is intact: rule, rule, fact.
    let depths: Vec<usize> = dag.proofs[0].steps.iter().map(|s| s.depth).collect();
    assert_eq!(depths, vec![0, 1, 2], "preorder depths: {depths:?}");

    assert!(
        find_first(&compound("reaches", vec![v("A"), v("B")]), &kb).is_some(),
        "the deterministic resolver still proves it too"
    );
}
