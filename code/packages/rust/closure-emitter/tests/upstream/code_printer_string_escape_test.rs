//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! `CodePrinterTest.java`'s string-escaping family (`testEscape`,
//! `testUnicode`, the `\n`/`\t`/`\\` cases threaded through
//! `assertPrint`). The sibling `code_printer_test.rs` already actives
//! the *quote-choice* case (`she said "hi"` → single quotes to avoid
//! escaping the inner `"`, gap-026). This file pins the **escape
//! sequences themselves** for both the default double-quote path and
//! the single-quote path that quote-choice selects.
//!
//! ## How the emitter escapes (recap of `choose_quote_and_escape`)
//!
//! The emitter counts `"` vs `'` in the value and wraps in whichever
//! needs fewer escapes (tie → double). Inside the chosen quotes it
//! escapes, in both paths:
//!
//! ```text
//!   \  → \\        \n → \n        \r → \r        \t → \t
//!   U+2028 →              U+2029 →
//!   any other control (< U+0020) → \uXXXX   (upper-case, 4 hex digits)
//! ```
//!
//! plus the *active* quote character (`"` in the double path, `'` in
//! the single path). Everything else — including printable non-ASCII —
//! is emitted verbatim (this is the default, non-`ascii_only`, mode).

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    Expression, ExpressionStatement, Program, ProgramItem, SourceType, Statement, StringLiteral,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers (mirrors `code_printer_test.rs`)
// =====================================================================

fn string(v: &str) -> Expression {
    Expression::StringLiteral(StringLiteral {
        cv: None,
        value: v.to_string(),
        // `raw` is ignored by the emitter for string literals — it re-derives
        // the shortest correctly-escaped spelling from `value` — but we fill it
        // for shape parity with the other ports' `string()` helper.
        raw: format!("\"{}\"", v),
    })
}

fn stmt(expr: Expression) -> ProgramItem {
    ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
        cv: None,
        expression: expr,
    }))
}

fn program_with(item: ProgramItem) -> Program {
    Program::new_untraced(EsVersion::Es2025, SourceType::Module).with_body(vec![item])
}

fn emit_default(prog: Program) -> String {
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
        .expect("emit failed")
        .code
}

/// Upstream `assertPrint(input, expected)` reshaped for our AST surface:
/// emit `expr` as the single statement of a program and assert the
/// resulting code equals `expected_emitted`.
fn assert_emits(expr: Expression, expected_emitted: &str) {
    let code = emit_default(program_with(stmt(expr)));
    assert_eq!(
        code, expected_emitted,
        "emit output did not match expected\n  actual:   {:?}\n  expected: {:?}",
        code, expected_emitted
    );
}

// =====================================================================
// Active — double-quote path escapes (default; tie → double quotes)
// =====================================================================

/// A literal backslash doubles: `a\b` → `"a\\b"`.
#[test]
fn escapes_backslash() {
    assert_emits(string("a\\b"), "\"a\\\\b\";");
}

/// Newline / carriage-return / tab collapse to their short escapes.
#[test]
fn escapes_newline_return_tab() {
    assert_emits(string("a\nb"), "\"a\\nb\";");
    assert_emits(string("a\rb"), "\"a\\rb\";");
    assert_emits(string("a\tb"), "\"a\\tb\";");
}

/// An "other" control character (below U+0020, not one of the named
/// escapes) becomes a 4-hex-digit `\uXXXX` with upper-case hex — here
/// U+0007 BELL → ``.
#[test]
fn escapes_other_control_char_as_u_hex() {
    assert_emits(string("a\u{07}b"), "\"a\\u0007b\";");
}

/// The ECMAScript line terminators U+2028 / U+2029 are escaped even
/// though they sit above U+0020 — an unescaped one is a pre-ES2019
/// `SyntaxError` inside a string literal.
#[test]
fn escapes_line_and_paragraph_separators() {
    assert_emits(string("a\u{2028}b"), "\"a\\u2028b\";");
    assert_emits(string("a\u{2029}b"), "\"a\\u2029b\";");
}

/// A printable non-ASCII codepoint is emitted VERBATIM in the default
/// (non-`ascii_only`) mode — `café` stays `café`, not `café`.
#[test]
fn printable_non_ascii_stays_verbatim() {
    assert_emits(string("caf\u{e9}"), "\"caf\u{e9}\";");
}

// =====================================================================
// Active — single-quote path (quote-choice) escapes
// =====================================================================

/// When the value has more `"` than `'`, the emitter switches to
/// single quotes (fewer escapes). The backslash rule is unchanged
/// across quote styles, so `""\` (two double quotes, one backslash)
/// prints single-quoted with the `"` left bare and the `\` doubled:
/// `'""\\'`.
#[test]
fn single_quote_path_still_escapes_backslash() {
    assert_emits(string("\"\"\\"), "'\"\"\\\\';");
}

/// In the single-quote path the active quote `'` is the one that gets
/// escaped, while `"` is left bare. A value with two `"` and one `'`
/// picks single quotes and escapes only the `'`: `'"\'"'`.
#[test]
fn single_quote_path_escapes_the_single_quote() {
    assert_emits(string("\"'\""), "'\"\\'\"';");
}
