//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! `CodePrinterTest.java` has a large family of number-printing
//! assertions (`testNumericKeys`, `testExponents`, the shortest-form
//! and negative-zero cases threaded through `assertPrint`). The
//! sibling `code_printer_test.rs` already actives the *core*
//! shortest-form cases (`1E9`, `1E6`, `1E21`, the `100` tie, `0.5`,
//! `-0`) under CLOC12.138. This file extends that with the
//! surrounding cut-over cases — where the emitter must choose between
//! the plain-decimal spelling and the uppercase-`E` exponential
//! spelling and pick the byte-shorter one (ties → decimal).
//!
//! ## How the emitter decides (recap of `format_js_number`)
//!
//! For a finite, non-zero value it forms two candidate spellings and
//! keeps whichever is **strictly shorter**, breaking ties toward the
//! decimal form:
//!
//! ```text
//!   decimal  =  (|n| < 2^63 && n integral)  ?  i64 spelling  :  f64 shortest-decimal
//!   expo     =  Rust {:e}  with the `e` upper-cased  (no `+` on positive exponents)
//!   result   =  expo.len() < decimal.len()  ?  expo  :  decimal
//! ```
//!
//! So the exponential form wins exactly when it saves at least one
//! byte. A large power of ten (`1e18`, `1e100`) collapses; a nine-digit
//! integer (`123456789`) stays decimal because its exponential
//! (`1.23456789E8`) is longer.
//!
//! ## Divergence pinned by this file (gap-133)
//!
//! Upstream Closure drops the redundant leading zero on a bare
//! fraction: `0.25` prints as `.25`. Our emitter's `format_js_number`
//! keeps the `0`, so those cases are `#[ignore = "blocked on gap-133"]`
//! placeholders documenting the intended upstream byte output. See
//! `code/specs/CLOC12-gaps.md` §CLOC12.138 (gap-133). Note this is the
//! *emitter* path (`format_js_number`, AST → string); the separate
//! source-preserving byte-identity path already elides the leading zero
//! (gap-107 / gap-113).

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    Expression, ExpressionStatement, NumericLiteral, Program, ProgramItem, SourceType, Statement,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers (mirrors `code_printer_test.rs`)
// =====================================================================

fn num(v: f64) -> Expression {
    Expression::NumericLiteral(NumericLiteral {
        cv: None,
        value: v,
        raw: if v.fract() == 0.0 && v.is_finite() {
            format!("{}", v as i64)
        } else {
            v.to_string()
        },
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
// Active — exponential-vs-decimal cut-over (the shorter spelling wins)
// =====================================================================

/// A large integral power of ten collapses to the exponential form:
/// the decimal `1000000000000000000` (19 bytes) loses to `1E18` (4).
/// `1e18` is still `< 2^63`, so the decimal candidate takes the exact
/// i64 spelling before losing on length.
#[test]
fn exponential_wins_for_large_power_of_ten_within_i64() {
    assert_emits(num(1e18), "1E18;");
}

/// Beyond `2^63` the decimal candidate falls back to the f64
/// shortest-decimal (a 101-digit positional string for `1e100`, since
/// Rust's `f64` `Display` never itself switches to scientific), which
/// the 5-byte `1E100` trivially beats.
#[test]
fn exponential_wins_far_beyond_i64_range() {
    assert_emits(num(1e100), "1E100;");
}

/// A round multiplier keeps its mantissa in the exponential form:
/// `25000000000` (11 bytes) loses to `2.5E10` (6).
#[test]
fn exponential_keeps_mantissa_when_shorter() {
    assert_emits(num(2.5e10), "2.5E10;");
}

/// A nine-digit integer stays decimal: its exponential spelling
/// `1.23456789E8` (12 bytes) is *longer* than `123456789` (9), so the
/// tie-break-toward-decimal rule is not even reached — decimal is
/// strictly shorter.
#[test]
fn nine_digit_integer_stays_decimal() {
    assert_emits(num(123456789.0), "123456789;");
}

/// A small fraction with a negative exponent flips to exponential:
/// the positional decimal `0.0000001` (9 bytes) loses to `1E-7` (4).
/// Note the exponential form carries no `+` and uses an uppercase `E`.
#[test]
fn tiny_fraction_flips_to_negative_exponential() {
    assert_emits(num(1e-7), "1E-7;");
}

/// A mixed fraction whose exponential spelling is *longer* stays
/// decimal: `1234.5` (6 bytes) beats `1.2345E3` (8).
#[test]
fn mixed_fraction_stays_decimal_when_exponential_is_longer() {
    assert_emits(num(1234.5), "1234.5;");
}

// =====================================================================
// Ignored — upstream leading-zero drop (gap-133)
// =====================================================================

/// Upstream `assertPrint("0.25", ".25")` — Closure drops the redundant
/// leading zero on a bare fraction. Our emitter's `format_js_number`
/// currently keeps it (`0.25`), so this pins the intended upstream byte
/// output until gap-133 closes.
#[test]
#[ignore = "blocked on gap-133: format_js_number keeps the leading zero (0.25 vs .25)"]
fn leading_zero_dropped_on_bare_fraction() {
    assert_emits(num(0.25), ".25;");
}

/// Same divergence, a longer fraction: upstream `0.125` → `.125`.
#[test]
#[ignore = "blocked on gap-133: format_js_number keeps the leading zero (0.125 vs .125)"]
fn leading_zero_dropped_on_longer_fraction() {
    assert_emits(num(0.125), ".125;");
}
