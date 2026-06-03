//! Integration tests for `mccarthy-lisp-lexer`.
//!
//! Test groups:
//!   1. Single-token shapes
//!   2. Canonical McCarthy 1960 paper expressions
//!   3. Whitespace + comments
//!   4. Error paths

use mccarthy_lisp_lexer::{tokenize, LexError, Loc, Token};

fn toks(src: &str) -> Vec<Token> {
    tokenize(src).expect("tokenize").into_iter().map(|t| t.tok).collect()
}

// ============================================================
// 1. Single-token shapes
// ============================================================

#[test]
fn lparen_alone() {
    assert_eq!(toks("("), vec![Token::LParen]);
}

#[test]
fn rparen_alone() {
    assert_eq!(toks(")"), vec![Token::RParen]);
}

#[test]
fn quote_alone() {
    assert_eq!(toks("'"), vec![Token::Quote]);
}

#[test]
fn dot_alone() {
    assert_eq!(toks("."), vec![Token::Dot]);
}

#[test]
fn single_symbol_uppercase() {
    assert_eq!(toks("CAR"), vec![Token::Symbol("CAR".into())]);
}

#[test]
fn symbol_with_digit_and_hyphen() {
    assert_eq!(toks("X-1"), vec![Token::Symbol("X-1".into())]);
}

#[test]
fn single_int_zero() {
    assert_eq!(toks("0"), vec![Token::Int(0)]);
}

#[test]
fn single_int_positive() {
    assert_eq!(toks("42"), vec![Token::Int(42)]);
}

#[test]
fn single_int_negative() {
    assert_eq!(toks("-42"), vec![Token::Int(-42)]);
}

#[test]
fn single_int_large() {
    // McCarthy didn't bound integer size; we cap at i64::MAX.
    assert_eq!(toks("9223372036854775807"), vec![Token::Int(i64::MAX)]);
}

// ============================================================
// 2. Canonical McCarthy 1960 paper expressions
// ============================================================

#[test]
fn car_of_quoted_list() {
    // (CAR '(A B C)) — McCarthy 1960, §3 example 1.
    assert_eq!(
        toks("(CAR '(A B C))"),
        vec![
            Token::LParen,
            Token::Symbol("CAR".into()),
            Token::Quote,
            Token::LParen,
            Token::Symbol("A".into()),
            Token::Symbol("B".into()),
            Token::Symbol("C".into()),
            Token::RParen,
            Token::RParen,
        ]
    );
}

#[test]
fn identity_lambda() {
    // (LAMBDA (X) X) — McCarthy 1960, §5.
    assert_eq!(
        toks("(LAMBDA (X) X)"),
        vec![
            Token::LParen,
            Token::Symbol("LAMBDA".into()),
            Token::LParen,
            Token::Symbol("X".into()),
            Token::RParen,
            Token::Symbol("X".into()),
            Token::RParen,
        ]
    );
}

#[test]
fn label_named_recursion() {
    // (LABEL FF (LAMBDA (X) (COND ((ATOM X) X) (T (FF (CAR X)))))) — McCarthy 1960, §6.
    let toks = toks("(LABEL FF (LAMBDA (X) (COND ((ATOM X) X) (T (FF (CAR X))))))");
    // 30 tokens — confirm the count rather than re-write the full vec.
    assert_eq!(toks.len(), 30);
    assert_eq!(toks[0], Token::LParen);
    assert_eq!(toks[1], Token::Symbol("LABEL".into()));
    assert_eq!(toks[2], Token::Symbol("FF".into()));
}

#[test]
fn cons_of_two_atoms() {
    assert_eq!(
        toks("(CONS 'A 'B)"),
        vec![
            Token::LParen,
            Token::Symbol("CONS".into()),
            Token::Quote,
            Token::Symbol("A".into()),
            Token::Quote,
            Token::Symbol("B".into()),
            Token::RParen,
        ]
    );
}

#[test]
fn dotted_pair_literal() {
    assert_eq!(
        toks("(A . B)"),
        vec![
            Token::LParen,
            Token::Symbol("A".into()),
            Token::Dot,
            Token::Symbol("B".into()),
            Token::RParen,
        ]
    );
}

// ============================================================
// 3. Whitespace + comments
// ============================================================

#[test]
fn empty_source_yields_nothing() {
    assert_eq!(toks(""), Vec::<Token>::new());
}

#[test]
fn whitespace_only() {
    assert_eq!(toks("   \t\n  \r\n  "), Vec::<Token>::new());
}

#[test]
fn comment_to_end_of_line() {
    assert_eq!(
        toks("; this is a comment\nCAR"),
        vec![Token::Symbol("CAR".into())]
    );
}

#[test]
fn comment_at_eof_no_newline() {
    assert_eq!(toks("X ; trailing"), vec![Token::Symbol("X".into())]);
}

// ============================================================
// 4. Locations
// ============================================================

#[test]
fn first_token_starts_at_1_1() {
    let toks = tokenize("(CAR X)").expect("tokenize");
    assert_eq!(toks[0].loc, Loc { line: 1, column: 1 });
    assert_eq!(toks[1].loc, Loc { line: 1, column: 2 });
}

#[test]
fn newline_advances_line() {
    let toks = tokenize("(\nCAR)").expect("tokenize");
    assert_eq!(toks[0].loc, Loc { line: 1, column: 1 });
    assert_eq!(toks[1].loc, Loc { line: 2, column: 1 });
}

// ============================================================
// 5. Error paths
// ============================================================

#[test]
fn lone_minus_is_lex_error() {
    let err = tokenize("(- X)").expect_err("`-` is not a valid Lisp 1.0 token");
    assert!(matches!(err, LexError::LoneMinus { .. }));
}

#[test]
fn lowercase_is_lex_error() {
    let err = tokenize("car").expect_err("lowercase not allowed");
    assert!(matches!(err, LexError::LowercaseInSymbol { .. }));
}

#[test]
fn unknown_byte_is_lex_error() {
    // `+` is an operator symbol — Lisp 1.0 has none.
    let err = tokenize("+").expect_err("`+` is not a valid Lisp 1.0 token");
    assert!(matches!(err, LexError::InvalidByte { .. }));
}

#[test]
fn integer_overflow_is_caught() {
    // i64::MAX + 1
    let err = tokenize("9223372036854775808").expect_err("overflow");
    assert!(matches!(err, LexError::IntegerOverflow { .. }));
}

#[test]
fn error_display_includes_location() {
    let err = tokenize("(- X)").expect_err("lone minus");
    let msg = format!("{err}");
    assert!(msg.contains("1:2"), "expected 1:2 in {msg}");
    assert!(msg.contains("not a valid Lisp 1.0 token"), "{msg}");
}
