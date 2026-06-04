//! Integration tests for `mccarthy-lisp-parser`.
//!
//! The crate is a thin wrapper over the shared `GrammarParser` plus a
//! typed-AST extractor.  These tests pin the *McCarthy-1960 AST
//! contract* — the desugaring of lists and quote, the dotted-pair
//! shape, and the canonical paper examples — plus the error paths that
//! the grammar now enforces for free.

use mccarthy_lisp_parser::{parse, LispExpr};

/// Parse a source that must contain exactly one top-level form.
fn one(src: &str) -> LispExpr {
    let mut forms = parse(src).unwrap_or_else(|e| panic!("parse failed for {src:?}: {e}"));
    assert_eq!(forms.len(), 1, "expected exactly one form in {src:?}");
    forms.pop().unwrap()
}

// ============================================================
// 1. Atoms
// ============================================================

#[test]
fn symbol_atom() {
    assert_eq!(one("CAR"), LispExpr::sym("CAR"));
    assert_eq!(one("FF"), LispExpr::sym("FF"));
    assert_eq!(one("LIST-OF-3"), LispExpr::sym("LIST-OF-3"));
}

#[test]
fn integer_atom() {
    assert_eq!(one("0"), LispExpr::Int(0));
    assert_eq!(one("42"), LispExpr::Int(42));
    assert_eq!(one("-7"), LispExpr::Int(-7));
}

// ============================================================
// 2. Empty list / NIL
// ============================================================

#[test]
fn empty_list_is_nil() {
    assert_eq!(one("()"), LispExpr::Nil);
}

#[test]
fn nil_symbol_is_a_symbol() {
    // At the parser layer, `NIL` the symbol is distinct from `()`.
    assert_eq!(one("NIL"), LispExpr::sym("NIL"));
}

// ============================================================
// 3. List nesting (the McCarthy-1960 encoding)
// ============================================================

#[test]
fn proper_list_nests_to_cons_cells() {
    // (A B C) → (A . (B . (C . NIL)))
    let expected = LispExpr::Cons(
        Box::new(LispExpr::sym("A")),
        Box::new(LispExpr::Cons(
            Box::new(LispExpr::sym("B")),
            Box::new(LispExpr::Cons(Box::new(LispExpr::sym("C")), Box::new(LispExpr::Nil))),
        )),
    );
    assert_eq!(one("(A B C)"), expected);
    assert_eq!(one("(A B C)"), LispExpr::list([LispExpr::sym("A"), LispExpr::sym("B"), LispExpr::sym("C")]));
}

#[test]
fn singleton_list() {
    assert_eq!(one("(A)"), LispExpr::list([LispExpr::sym("A")]));
}

#[test]
fn nested_lists() {
    assert_eq!(
        one("((A) (B))"),
        LispExpr::list([
            LispExpr::list([LispExpr::sym("A")]),
            LispExpr::list([LispExpr::sym("B")]),
        ])
    );
}

// ============================================================
// 4. Dotted pairs
// ============================================================

#[test]
fn simple_dotted_pair() {
    assert_eq!(
        one("(A . B)"),
        LispExpr::Cons(Box::new(LispExpr::sym("A")), Box::new(LispExpr::sym("B")))
    );
}

#[test]
fn dotted_tail_after_elements() {
    // (A B . C) → (A . (B . C))
    assert_eq!(
        one("(A B . C)"),
        LispExpr::Cons(
            Box::new(LispExpr::sym("A")),
            Box::new(LispExpr::Cons(Box::new(LispExpr::sym("B")), Box::new(LispExpr::sym("C"))))
        )
    );
}

#[test]
fn dotted_pair_with_integer_cdr() {
    assert_eq!(
        one("(A . -42)"),
        LispExpr::Cons(Box::new(LispExpr::sym("A")), Box::new(LispExpr::Int(-42)))
    );
}

// ============================================================
// 5. Quote sugar
// ============================================================

#[test]
fn quote_a_symbol() {
    assert_eq!(one("'X"), LispExpr::quote(LispExpr::sym("X")));
}

#[test]
fn quote_a_list() {
    assert_eq!(
        one("'(A B C)"),
        LispExpr::quote(LispExpr::list([
            LispExpr::sym("A"),
            LispExpr::sym("B"),
            LispExpr::sym("C"),
        ]))
    );
}

#[test]
fn nested_quotes() {
    // ''X → (QUOTE (QUOTE X))
    assert_eq!(one("''X"), LispExpr::quote(LispExpr::quote(LispExpr::sym("X"))));
}

// ============================================================
// 6. Canonical paper examples
// ============================================================

#[test]
fn car_of_a_literal_list() {
    assert_eq!(
        one("(CAR '(A B C))"),
        LispExpr::list([
            LispExpr::sym("CAR"),
            LispExpr::quote(LispExpr::list([
                LispExpr::sym("A"),
                LispExpr::sym("B"),
                LispExpr::sym("C"),
            ])),
        ])
    );
}

#[test]
fn identity_lambda() {
    assert_eq!(
        one("(LAMBDA (X) X)"),
        LispExpr::list([
            LispExpr::sym("LAMBDA"),
            LispExpr::list([LispExpr::sym("X")]),
            LispExpr::sym("X"),
        ])
    );
}

#[test]
fn label_recursive_definition() {
    // (LABEL FF (LAMBDA (X) (COND ((ATOM X) X) ((QUOTE T) (FF (CAR X))))))
    let src = "(LABEL FF (LAMBDA (X) (COND ((ATOM X) X) ('T (FF (CAR X))))))";
    let form = one(src);
    // Spot-check the spine: (LABEL FF (LAMBDA ...))
    match form {
        LispExpr::Cons(ref head, _) => assert_eq!(**head, LispExpr::sym("LABEL")),
        other => panic!("expected a Cons, got {other:?}"),
    }
}

#[test]
fn cond_form() {
    let src = "(COND ((ATOM X) (QUOTE A)) ('T (QUOTE B)))";
    let form = one(src);
    match form {
        LispExpr::Cons(ref head, _) => assert_eq!(**head, LispExpr::sym("COND")),
        other => panic!("expected a Cons, got {other:?}"),
    }
}

// ============================================================
// 7. Error paths (now enforced by the grammar)
// ============================================================

#[test]
fn unbalanced_parens_error() {
    assert!(parse("(CAR").is_err());
    assert!(parse("CAR)").is_err());
    assert!(parse("(((").is_err());
}

#[test]
fn malformed_dotted_forms_error() {
    assert!(parse("(. X)").is_err()); // no head before dot
    assert!(parse("(A . B C)").is_err()); // extra element after dotted tail
    assert!(parse("(A . . B)").is_err()); // two dots
    assert!(parse("(A .)").is_err()); // dot with no cdr
}

#[test]
fn dialect_violations_error() {
    assert!(parse("car").is_err()); // lowercase
    assert!(parse("(+ A B)").is_err()); // operator symbol
    assert!(parse("\"hi\"").is_err()); // string literal
}

#[test]
fn integer_overflow_errors() {
    assert!(parse("123456789012345678901234567890").is_err());
}

#[test]
fn pathological_nesting_does_not_crash() {
    // DoS hardening: neither a deep paren nest nor a long quote chain
    // (which has paren-depth 0 but unbounded parser recursion) may abort
    // the process — both must return a clean Err.
    assert!(parse(&"(".repeat(10_000)).is_err());
    assert!(parse(&format!("{}X", "'".repeat(10_000))).is_err());
}

#[test]
fn huge_flat_list_builds_and_drops_without_overflow() {
    // A flat list `(A A … A)` is only paren-depth 1, so it bypasses the
    // nesting cap, yet it builds an N-deep Cons chain. Both building it
    // (parser) and dropping it (LispExpr's iterative Drop) must avoid
    // recursing to a stack overflow. 100k elements is well past the
    // recursive-drop overflow point.
    let n = 100_000;
    let mut src = String::with_capacity(2 * n + 2);
    src.push('(');
    for _ in 0..n {
        src.push_str("A ");
    }
    src.push(')');
    let forms = parse(&src).expect("huge flat list should parse");
    assert_eq!(forms.len(), 1);
    drop(forms); // force the iterative Drop here — passing means no overflow
}

// ============================================================
// 8. Multi-form programs
// ============================================================

#[test]
fn sequence_of_forms() {
    let forms = parse("(CAR X)\n(CDR X)\nNIL").unwrap();
    assert_eq!(forms.len(), 3);
    assert_eq!(forms[2], LispExpr::sym("NIL"));
}

// ============================================================
// 9. Display round-trip
// ============================================================

#[test]
fn display_prints_dotted_cons_form() {
    assert_eq!(one("(A B)").to_string(), "(A . (B . NIL))");
    assert_eq!(one("(A . B)").to_string(), "(A . B)");
    assert_eq!(one("NIL").to_string(), "NIL");
    assert_eq!(one("()").to_string(), "NIL");
    assert_eq!(one("-5").to_string(), "-5");
}
