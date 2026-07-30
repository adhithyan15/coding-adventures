//! End-to-end tests for ADJ-STATEMACHINE RS-3c — the `statemachine` **driver**:
//! the deterministic, total loop that RUNS the machines RS-3b lowered, its four
//! typed outcomes (§4), cycle detection (§3.1), the CLI `state_machines` JSON
//! section, and the `--explain` rendering of the run. Driven through the built CLI
//! binary, mirroring the rs3b lowering harness and the rs4e `--explain` harness.
//!
//! The invariants pinned here are the ones that make a run trustworthy:
//!
//! - **(a) terminating** — the §6.1 titrate machine with `observe inr(2.5)` makes
//!   the exit guard `inr >= 2` hold; the outcome is `Halted`, yielding `at_target`,
//!   and `--explain` says `=> Halted`.
//! - **(b) non-terminating / budget-bounded** — the §6.2 `a↔b` spin with no
//!   exit-enabling fact returns `NonTerminating` (cycle) OR `StepBudgetExceeded`,
//!   and — critically — the process COMPLETES (the guard bounds the loop; a hang
//!   would fail the test by never returning).
//! - **(c) stuck** — a machine whose only transition guard is unsatisfiable and
//!   whose exit never holds returns `Stuck`.
//! - **(d) determinism** — the `--explain` render is byte-identical across two runs
//!   (ADJ-REASON-MATH §E.8 P4).

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs3c_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the CLI with optional leading args; returns (success, stdout, stderr).
fn run_with(args: &[&str], program: &Path) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .args(args)
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn run_json(program: &Path) -> String {
    let (ok, out, err) = run_with(&[], program);
    assert!(ok, "CLI exited non-zero: {out}{err}");
    out
}

fn run_explain(program: &Path) -> String {
    let (ok, out, err) = run_with(&["--explain"], program);
    assert!(ok, "--explain exited non-zero: {err}");
    out
}

fn write(dir: &Path, name: &str, src: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p
}

/// The §6.1 titrate-to-target machine: a `check` state with two comparison guards,
/// two `do assert` states, a comparison exit `inr >= 2`, and a step budget.
const TITRATE: &str = "statemachine warfarin_titration {\n\
     \x20   initial check\n\
     \x20   state check {\n\
     \x20       transition on inr < 2 to increase_dose\n\
     \x20       transition on inr > 3 to decrease_dose\n\
     \x20   }\n\
     \x20   state increase_dose { transition on true to check do assert dose_changed }\n\
     \x20   state decrease_dose { transition on true to check do assert dose_changed }\n\
     \x20   exit when inr >= 2 yield at_target\n\
     \x20   budget 20 steps\n\
     \x20   source \"warfarin dosing protocol (worked example)\"\n\
     \x20   trust authoritative\n\
     }\n";

// ---------------------------------------------------------------------------
// (a) TERMINATING — the exit guard holds, the machine Halts with its yield.
// ---------------------------------------------------------------------------

#[test]
fn terminating_run_halts_with_the_yielded_result() {
    let dir = scratch("halt");
    // observe inr(2.5): both `check` guards fail (2.5 is neither < 2 nor > 3), and
    // the exit `inr >= 2` holds, so the machine halts in `check` in 0 transitions.
    let prog = write(dir.as_path(), "case.adj", &format!("{TITRATE}observe inr(2.5)\n"));

    let out = run_json(&prog);
    assert!(
        out.contains("\"state_machines\":["),
        "state_machines section present: {out}"
    );
    assert!(
        out.contains("\"type\":\"halted\""),
        "outcome is Halted: {out}"
    );
    assert!(
        out.contains("\"state\":\"check\""),
        "halted in the `check` state: {out}"
    );
    // The yield `at_target` is a symbolic finding atom (no numeric binding), so the
    // result is that symbol.
    assert!(
        out.contains("\"kind\":\"symbol\"") && out.contains("\"symbol\":\"at_target\""),
        "yielded the symbolic result at_target: {out}"
    );

    let ex = run_explain(&prog);
    assert!(
        ex.contains("Run of warfarin_titration:"),
        "explain names the machine: {ex:?}"
    );
    assert!(
        ex.contains("=> Halted at check, yields at_target"),
        "explain shows the Halted outcome + yield: {ex:?}"
    );
}

// ---------------------------------------------------------------------------
// (b) NON-TERMINATING / BUDGET — the spin loop is caught, and the run RETURNS.
// ---------------------------------------------------------------------------

#[test]
fn spin_loop_is_caught_and_never_hangs() {
    let dir = scratch("spin");
    // The §6.2 a↔b ping-pong with no `observe done`: no exit ever holds. Cycle
    // detection fires (the `(a, ∅)` key repeats) → NonTerminating; absent that, the
    // budget would return StepBudgetExceeded. Either is a typed, grounded
    // abstention — and the mere fact this test RETURNS proves the loop is bounded.
    let prog = write(
        dir.as_path(),
        "case.adj",
        "statemachine spin {\n\
         \x20   initial a\n\
         \x20   state a { transition on true to b }\n\
         \x20   state b { transition on true to a }\n\
         \x20   exit when done yield ok\n\
         \x20   budget 8 steps\n\
         \x20   source \"loop (worked example)\"\n\
         \x20   trust inferred\n\
         }\n",
    );

    let out = run_json(&prog);
    assert!(
        out.contains("\"type\":\"non_terminating\"") || out.contains("\"type\":\"step_budget_exceeded\""),
        "spin returns a typed non-terminating/budget outcome: {out}"
    );

    let ex = run_explain(&prog);
    assert!(
        ex.contains("=> NonTerminating (cycle at a)")
            || ex.contains("=> StepBudgetExceeded"),
        "explain shows the typed abstention: {ex:?}"
    );
}

// ---------------------------------------------------------------------------
// (c) STUCK — no transition guard holds and no exit holds → a dead end.
// ---------------------------------------------------------------------------

#[test]
fn dead_end_run_is_stuck() {
    let dir = scratch("stuck");
    // observe inr(5): in `check`, the only transition guard `inr < 2` is false
    // (5 is not < 2), and the exit `inr < 0` is false (5 is not < 0). No transition
    // applies and no exit holds → Stuck in `check`.
    let prog = write(
        dir.as_path(),
        "case.adj",
        "statemachine stuck_demo {\n\
         \x20   initial check\n\
         \x20   state check { transition on inr < 2 to bump }\n\
         \x20   state bump { transition on true to check do assert dose_changed }\n\
         \x20   exit when inr < 0 yield done\n\
         \x20   budget 10 steps\n\
         \x20   source \"stuck (worked example)\"\n\
         \x20   trust inferred\n\
         }\n\
         observe inr(5)\n",
    );

    let out = run_json(&prog);
    assert!(out.contains("\"type\":\"stuck\""), "outcome is Stuck: {out}");
    assert!(
        out.contains("\"state\":\"check\""),
        "stuck in the `check` state: {out}"
    );

    let ex = run_explain(&prog);
    assert!(
        ex.contains("=> Stuck in check"),
        "explain shows the Stuck outcome: {ex:?}"
    );
}

// ---------------------------------------------------------------------------
// (d) DETERMINISM — the `--explain` render is byte-identical across two runs.
// ---------------------------------------------------------------------------

#[test]
fn explain_render_is_deterministic() {
    let dir = scratch("determinism");
    let prog = write(dir.as_path(), "case.adj", &format!("{TITRATE}observe inr(2.5)\n"));
    let a = run_explain(&prog);
    let b = run_explain(&prog);
    assert_eq!(a, b, "the same program renders byte-identical --explain output");
}

// ---------------------------------------------------------------------------
// (e) OMISSION — a program with no `statemachine` emits no section, so the JSON
//     shape is byte-for-byte unchanged (the omit-when-empty invariant).
// ---------------------------------------------------------------------------

#[test]
fn no_statemachine_omits_the_section() {
    let dir = scratch("omit");
    let prog = write(dir.as_path(), "case.adj", "let dose = 5 * 60 / 100\n? dose\n");
    let out = run_json(&prog);
    assert!(
        !out.contains("state_machines"),
        "no state_machines key when the program declares no machine: {out}"
    );
}

// ---------------------------------------------------------------------------
// (f) PROVENANCE + STEPS — a run that fires transitions records each one with the
//     machine's cited provenance and any asserted facts (§3, "every loop action is
//     one provenanced entry in the ReasoningTrace").
// ---------------------------------------------------------------------------

#[test]
fn fired_transitions_carry_provenance_and_asserted_facts() {
    let dir = scratch("prov");
    // observe inr(1.5): `check` takes `inr < 2` → increase_dose (asserts
    // dose_changed), loops back, and eventually the `(state, asserted-set)` key
    // repeats → NonTerminating, with several recorded steps along the way.
    let prog = write(dir.as_path(), "case.adj", &format!("{TITRATE}observe inr(1.5)\n"));

    let out = run_json(&prog);
    assert!(
        out.contains("\"guard\":\"inr < 2\""),
        "a fired transition records its guard: {out}"
    );
    assert!(
        out.contains("\"asserted\":[\"dose_changed\"]"),
        "the asserting transition records its fact: {out}"
    );
    // Each step inlines the machine's cited provenance (inherited per transition).
    assert!(
        out.contains("warfarin dosing protocol (worked example)"),
        "steps carry the machine's cited source: {out}"
    );

    let ex = run_explain(&prog);
    assert!(
        ex.contains("(asserted dose_changed)"),
        "explain names the asserted fact on the step line: {ex:?}"
    );
}
