//! End-to-end execution tests: McCarthy source → IIR → run on `twig-vm`.
//!
//! These prove the headline L2a claim — that the emitted IIR actually
//! *runs* and produces the right Lisp value — using `twig-vm`, the
//! cons-capable reference interpreter (`vm-core` is scalar-only and
//! cannot represent symbols / cons cells).
//!
//! Results are inspected through `lispy-runtime`: symbols via the intern
//! table (`name_of`), cons cells via the `car` / `cdr` builtins.

use lispy_runtime::{name_of, LispyValue};
use mccarthy_lisp_iir_compiler::compile_source;
use twig_vm::dispatch::run;

/// Compile + run a McCarthy program, returning the result value.
fn eval(src: &str) -> LispyValue {
    let module = compile_source(src, "e2e").unwrap_or_else(|e| panic!("compile {src:?}: {e}"));
    run(&module).unwrap_or_else(|e| panic!("run {src:?}: {e}"))
}

/// The human-readable name of a symbol value.
fn sym_name(v: LispyValue) -> String {
    let id = v.as_symbol().unwrap_or_else(|| panic!("expected a symbol, got {v}"));
    name_of(id).unwrap_or_else(|| panic!("symbol id {id} has no interned name"))
}

/// car / cdr via the runtime builtins (so tests don't reach into heap unsafe).
fn car(v: LispyValue) -> LispyValue {
    lispy_runtime::builtins::car(&[v]).expect("car")
}
fn cdr(v: LispyValue) -> LispyValue {
    lispy_runtime::builtins::cdr(&[v]).expect("cdr")
}

// ============================================================
// Literals
// ============================================================

#[test]
fn integer_literal() {
    assert_eq!(eval("42").as_int(), Some(42));
    assert_eq!(eval("-7").as_int(), Some(-7));
    assert_eq!(eval("0").as_int(), Some(0));
}

#[test]
fn empty_list_is_nil() {
    assert!(eval("'()").is_nil());
}

#[test]
fn quoted_symbol() {
    assert_eq!(sym_name(eval("'FOO")), "FOO");
}

// ============================================================
// The canonical McCarthy examples
// ============================================================

#[test]
fn car_of_a_literal_list() {
    // (CAR '(A B C)) → A
    assert_eq!(sym_name(eval("(CAR '(A B C))")), "A");
}

#[test]
fn cdr_of_a_literal_list() {
    // (CDR '(A B C)) → (B C)
    let rest = eval("(CDR '(A B C))");
    assert_eq!(sym_name(car(rest)), "B");
    assert_eq!(sym_name(car(cdr(rest))), "C");
    assert!(cdr(cdr(rest)).is_nil());
}

#[test]
fn cadr_composition() {
    // (CAR (CDR '(A B C))) → B
    assert_eq!(sym_name(eval("(CAR (CDR '(A B C)))")), "B");
}

#[test]
fn cons_builds_a_pair() {
    // (CONS 'A 'B) → (A . B)
    let p = eval("(CONS 'A 'B)");
    assert_eq!(sym_name(car(p)), "A");
    assert_eq!(sym_name(cdr(p)), "B");
}

#[test]
fn cons_onto_a_list() {
    // (CONS 'A '(B C)) → (A B C)
    let p = eval("(CONS 'A '(B C))");
    assert_eq!(sym_name(car(p)), "A");
    assert_eq!(sym_name(car(cdr(p))), "B");
    assert_eq!(sym_name(car(cdr(cdr(p)))), "C");
}

// ============================================================
// Predicates
// ============================================================

#[test]
fn atom_of_a_symbol_is_true() {
    assert!(eval("(ATOM 'X)").is_true());
}

#[test]
fn atom_of_a_list_is_false() {
    assert!(eval("(ATOM '(A))").is_false());
}

#[test]
fn atom_of_nil_is_true() {
    // NIL is an atom in McCarthy's algebra.
    assert!(eval("(ATOM '())").is_true());
}

#[test]
fn eq_of_equal_symbols_is_true() {
    assert!(eval("(EQ 'A 'A)").is_true());
}

#[test]
fn eq_of_different_symbols_is_false() {
    assert!(eval("(EQ 'A 'B)").is_false());
}

// ============================================================
// Nesting + sequencing
// ============================================================

#[test]
fn nested_quote_builds_nested_structure() {
    // (CAR '((A B) C)) → (A B)
    let head = eval("(CAR '((A B) C))");
    assert_eq!(sym_name(car(head)), "A");
    assert_eq!(sym_name(car(cdr(head))), "B");
}

#[test]
fn last_form_is_the_program_value() {
    // A sequence of forms returns the value of the last.
    assert_eq!(sym_name(eval("'A 'B 'C")), "C");
}

#[test]
fn dotted_pair_literal() {
    // (CDR '(A . B)) → B
    assert_eq!(sym_name(eval("(CDR '(A . B))")), "B");
}
