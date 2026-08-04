//! End-to-end tests for ADJ-STATEMACHINE RS-3b — the native `statemachine`
//! construct's grammar + AST + adapter + lowering — driven through the built CLI
//! binary. RS-3b lowers the STRUCTURE only (no driver, RS-3c); so the well-formed
//! case need only COMPILE clean, and each malformed case must yield its SPECIFIC
//! typed diagnostic (a `LowerError`, or — where the grammar rejects the shape
//! before lowering — a clean parse error), never a panic or a message-less exit.
//!
//! Five well-formedness gates from ADJ-STATEMACHINE §2 are exercised:
//!
//! - `SmUnknownState` — a transition `to` a state that isn't declared.
//! - `SmBudgetNotPositive` — `budget 0 steps`.
//! - `SmMissingProvenance` — a machine with no `source` (the shared write gate).
//! - `SmMissingExit` — no `exit when … yield …` (the grammar admits zero exits, so
//!   the lowerer is what rejects it).
//! - initial-state omitted — the grammar REQUIRES `initial IDENT`, so omitting it is
//!   a clean PARSE error, not `SmMissingInitial` (which is a defensive-only lower
//!   check; see the deviation note in that test). ADJ-STATEMACHINE §2's
//!   `SmMissingInitial` is therefore unreachable from the surface — the grammar
//!   enforces the same invariant earlier and more strictly.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs3b_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the CLI, returning (exit-ok, stdout, stderr) so the error-path tests can
/// assert on the diagnostic regardless of which stream it lands on.
fn run_full(program: &Path) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn write(dir: &Path, name: &str, src: &str) {
    std::fs::write(dir.join(name), src).unwrap();
}

// ---------------------------------------------------------------------------
// (a) A well-formed machine compiles clean — parse → adapt → lower with NO error.
// ---------------------------------------------------------------------------

#[test]
fn wellformed_statemachine_compiles_without_error() {
    let dir = scratch("ok");
    // The §6.1 titrate-to-target shape: an initial state with two comparison
    // guards, two do-action states (`assert`), a comparison exit, and a budget.
    write(
        dir.as_path(),
        "case.adj",
        "statemachine warfarin_titration {\n\
         \x20   initial check\n\
         \x20   state check {\n\
         \x20       transition on inr < 2 to increase_dose\n\
         \x20       transition on inr > 3 to decrease_dose\n\
         \x20   }\n\
         \x20   state increase_dose { transition on dose_low to check do assert dose_changed }\n\
         \x20   state decrease_dose { transition on dose_high to check do assert dose_changed }\n\
         \x20   exit when inr >= 2 yield at_target\n\
         \x20   budget 20 steps\n\
         \x20   source \"warfarin dosing protocol (worked example)\"\n\
         \x20   trust authoritative\n\
         }\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "well-formed statemachine must compile: {out}{err}");
    // It need not RUN (the driver is RS-3c) — it must just lower without error.
    assert!(
        !out.contains("\"error\""),
        "no compile error in the output: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b) Unknown transition target — a `to` a state that was never declared.
// ---------------------------------------------------------------------------

#[test]
fn transition_to_undeclared_state_is_sm_unknown_state() {
    let dir = scratch("unknown");
    write(
        dir.as_path(),
        "case.adj",
        "statemachine m {\n\
         \x20   initial a\n\
         \x20   state a { transition on go to nowhere }\n\
         \x20   exit when done yield ok\n\
         \x20   budget 5 steps\n\
         \x20   source \"loop (worked example)\"\n\
         \x20   trust inferred\n\
         }\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "unknown target must fail: {out}{err}");
    let combined = format!("{out}{err}");
    assert!(
        combined.contains("SmUnknownState") && combined.contains("nowhere"),
        "diagnostic names the unknown target state: {combined}"
    );
}

// ---------------------------------------------------------------------------
// (c) Non-positive budget — `budget 0 steps` (the termination guarantee).
// ---------------------------------------------------------------------------

#[test]
fn zero_budget_is_sm_budget_not_positive() {
    let dir = scratch("budget0");
    write(
        dir.as_path(),
        "case.adj",
        "statemachine m {\n\
         \x20   initial a\n\
         \x20   state a { transition on go to a }\n\
         \x20   exit when done yield ok\n\
         \x20   budget 0 steps\n\
         \x20   source \"loop (worked example)\"\n\
         \x20   trust inferred\n\
         }\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "zero budget must fail: {out}{err}");
    let combined = format!("{out}{err}");
    assert!(
        combined.contains("SmBudgetNotPositive"),
        "diagnostic names the non-positive budget: {combined}"
    );
}

// ---------------------------------------------------------------------------
// (d) Missing provenance — a machine with no `source` (shared write gate).
// ---------------------------------------------------------------------------

#[test]
fn unsourced_statemachine_is_sm_missing_provenance() {
    let dir = scratch("nosrc");
    write(
        dir.as_path(),
        "case.adj",
        "statemachine m {\n\
         \x20   initial a\n\
         \x20   state a { transition on go to a }\n\
         \x20   exit when done yield ok\n\
         \x20   budget 5 steps\n\
         \x20   trust inferred\n\
         }\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "unsourced machine must fail: {out}{err}");
    let combined = format!("{out}{err}");
    assert!(
        combined.contains("SmMissingProvenance")
            || combined.to_lowercase().contains("provenance")
            || combined.to_lowercase().contains("source"),
        "diagnostic names the missing provenance: {combined}"
    );
}

// ---------------------------------------------------------------------------
// (e) Missing exit — the grammar admits zero exits (`{ sm_exit }`), so the
//     LOWERER is what rejects a machine that can never halt on a criterion.
// ---------------------------------------------------------------------------

#[test]
fn statemachine_without_exit_is_sm_missing_exit() {
    let dir = scratch("noexit");
    write(
        dir.as_path(),
        "case.adj",
        "statemachine m {\n\
         \x20   initial a\n\
         \x20   state a { transition on go to a }\n\
         \x20   budget 5 steps\n\
         \x20   source \"loop (worked example)\"\n\
         \x20   trust inferred\n\
         }\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "machine with no exit must fail: {out}{err}");
    let combined = format!("{out}{err}");
    assert!(
        combined.contains("SmMissingExit"),
        "diagnostic names the missing exit: {combined}"
    );
}

// ---------------------------------------------------------------------------
// (f) Missing `initial` — DEVIATION from a pure LowerError expectation.
//
//     ADJ-STATEMACHINE §2 lists `SmMissingInitial` as a well-formedness error,
//     but the normative grammar makes `"initial" IDENT` a REQUIRED clause of
//     `statemachine_decl` (exactly one, before the states). So a machine that
//     omits `initial` is rejected at PARSE time, never reaching the lowerer's
//     `SmMissingInitial` check — that variant is retained only as a defensive
//     guard against a degenerate/empty initial name. We assert the clean,
//     non-panicking PARSE error here, which is the stronger, earlier enforcement
//     of the same invariant.
// ---------------------------------------------------------------------------

#[test]
fn statemachine_without_initial_is_a_clean_parse_error() {
    let dir = scratch("noinit");
    write(
        dir.as_path(),
        "case.adj",
        "statemachine m {\n\
         \x20   state a { transition on go to a }\n\
         \x20   exit when done yield ok\n\
         \x20   budget 5 steps\n\
         \x20   source \"loop (worked example)\"\n\
         \x20   trust inferred\n\
         }\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "machine with no initial must fail: {out}{err}");
    let combined = format!("{out}{err}");
    // A clean, message-carrying error (the grammar expected `initial`), not a panic.
    assert!(
        combined.contains("\"error\"")
            && (combined.contains("initial") || combined.to_lowercase().contains("parse")),
        "clean parse error naming the missing `initial`: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("panic"),
        "must not panic: {combined}"
    );
}
