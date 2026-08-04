//! AR-3 §4 — the `--explain` DIALECTICAL rendering: when a paper's rebuttal
//! DEFEATS a conclusion (a `functional` thesis + `context_order` → the engine's
//! `enumerate_governing` marks the loser `Defeated`/the winner `Governing`),
//! `--explain` now NARRATES the withdrawal — it names the defeated conclusion
//! WITHDRAWN, cites its defeater and the context precedence that withdrew it, and
//! marks the surviving rival GOVERNING. The `--explain` renderer reads the SAME
//! `governing` resolution the plain JSON already reports; it re-decides nothing.
//! An uncontested recall query is left exactly as ADR-6 rendered it (no noise).

use std::path::{Path, PathBuf};
use std::process::Command;

fn arg_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs/data/adj-argument-ir/rebuttal")
}

fn facts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs/data/adj-facts-stdlib")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_defby_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn explain(program: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg("--explain")
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    assert!(out.status.success(), "--explain exited non-zero: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

// ---------------------------------------------------------------------------
// A rebutted conclusion is narrated as WITHDRAWN (defeated by, under precedence);
// the surviving rival is GOVERNING.
// ---------------------------------------------------------------------------

#[test]
fn explain_narrates_the_withdrawal_and_the_governing_rival() {
    let s = explain(&arg_dir().join("rebuttal-inblock.adj"));
    // The fatigue conclusion (context initial_report) is WITHDRAWN, defeated by
    // the overload conclusion under the paper's context precedence.
    assert!(
        s.contains("failed_by(axle, fatigue)")
            && s.contains("[WITHDRAWN — defeated by failed_by(axle, overload)"),
        "the fatigue conclusion must be narrated as withdrawn, defeated by overload:\n{s}"
    );
    // The context precedence that withdrew it is named (reanalysis outranks initial_report).
    assert!(
        s.contains("reanalysis outranks initial_report"),
        "the withdrawal must cite the context precedence that resolved it:\n{s}"
    );
    // The reanalysis conclusion GOVERNS.
    assert!(
        s.contains("failed_by(axle, overload)") && s.contains("[GOVERNING]"),
        "the reanalysis conclusion must be marked governing:\n{s}"
    );
    // We narrate the resolution, we don't discard the loser — the withdrawn
    // conclusion's own premise chain is still rendered (auditable).
    assert!(
        s.contains("premise shows(surface, beach_marks)"),
        "the defeated conclusion's grounds stay visible for audit:\n{s}"
    );
}

// ---------------------------------------------------------------------------
// An UNDERCUT thesis abstains — it is not falsely narrated as "withdrawn".
// ---------------------------------------------------------------------------

#[test]
fn explain_undercut_abstains_not_withdrawn() {
    let s = explain(&arg_dir().join("undercut-inblock.adj"));
    assert!(s.contains("abstained: no grounded chain derives this"), "{s}");
    assert!(!s.contains("WITHDRAWN"), "an undercut abstention is not a defeat:\n{s}");
    assert!(!s.contains("GOVERNING"), "no rival is asserted under an undercut:\n{s}");
}

// ---------------------------------------------------------------------------
// An ordinary uncontested recall query is unchanged — no dialectical suffix.
// ---------------------------------------------------------------------------

#[test]
fn explain_uncontested_recall_carries_no_govern_suffix() {
    let dir = scratch("recall");
    std::fs::copy(
        facts_dir().join("language/greek-alphabet.adj"),
        dir.join("greek-alphabet.adj"),
    )
    .expect("copy shipped greek-alphabet.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"greek-alphabet.adj\"\n? greek_letter_position(alpha, $N)\n",
    )
    .unwrap();

    let s = explain(&dir.join("case.adj"));
    assert!(s.contains("greek_letter_position(alpha, 1)"), "the recall still renders: {s}");
    assert!(
        !s.contains("GOVERNING") && !s.contains("WITHDRAWN") && !s.contains("CONFLICT"),
        "an uncontested recall must carry no dialectical suffix:\n{s}"
    );
}
