//! End-to-end tests for the NUM-6a `round_to(x, n)` precision narrowing, driven
//! through the built `adj-lang-cli` binary. They prove the whole path — native
//! application surface → adapter/lower → the engine's exact rounding → the audit
//! JSON — works together: the value is rounded **exactly** (`1/3 → 33/100`, no
//! `f64` tie-break), the audit exposes the narrowing (`node:round`, `places`,
//! `mode:half_even`, the operand subtree), and a bad precision is a clean compile
//! error rather than a silent mis-rounding (ADJ-NUMERIC-SUBSTRATE §4.1–§4.4).

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_num6a_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(src: &str, tag: &str) -> (bool, String) {
    let dir = scratch(tag);
    let p = dir.join("case.adj");
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(&p)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn round_to_computes_exactly_and_audits_the_narrowing() {
    // 10/3 = 3.333… → 2 decimal places (half-even) = 3.33, held as the EXACT
    // fraction 333/100 (not a lossy 0.3300000004).
    let (ok, s) = run("let r = round_to(10 / 3, 2)\n? r\n", "value");
    assert!(ok, "cli should succeed: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(s.contains("\"value\":3.33"), "rounds to 3.33: {s}");
    // The exact sidecar is the true fraction, so an auditor re-derives it precisely.
    assert!(
        s.contains("\"num\":\"333\"") && s.contains("\"den\":\"100\""),
        "carries the exact rounded fraction 333/100: {s}"
    );
    // The audit trail exposes the narrowing as a first-class, checkable step:
    // the node type, the precision, the stated mode, and the operand subtree it
    // rounded (the 10/3 division) — everything `adj-verify` needs to re-round.
    assert!(
        s.contains("\"node\":\"round\"")
            && s.contains("\"places\":2")
            && s.contains("\"mode\":\"half_even\""),
        "audit records node/places/mode: {s}"
    );
    assert!(
        s.contains("\"operand\":{\"node\":\"op\",\"op\":\"/\""),
        "the narrowing's operand subtree is the exact source division: {s}"
    );
}

#[test]
fn round_to_default_mode_breaks_ties_to_even() {
    // 5/2 = 2.5 → 0 places. Half-even (the default) rounds the tie to the EVEN
    // neighbour, 2 — NOT 3, which the legacy ties-away integer round would give.
    let (ok, s) = run("let r = round_to(5 / 2, 0)\n? r\n", "tie");
    assert!(ok, "cli should succeed: {s}");
    assert!(
        s.contains("\"value\":2") && !s.contains("\"value\":3"),
        "2.5 rounds half-even to 2, not 3: {s}"
    );
}

#[test]
fn a_non_integer_precision_is_a_compile_error_not_a_silent_rounding() {
    // The precision must be a non-negative integer literal; `2.5` is rejected at
    // compile time rather than silently truncated to some rounding.
    let (ok, s) = run("let r = round_to(10 / 3, 2.5)\n? r\n", "badprec");
    assert!(
        !ok || s.contains("\"error\""),
        "a non-integer precision must be rejected: {s}"
    );
    assert!(
        !s.contains("\"value\":3.3"),
        "must not emit a silently-rounded value: {s}"
    );
}

#[test]
fn a_negative_precision_is_a_compile_error() {
    let (ok, s) = run("let r = round_to(10 / 3, -1)\n? r\n", "negprec");
    assert!(
        !ok || s.contains("\"error\""),
        "a negative precision must be rejected: {s}"
    );
}
