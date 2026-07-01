//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest.testUnicode`-style output under the
//! **`--output_charset=US-ASCII`** setting — the mode where every
//! non-ASCII code point is escaped to a `\uXXXX` sequence so the
//! output is pure ASCII. Our emitter models this as
//! `EmitOptions { ascii_only: true }`, handled by `escape_ascii_only`.
//! The sibling `code_printer_string_escape_test.rs` pins the DEFAULT
//! (non-ASCII-passthrough) mode; this file pins the ASCII-only path,
//! which is a distinct branch of `emit_string`.
//!
//! ## How the emitter escapes under `ascii_only` (recap)
//!
//! ```text
//!   \  → \\        "  → \"        \n → \n    \r → \r    \t → \t
//!   control (< U+0020)           → \uXXXX        (upper-case, 4 digits)
//!   printable ASCII (0x20-0x7E)  → verbatim
//!   non-ASCII, U+0080..=U+FFFF   → \uXXXX        (upper-case, 4 digits)
//!   non-ASCII, > U+FFFF (astral) → \u{XXXXXX}    (upper-case, braces)
//! ```
//!
//! `ascii_only` ALWAYS wraps in double quotes (unlike the default
//! mode's quote-choice optimisation), escaping any inner `"`.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    Expression, ExpressionStatement, Program, ProgramItem, SourceType, Statement, StringLiteral,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn string(v: &str) -> Expression {
    Expression::StringLiteral(StringLiteral {
        cv: None,
        value: v.to_string(),
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

/// Emit a program with `ascii_only` turned on.
fn emit_ascii(prog: Program) -> String {
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let opts = EmitOptions {
        ascii_only: true,
        ..EmitOptions::default()
    };
    emit(&prog, &sidecar, &mut cv, &opts)
        .expect("emit failed")
        .code
}

/// Emit `expr` as a single statement under `ascii_only` and assert the
/// resulting code equals `expected_emitted`.
fn assert_ascii(expr: Expression, expected_emitted: &str) {
    let code = emit_ascii(program_with(stmt(expr)));
    assert_eq!(
        code, expected_emitted,
        "ascii_only emit output did not match expected\n  actual:   {:?}\n  expected: {:?}",
        code, expected_emitted
    );
}

// =====================================================================
// Active — ascii_only escaping
// =====================================================================

/// Printable ASCII passes through untouched.
#[test]
fn printable_ascii_stays_verbatim() {
    assert_ascii(string("abc XYZ 123 !~"), "\"abc XYZ 123 !~\";");
}

/// A Latin-1 accented letter (`U+00E9 é`) becomes `é` — upper-case,
/// four hex digits.
#[test]
fn latin1_non_ascii_escapes_to_u_hex() {
    assert_ascii(string("caf\u{e9}"), "\"caf\\u00E9\";");
}

/// A BMP CJK ideograph (`U+4E2D 中`) becomes `中`.
#[test]
fn bmp_non_ascii_escapes_to_u_hex() {
    assert_ascii(string("\u{4e2d}"), "\"\\u4E2D\";");
}

/// An astral-plane code point (`U+1F4A9 💩`, above U+FFFF) uses the
/// braced `\u{XXXXXX}` form rather than a surrogate pair.
#[test]
fn astral_code_point_uses_braced_escape() {
    assert_ascii(string("\u{1f4a9}"), "\"\\u{1F4A9}\";");
}

/// Control characters still take the `\uXXXX` form (`U+0007` → ``).
#[test]
fn control_char_escapes_to_u_hex() {
    assert_ascii(string("a\u{07}b"), "\"a\\u0007b\";");
}

/// The named short escapes still apply under `ascii_only`.
#[test]
fn named_escapes_still_apply() {
    assert_ascii(string("a\nb\tc"), "\"a\\nb\\tc\";");
}

/// Unlike the default mode's quote-choice, `ascii_only` ALWAYS uses
/// double quotes and escapes any inner `"` — a value of two double
/// quotes prints as `"\"\""`, not single-quoted.
#[test]
fn ascii_only_always_double_quotes() {
    assert_ascii(string("\"\""), "\"\\\"\\\"\";");
}
