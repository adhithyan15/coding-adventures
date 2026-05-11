//! Integration tests using the classic "family relations" Prolog example,
//! which is the educational running example throughout the LP layer's
//! specs. Verifies that Facts, Rules with bodies, and variable renaming
//! across multiple clause instantiations all compose correctly.

use logic_core::{atom, compound, var, Term};
use logic_engine::{find_first, BodyLiteral, Fact, KnowledgeBase, Rule};

/// Build a small family-tree knowledge base:
///
///   father(homer, bart).
///   father(homer, lisa).
///   father(grandpa_abe, homer).
///   mother(marge, bart).
///   mother(marge, lisa).
///   parent(X, Y) :- father(X, Y).
///   parent(X, Y) :- mother(X, Y).
///   grandparent(X, Z) :- parent(X, Y), parent(Y, Z).
fn make_simpsons_kb() -> KnowledgeBase {
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
        vec![atom("grandpa_abe"), atom("homer")],
    )));
    kb.add_fact(Fact::certain(compound(
        "mother",
        vec![atom("marge"), atom("bart")],
    )));
    kb.add_fact(Fact::certain(compound(
        "mother",
        vec![atom("marge"), atom("lisa")],
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

    let x = var("X");
    let y = var("Y");
    kb.add_rule(Rule::certain(
        compound("parent", vec![Term::Var(x.clone()), Term::Var(y.clone())]),
        vec![BodyLiteral::Pos(compound(
            "mother",
            vec![Term::Var(x.clone()), Term::Var(y.clone())],
        ))],
    ));

    let x = var("X");
    let y = var("Y");
    let z = var("Z");
    kb.add_rule(Rule::certain(
        compound("grandparent", vec![Term::Var(x.clone()), Term::Var(z.clone())]),
        vec![
            BodyLiteral::Pos(compound(
                "parent",
                vec![Term::Var(x.clone()), Term::Var(y.clone())],
            )),
            BodyLiteral::Pos(compound(
                "parent",
                vec![Term::Var(y.clone()), Term::Var(z.clone())],
            )),
        ],
    ));

    kb
}

#[test]
fn direct_father_query_succeeds() {
    let kb = make_simpsons_kb();
    let s = find_first(
        &compound("father", vec![atom("homer"), atom("bart")]),
        &kb,
    )
    .expect("father(homer, bart) is a fact");
    assert!(s == logic_core::Substitution::empty());
}

#[test]
fn parent_rule_resolves_through_father_clause() {
    let kb = make_simpsons_kb();
    let who = var("Who");
    let s = find_first(
        &compound(
            "parent",
            vec![Term::Var(who.clone()), atom("bart")],
        ),
        &kb,
    )
    .expect("parent(Who, bart) should succeed via father(homer, bart)");

    // The first parent rule head matches father; the first father fact for
    // bart is father(homer, bart). So Who = homer.
    assert_eq!(s.walk_var(&who), atom("homer"));
}

#[test]
fn grandparent_query_chains_two_parent_resolutions() {
    let kb = make_simpsons_kb();
    let who = var("Who");
    let s = find_first(
        &compound(
            "grandparent",
            vec![atom("grandpa_abe"), Term::Var(who.clone())],
        ),
        &kb,
    )
    .expect("grandparent(grandpa_abe, Who) should succeed");

    // grandpa_abe -> homer -> {bart, lisa}; first matching is bart.
    assert_eq!(s.walk_var(&who), atom("bart"));
}

#[test]
fn nonexistent_relation_returns_none() {
    let kb = make_simpsons_kb();
    assert!(
        find_first(
            &compound("sibling", vec![atom("bart"), atom("lisa")]),
            &kb
        )
        .is_none(),
        "sibling/2 is not defined; should not be derivable"
    );
}

#[test]
fn knowledge_base_is_all_certain_by_construction() {
    let kb = make_simpsons_kb();
    assert!(
        kb.is_all_certain(),
        "every fact and rule was added with Fact::certain / Rule::certain"
    );
}
