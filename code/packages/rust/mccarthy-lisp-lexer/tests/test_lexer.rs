//! Integration tests for `mccarthy-lisp-lexer`.
//!
//! Since this crate is a thin wrapper over the shared, separately-tested
//! `GrammarLexer`, these tests focus on the *McCarthy dialect contract*
//! that lives in `code/grammars/mccarthy_lisp.tokens`:
//!
//!   1. Each token kind is recognised with the right name + value.
//!   2. The canonical McCarthy 1960 paper expressions tokenize cleanly.
//!   3. Whitespace and `;` comments are skipped.
//!   4. The dialect restrictions (no lowercase, no operator symbols)
//!      surface as lex errors.

use mccarthy_lisp_lexer::{tokenize_mccarthy, TokenType};

/// Token (name, value) pairs, excluding the trailing EOF.
fn pairs(src: &str) -> Vec<(String, String)> {
    tokenize_mccarthy(src)
        .expect("tokenize")
        .into_iter()
        .filter(|t| t.type_ != TokenType::Eof)
        .map(|t| (t.effective_type_name().to_string(), t.value))
        .collect()
}

/// Just the token names, excluding EOF.
fn names(src: &str) -> Vec<String> {
    pairs(src).into_iter().map(|(n, _)| n).collect()
}

// ============================================================
// 1. Single-token shapes
// ============================================================

#[test]
fn single_tokens() {
    assert_eq!(names("("), vec!["LPAREN"]);
    assert_eq!(names(")"), vec!["RPAREN"]);
    assert_eq!(names("'"), vec!["QUOTE"]);
    assert_eq!(names("."), vec!["DOT"]);
    assert_eq!(names("CAR"), vec!["SYMBOL"]);
    assert_eq!(names("42"), vec!["INTEGER"]);
}

#[test]
fn integer_values_round_trip() {
    assert_eq!(pairs("0"), vec![("INTEGER".into(), "0".into())]);
    assert_eq!(pairs("42"), vec![("INTEGER".into(), "42".into())]);
    assert_eq!(pairs("-1"), vec![("INTEGER".into(), "-1".into())]);
}

#[test]
fn symbol_values_round_trip() {
    assert_eq!(pairs("CAR"), vec![("SYMBOL".into(), "CAR".into())]);
    assert_eq!(pairs("FF"), vec![("SYMBOL".into(), "FF".into())]);
    assert_eq!(pairs("LIST-OF-3"), vec![("SYMBOL".into(), "LIST-OF-3".into())]);
    // Digits are allowed inside (but not at the start of) a symbol.
    assert_eq!(pairs("A1"), vec![("SYMBOL".into(), "A1".into())]);
}

// ============================================================
// 2. Canonical McCarthy 1960 paper expressions
// ============================================================

#[test]
fn car_of_a_literal_list() {
    assert_eq!(
        names("(CAR '(A B C))"),
        vec!["LPAREN", "SYMBOL", "QUOTE", "LPAREN", "SYMBOL", "SYMBOL", "SYMBOL", "RPAREN", "RPAREN"]
    );
}

#[test]
fn the_identity_lambda() {
    assert_eq!(
        names("(LAMBDA (X) X)"),
        vec!["LPAREN", "SYMBOL", "LPAREN", "SYMBOL", "RPAREN", "SYMBOL", "RPAREN"]
    );
}

#[test]
fn a_dotted_pair() {
    assert_eq!(names("(A . B)"), vec!["LPAREN", "SYMBOL", "DOT", "SYMBOL", "RPAREN"]);
}

#[test]
fn the_label_recursive_definition_header() {
    // (LABEL FF (LAMBDA (X) ...)) — just the token shape of the header.
    assert_eq!(
        names("(LABEL FF (LAMBDA (X) X))"),
        vec![
            "LPAREN", "SYMBOL", "SYMBOL", "LPAREN", "SYMBOL", "LPAREN", "SYMBOL", "RPAREN",
            "SYMBOL", "RPAREN", "RPAREN"
        ]
    );
}

// ============================================================
// 3. Whitespace + comments
// ============================================================

#[test]
fn comments_are_skipped() {
    assert_eq!(names("CAR ; comment\nCDR"), vec!["SYMBOL", "SYMBOL"]);
}

#[test]
fn newlines_and_tabs_are_skipped() {
    assert_eq!(names("\t( A\n\tB )"), vec!["LPAREN", "SYMBOL", "SYMBOL", "RPAREN"]);
}

#[test]
fn only_whitespace_and_comments_is_empty() {
    assert!(names("  \n ; nothing here\n").is_empty());
}

// ============================================================
// 4. Dialect-restriction error paths
// ============================================================

#[test]
fn lowercase_symbol_rejected() {
    assert!(tokenize_mccarthy("car").is_err());
    assert!(tokenize_mccarthy("(CAR x)").is_err());
}

#[test]
fn bare_operator_symbols_rejected() {
    // Lisp 1.0 has no operator symbols — these all match no token rule.
    assert!(tokenize_mccarthy("+").is_err());
    assert!(tokenize_mccarthy("-").is_err());
    assert!(tokenize_mccarthy("<=").is_err());
}

#[test]
fn strings_are_rejected() {
    // No string literals in Lisp 1.0 — the `"` matches nothing.
    assert!(tokenize_mccarthy("\"hello\"").is_err());
}
