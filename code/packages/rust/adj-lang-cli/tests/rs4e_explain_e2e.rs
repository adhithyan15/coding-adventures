//! End-to-end tests for the `explain` renderer (ADJ-REASON-MATH §E.8, RS-4 PR-E)
//! — the human-readable, projection-only view of the reasoning, driven through
//! the built CLI binary with `--explain`.
//!
//! This first slice renders the DERIVATIONS surface: the arithmetic behind each
//! `let`/formula value, shown operand-by-operand down to its cited leaves. The
//! tests pin the invariants that make the explanation trustworthy rather than
//! merely pretty:
//!
//! - the §E.8.4 op-tree shape (operand-by-operand, literals inline),
//! - P2 provenance on every line (the applied formula's citation on the value
//!   line; an observed leaf without attribution marked `[unattributed]`, never
//!   silently blank),
//! - P4 determinism (the same program renders byte-identical text twice),
//! - and that the JSON trail is unchanged when `--explain` is absent (the human
//!   view is opt-in, not a replacement).

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs4e_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the CLI with the given extra args before the program path; returns
/// (success, stdout, stderr).
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

fn explain(program: &Path) -> String {
    let (ok, out, err) = run_with(&["--explain"], program);
    assert!(ok, "--explain exited non-zero: {err}");
    out
}

fn write(dir: &Path, name: &str, src: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p
}

// ---------------------------------------------------------------------------
// (1) A pure `let` renders the arithmetic operand-by-operand (the §E.8.4 shape),
//     with literal constants shown inline (they assert nothing new).
// ---------------------------------------------------------------------------

#[test]
fn a_pure_let_renders_the_op_tree_operand_by_operand() {
    let dir = scratch("let");
    let prog = write(&dir, "case.adj", "let dose = 5 * 60 / 100\n? dose\n");
    let s = explain(&prog);

    // The bound value and its dimension head the block.
    assert!(
        s.contains("dose = 3 [scalar]"),
        "value line present: {s:?}"
    );
    // The division is shown over its two operands (the product, parenthesized,
    // and the literal 100)...
    assert!(
        s.contains("3 = (300) / 100"),
        "outer division shown operand-by-operand: {s:?}"
    );
    // ...and the product is expanded one level deeper.
    assert!(
        s.contains("300 = 5 * 60"),
        "inner product expanded a level deeper: {s:?}"
    );
    // A pure `let` has no library claim, so no formula citation is attached.
    assert!(
        !s.contains("<= source"),
        "no formula citation on a plain let: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// (2) A provenanced `formula` puts its citation on the value line (P2), and an
//     observed leaf with no attribution renders `[unattributed]` (P2) — never a
//     silent blank.
// ---------------------------------------------------------------------------

const FORMULA_PROG: &str = "dictionary d {\n\
     define a : finding surface \"a\"\n\
     define b : finding surface \"b\"\n\
     }\n\
     formulabook fb {\n\
     use d\n\
     formula total(a, b) = a + b source \"test source\" locator \"L1\" trust authoritative\n\
     }\n\
     observe a(2)\n\
     observe b(3)\n\
     ? total(a, b)\n";

#[test]
fn a_formula_value_carries_its_citation_and_marks_unattributed_leaves() {
    let dir = scratch("formula");
    let prog = write(&dir, "case.adj", FORMULA_PROG);
    let s = explain(&prog);

    // P2: the applied formula's citation is on the value line.
    assert!(
        s.contains("total = 5 [scalar]")
            && s.contains("<= source \"test source\" locator \"L1\" trust authoritative"),
        "formula value carries its cited provenance: {s:?}"
    );
    // The sum is shown over its named operands.
    assert!(s.contains("5 = a + b"), "sum shown operand-by-operand: {s:?}");
    // P2: the observed leaves carry no attribution, so they are marked — not blank.
    assert!(
        s.contains("a = 2   [unattributed]") && s.contains("b = 3   [unattributed]"),
        "unattributed observed leaves are marked, never silently blank: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// (3) P4 — determinism: the same program renders BYTE-IDENTICAL text twice.
// ---------------------------------------------------------------------------

#[test]
fn the_explanation_is_byte_deterministic() {
    let dir = scratch("determinism");
    let prog = write(&dir, "case.adj", FORMULA_PROG);
    let a = explain(&prog);
    let b = explain(&prog);
    assert_eq!(a, b, "same program must render identical explanation text");
    // And it is non-trivial (guards against "identical because both empty").
    assert!(a.contains("total = 5"), "explanation is non-empty: {a:?}");
}

// ---------------------------------------------------------------------------
// (4) The JSON trail is unchanged when `--explain` is absent — the human view is
//     opt-in, not a replacement for the machine trail (§E.8.3).
// ---------------------------------------------------------------------------

#[test]
fn without_the_flag_the_json_trail_is_unchanged() {
    let dir = scratch("json");
    let prog = write(&dir, "case.adj", FORMULA_PROG);
    let (ok, out, err) = run_with(&[], &prog);
    assert!(ok, "default run exited non-zero: {err}");
    // Default output is the JSON object, not the explanation text.
    assert!(
        out.trim_start().starts_with('{') && out.contains("\"derived\":["),
        "default output is the JSON trail: {out:?}"
    );
    assert!(
        !out.contains("[unattributed]") && !out.contains("<= source"),
        "explanation prose does not leak into the default JSON: {out:?}"
    );
}
