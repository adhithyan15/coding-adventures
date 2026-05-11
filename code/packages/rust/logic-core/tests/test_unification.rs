//! Integration tests for unification, exercising scenarios that span
//! multiple constructors at once. The fine-grained unit tests live next to
//! the implementation inside `src/lib.rs`.

use logic_core::{atom, compound, int, logic_list, string, unify, var, Substitution, Term};

#[test]
fn unifying_a_list_against_a_list_binds_element_variables() {
    // [X, Y, c] = [a, b, c]   ->   X = a, Y = b
    let x = var("X");
    let y = var("Y");

    let pattern = logic_list(vec![Term::Var(x.clone()), Term::Var(y.clone()), atom("c")]);
    let value = logic_list(vec![atom("a"), atom("b"), atom("c")]);

    let s = unify(&pattern, &value, &Substitution::empty()).expect("lists should unify");
    assert_eq!(s.walk_var(&x), atom("a"));
    assert_eq!(s.walk_var(&y), atom("b"));
}

#[test]
fn unifying_compound_with_shared_variable_propagates_binding() {
    // p(X, X) = p(a, a) succeeds with X = a
    // p(X, X) = p(a, b) fails
    let x = var("X");

    let pattern = compound("p", vec![Term::Var(x.clone()), Term::Var(x.clone())]);

    let ok = compound("p", vec![atom("a"), atom("a")]);
    let bad = compound("p", vec![atom("a"), atom("b")]);

    let s = unify(&pattern, &ok, &Substitution::empty()).expect("p(X,X) unifies with p(a,a)");
    assert_eq!(s.walk_var(&x), atom("a"));

    assert!(
        unify(&pattern, &bad, &Substitution::empty()).is_none(),
        "p(X,X) must not unify with p(a,b) because X cannot be both a and b"
    );
}

#[test]
fn strings_and_atoms_are_disjoint_under_unification() {
    // Prolog draws a sharp line between atom 'foo' and string "foo"; we mirror
    // that here so downstream Prolog frontends do not have to special-case.
    let a = atom("foo");
    let s = string("foo");
    assert!(unify(&a, &s, &Substitution::empty()).is_none());
}

#[test]
fn nested_compound_unification_propagates_through_arguments() {
    // pair(pair(X, b), c) = pair(pair(a, b), c) -> X = a
    let x = var("X");
    let inner = compound("pair", vec![Term::Var(x.clone()), atom("b")]);
    let outer = compound("pair", vec![inner, atom("c")]);

    let inner_val = compound("pair", vec![atom("a"), atom("b")]);
    let outer_val = compound("pair", vec![inner_val, atom("c")]);

    let s = unify(&outer, &outer_val, &Substitution::empty()).unwrap();
    assert_eq!(s.walk_var(&x), atom("a"));
}

#[test]
fn integer_terms_unify_when_values_match() {
    assert!(unify(&int(42), &int(42), &Substitution::empty()).is_some());
    assert!(unify(&int(42), &int(43), &Substitution::empty()).is_none());
}
