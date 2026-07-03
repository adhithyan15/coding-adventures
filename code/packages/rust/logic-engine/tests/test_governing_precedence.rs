//! ADJ73 integration test: ONE precedence mechanism, two domains.
//!
//! The point of defeasible precedence is that it is *domain-neutral* — the same
//! `enumerate_governing` resolution that picks a medical timing decision also picks the
//! governing legal reading when jurisdictions are ordered. This test exercises the public
//! engine API end-to-end on both, proving the substrate claim from ADJ73 §5 directly (no
//! surface syntax needed — that is PR-2): a medical "default with exception" ladder and a
//! legal "higher court overrides lower" precedence, resolved by the identical machinery.

use logic_core::{LogicVar, Term};
use logic_engine::{
    enumerate_governing, BodyLiteral, Fact, GovernStatus, KnowledgeBase, Priority, Rule,
};

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
    Term::Var(LogicVar::fresh(Some(name)))
}

/// MEDICINE — the MYCIN wait-vs-treat ladder (ADJ73 §5.1), the case `decide_timing` will
/// eventually compile to. A stable, routine, culture-pending patient derives BOTH the
/// `await_culture` exception (priority 10) and the unconditional `treat_now` default
/// (priority 0); `timing` is functional, so they conflict and the exception governs.
#[test]
fn medical_timing_ladder_picks_the_exception_over_the_default() {
    let mut kb = KnowledgeBase::new();
    kb.declare_functional("timing", 1);

    // per-case facts: this patient is stable, the disease is routine, the culture is pending.
    kb.add_fact(Fact::certain(atom("stable")));
    kb.add_fact(Fact::certain(atom("routine")));
    kb.add_fact(Fact::certain(atom("culture_pending")));

    // exception: await the culture when it is safe to wait (priority 10).
    kb.add_rule(
        Rule::certain(
            comp("timing", vec![atom("await_culture")]),
            vec![
                BodyLiteral::Pos(atom("stable")),
                BodyLiteral::Pos(atom("routine")),
                BodyLiteral::Pos(atom("culture_pending")),
            ],
        )
        .with_priority(Priority::Specific),
    );
    // conservative default: treat now (priority 0).
    kb.add_rule(Rule::certain(
        comp("timing", vec![atom("treat_now_empiric")]),
        vec![],
    ));

    let res = enumerate_governing(&comp("timing", vec![var("D")]), &kb);
    let governing: Vec<&Term> = res.governing().map(|a| &a.term).collect();
    assert_eq!(
        governing,
        vec![&comp("timing", vec![atom("await_culture")])],
        "the safe-to-wait exception should govern the conservative default"
    );
    assert!(!res.has_conflict());
    // the default is retained but defeated — the audit trail shows the override.
    let default = res
        .answers
        .iter()
        .find(|a| a.term == comp("timing", vec![atom("treat_now_empiric")]))
        .expect("the defeated default is kept, not discarded");
    assert!(matches!(default.status, GovernStatus::Defeated { .. }));
}

/// A time-critical patient: the exception's body fails (not stable/routine), only the default
/// fires → treat now. Same rulebook, different per-case facts, no conflict.
#[test]
fn medical_timing_falls_back_to_the_default_when_the_exception_does_not_fire() {
    let mut kb = KnowledgeBase::new();
    kb.declare_functional("timing", 1);
    // (no stable/routine/culture_pending facts asserted → the exception body cannot prove.)
    kb.add_rule(
        Rule::certain(
            comp("timing", vec![atom("await_culture")]),
            vec![
                BodyLiteral::Pos(atom("stable")),
                BodyLiteral::Pos(atom("routine")),
                BodyLiteral::Pos(atom("culture_pending")),
            ],
        )
        .with_priority(Priority::Specific),
    );
    kb.add_rule(Rule::certain(
        comp("timing", vec![atom("treat_now_empiric")]),
        vec![],
    ));

    let res = enumerate_governing(&comp("timing", vec![var("D")]), &kb);
    let governing: Vec<&Term> = res.governing().map(|a| &a.term).collect();
    assert_eq!(
        governing,
        vec![&comp("timing", vec![atom("treat_now_empiric")])]
    );
}

/// LAW — the north-star case (ADJ73 §5.3), the SAME mechanism. A term's meaning is derived
/// per governing context; a higher court's reading defeats a lower court's. `means` is
/// functional on its reading per `(term, _)` — here keyed by the term `navigable_waters`.
#[test]
fn legal_higher_court_reading_governs_the_lower_court() {
    let mut kb = KnowledgeBase::new();
    // means(term, reading) — at most one governing reading per term.
    kb.declare_functional("means", 2);

    // ninth_circuit reads "navigable_waters" broadly (higher court → priority 20).
    kb.add_rule(
        Rule::certain(
            comp("means", vec![atom("navigable_waters"), atom("broad")]),
            vec![],
        )
        .with_priority(Priority::Authoritative),
    );
    // a district court reads it narrowly (lower court → priority 10).
    kb.add_rule(
        Rule::certain(
            comp("means", vec![atom("navigable_waters"), atom("narrow")]),
            vec![],
        )
        .with_priority(Priority::Specific),
    );

    let res = enumerate_governing(
        &comp("means", vec![atom("navigable_waters"), var("R")]),
        &kb,
    );
    let governing: Vec<&Term> = res.governing().map(|a| &a.term).collect();
    assert_eq!(
        governing,
        vec![&comp(
            "means",
            vec![atom("navigable_waters"), atom("broad")]
        )],
        "the higher court's reading should govern"
    );
    // the lower reading is retained, defeated by the broad reading (auditable override chain).
    let narrow = res
        .answers
        .iter()
        .find(|a| a.term == comp("means", vec![atom("navigable_waters"), atom("narrow")]))
        .unwrap();
    assert_eq!(
        narrow.status,
        GovernStatus::Defeated {
            by: comp("means", vec![atom("navigable_waters"), atom("broad")])
        }
    );
}

/// Two co-equal authorities (same priority, conflicting readings) → an unresolved CONFLICT,
/// surfaced as peers, never silently resolved. The caller must abstain or seek a tiebreaker —
/// the honest stance for a genuine split of authority (or a clinical equipoise).
#[test]
fn co_equal_authorities_yield_an_unresolved_conflict() {
    let mut kb = KnowledgeBase::new();
    kb.declare_functional("means", 2);
    kb.add_rule(
        Rule::certain(comp("means", vec![atom("term"), atom("reading_a")]), vec![])
            .with_priority(Priority::Authoritative),
    );
    kb.add_rule(
        Rule::certain(comp("means", vec![atom("term"), atom("reading_b")]), vec![])
            .with_priority(Priority::Authoritative),
    );

    let res = enumerate_governing(&comp("means", vec![atom("term"), var("R")]), &kb);
    assert!(
        res.has_conflict(),
        "a split of co-equal authority is a conflict"
    );
    assert_eq!(
        res.governing().count(),
        0,
        "nothing governs — the caller must abstain"
    );
}
