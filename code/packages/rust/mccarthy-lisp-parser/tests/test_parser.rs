//! Integration tests for `mccarthy-lisp-parser`.
//!
//! Test groups:
//!   1. Atoms
//!   2. Empty list / NIL
//!   3. List nesting (the McCarthy-1960 encoding)
//!   4. Dotted pairs
//!   5. Quote sugar
//!   6. Canonical paper examples (CAR / LAMBDA / LABEL / COND)
//!   7. Error paths
//!   8. Multi-form programs
//!   9. Display round-trip

use mccarthy_lisp_parser::{parse, LispExpr, ParseError};

fn one(src: &str) -> LispExpr {
    let forms = parse(src).expect("parse");
    assert_eq!(forms.len(), 1, "expected exactly one form, got {forms:?}");
    forms.into_iter().next().unwrap()
}

// ============================================================
// 1. Atoms
// ============================================================

#[test]
fn parse_symbol() {
    assert_eq!(one("CAR"), LispExpr::sym("CAR"));
}

#[test]
fn parse_int() {
    assert_eq!(one("42"), LispExpr::Int(42));
}

#[test]
fn parse_negative_int() {
    assert_eq!(one("-1"), LispExpr::Int(-1));
}

// ============================================================
// 2. NIL / empty list
// ============================================================

#[test]
fn empty_list_is_nil() {
    assert_eq!(one("()"), LispExpr::Nil);
}

// ============================================================
// 3. List nesting
// ============================================================

#[test]
fn single_element_list() {
    // (A) == (A . NIL) == Cons(A, Nil)
    let expected = LispExpr::Cons(Box::new(LispExpr::sym("A")), Box::new(LispExpr::Nil));
    assert_eq!(one("(A)"), expected);
}

#[test]
fn three_element_list_uses_nested_cons() {
    // (A B C) == (A . (B . (C . NIL)))
    let expected = LispExpr::list([
        LispExpr::sym("A"),
        LispExpr::sym("B"),
        LispExpr::sym("C"),
    ]);
    assert_eq!(one("(A B C)"), expected);
}

#[test]
fn nested_lists() {
    // (CAR (CDR X)) parses to a 2-element list, second element being a list.
    let expected = LispExpr::list([
        LispExpr::sym("CAR"),
        LispExpr::list([LispExpr::sym("CDR"), LispExpr::sym("X")]),
    ]);
    assert_eq!(one("(CAR (CDR X))"), expected);
}

// ============================================================
// 4. Dotted pairs
// ============================================================

#[test]
fn dotted_pair_atoms() {
    // (A . B) == Cons(A, B) — no NIL terminator.
    let expected = LispExpr::Cons(Box::new(LispExpr::sym("A")), Box::new(LispExpr::sym("B")));
    assert_eq!(one("(A . B)"), expected);
}

#[test]
fn dotted_tail_after_items() {
    // (A B . C) == Cons(A, Cons(B, C))
    let expected = LispExpr::Cons(
        Box::new(LispExpr::sym("A")),
        Box::new(LispExpr::Cons(
            Box::new(LispExpr::sym("B")),
            Box::new(LispExpr::sym("C")),
        )),
    );
    assert_eq!(one("(A B . C)"), expected);
}

// ============================================================
// 5. Quote sugar
// ============================================================

#[test]
fn quote_sugar_atom() {
    // 'X == (QUOTE X) == Cons(QUOTE, Cons(X, NIL))
    let expected = LispExpr::quote(LispExpr::sym("X"));
    assert_eq!(one("'X"), expected);
}

#[test]
fn quote_sugar_list() {
    // '(A B C) == (QUOTE (A B C))
    let expected = LispExpr::quote(LispExpr::list([
        LispExpr::sym("A"),
        LispExpr::sym("B"),
        LispExpr::sym("C"),
    ]));
    assert_eq!(one("'(A B C)"), expected);
}

// ============================================================
// 6. Canonical McCarthy 1960 paper examples
// ============================================================

#[test]
fn car_of_quoted_list_paper_example() {
    // (CAR '(A B C)) — McCarthy 1960, §3 example 1.
    let expected = LispExpr::list([
        LispExpr::sym("CAR"),
        LispExpr::quote(LispExpr::list([
            LispExpr::sym("A"),
            LispExpr::sym("B"),
            LispExpr::sym("C"),
        ])),
    ]);
    assert_eq!(one("(CAR '(A B C))"), expected);
}

#[test]
fn identity_lambda_paper_example() {
    // (LAMBDA (X) X) — McCarthy 1960, §5.
    let expected = LispExpr::list([
        LispExpr::sym("LAMBDA"),
        LispExpr::list([LispExpr::sym("X")]),
        LispExpr::sym("X"),
    ]);
    assert_eq!(one("(LAMBDA (X) X)"), expected);
}

#[test]
fn label_named_recursion_paper_example() {
    // McCarthy 1960 §6: (LABEL FF (LAMBDA (X) (COND ((ATOM X) X) (T (FF (CAR X))))))
    // Just confirm it parses cleanly and the outermost shape is right.
    let forms = parse("(LABEL FF (LAMBDA (X) (COND ((ATOM X) X) (T (FF (CAR X))))))").expect("parse");
    assert_eq!(forms.len(), 1);
    match &forms[0] {
        LispExpr::Cons(car, _) => match car.as_ref() {
            LispExpr::Symbol(s) => assert_eq!(s, "LABEL"),
            other => panic!("expected LABEL, got {other:?}"),
        },
        other => panic!("expected outer Cons, got {other:?}"),
    }
}

// ============================================================
// 7. Error paths
// ============================================================

#[test]
fn unbalanced_open_paren_errors() {
    let err = parse("(CAR").expect_err("missing close paren");
    assert!(matches!(err, ParseError::UnexpectedEof { .. }));
}

#[test]
fn stray_close_paren_errors() {
    let err = parse(")").expect_err("stray rparen");
    assert!(matches!(err, ParseError::UnexpectedToken { token: mccarthy_lisp_lexer::Token::RParen, .. }));
}

#[test]
fn stray_dot_at_top_level_errors() {
    let err = parse(".").expect_err("stray dot");
    assert!(matches!(err, ParseError::StrayDot { .. }));
}

#[test]
fn dot_at_list_head_errors() {
    let err = parse("(. X)").expect_err("dot without car");
    assert!(matches!(err, ParseError::UnexpectedToken { .. }));
}

#[test]
fn dot_without_cdr_errors() {
    let err = parse("(A .)").expect_err("dot without cdr");
    assert!(matches!(err, ParseError::DotWithoutCdr { .. }));
}

#[test]
fn extra_after_dotted_tail_errors() {
    let err = parse("(A . B C)").expect_err("extra after dotted tail");
    assert!(matches!(err, ParseError::ExtraAfterDottedTail { .. }));
}

#[test]
fn multiple_dots_in_list_errors() {
    let err = parse("(A . B . C)").expect_err("multiple dots");
    // The parser flags this at the second dot — could be either
    // MultipleDotsInList (preferred) or ExtraAfterDottedTail
    // depending on which check fires first.
    assert!(matches!(
        err,
        ParseError::MultipleDotsInList { .. } | ParseError::ExtraAfterDottedTail { .. }
    ));
}

#[test]
fn lex_error_propagates_to_parser() {
    let err = parse("(+).").expect_err("`+` is not a valid Lisp 1.0 token");
    assert!(matches!(err, ParseError::Lex(_)));
}

// ============================================================
// 8. Multi-form programs
// ============================================================

#[test]
fn multi_form_program() {
    let forms = parse("(QUOTE A)\n(QUOTE B)\n42").expect("parse");
    assert_eq!(forms.len(), 3);
    assert_eq!(forms[2], LispExpr::Int(42));
}

#[test]
fn empty_program_yields_no_forms() {
    let forms = parse("").expect("parse");
    assert!(forms.is_empty());
}

#[test]
fn whitespace_and_comments_only_yields_no_forms() {
    let forms = parse("\n; comment\n   \n").expect("parse");
    assert!(forms.is_empty());
}

// ============================================================
// 9. Display round-trip
// ============================================================

#[test]
fn display_of_dotted_pair() {
    let e = LispExpr::Cons(Box::new(LispExpr::sym("A")), Box::new(LispExpr::sym("B")));
    assert_eq!(format!("{e}"), "(A . B)");
}

#[test]
fn display_of_nil() {
    assert_eq!(format!("{}", LispExpr::Nil), "NIL");
}

// ============================================================
// 10. DoS-hardening — bounded recursion depth
// ============================================================

#[test]
fn deep_paren_nesting_is_rejected_before_stack_overflow() {
    use mccarthy_lisp_parser::MAX_NESTING;

    // MAX_NESTING + 8 open parens, no close — guaranteed to trip
    // the depth guard before we run out of input or stack.
    let depth = MAX_NESTING + 8;
    let src = "(".repeat(depth);
    let err = parse(&src).expect_err("must trip NestingTooDeep");
    assert!(matches!(err, ParseError::NestingTooDeep { .. }),
        "expected NestingTooDeep, got {err:?}");
}

#[test]
fn deep_quote_chains_are_rejected_before_stack_overflow() {
    use mccarthy_lisp_parser::MAX_NESTING;

    // MAX_NESTING + 8 quote characters, then `X`.  Each `'` recurses
    // through parse_quote → parse_expr; depth guard must catch it.
    let depth = MAX_NESTING + 8;
    let mut src = "'".repeat(depth);
    src.push('X');
    let err = parse(&src).expect_err("must trip NestingTooDeep");
    assert!(matches!(err, ParseError::NestingTooDeep { .. }),
        "expected NestingTooDeep, got {err:?}");
}

#[test]
fn legal_deep_nesting_still_parses() {
    // 100 nested parens with no body — well below MAX_NESTING — must
    // round-trip without the depth guard firing.  Confirms the guard
    // doesn't reject reasonable programs.
    let mut src = "(".repeat(100);
    src.push_str(&")".repeat(100));
    let forms = parse(&src).expect("100 levels of nesting is well within MAX_NESTING");
    assert_eq!(forms.len(), 1);
}

#[test]
fn display_of_list_uses_dotted_form() {
    // McCarthy's printed form for a list IS the dotted form.
    // Pretty-printing as `(A B C)` is a future enhancement; for now
    // the canonical form matches the AST exactly.
    let e = LispExpr::list([LispExpr::sym("A"), LispExpr::sym("B")]);
    assert_eq!(format!("{e}"), "(A . (B . NIL))");
}
