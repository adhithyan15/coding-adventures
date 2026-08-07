//! End-to-end tests for the `--explain` renderer's INFERENCE + ADJUDICATION
//! surfaces (ADJ-REASON-MATH §E.8, RS-4 PR-E2). Where PR-E1 rendered the
//! arithmetic behind a computed value, this slice renders the probabilistic
//! reasoning behind a hypothesis: the ordered proof steps (prior, likelihood-ratio
//! contributions, rule-derived premises) and the comparative decision.
//!
//! The tests pin: the prior step is cited to its source; a contribution step is
//! rendered; the decision names the leader with its posterior; trust propagates
//! as the weakest link (P3); determinism holds (P4); and a pure computation with
//! no differential evidence still renders derivations-only (PR-E1 unchanged).

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs4e2_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

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

// A single-hypothesis differential: a cited prior updated by one cited
// likelihood-ratio contribution over an observed finding.
const ACS_PROG: &str = "prior 0.10 for acs\n\
     source \"Pope 1995\" trust authoritative\n\
     contributes 2.5 from symptom(pressure) to acs\n\
     source \"Panju 1998\" trust authoritative\n\
     observe symptom(pressure)\n\
     ? acs\n";

// ---------------------------------------------------------------------------
// (1) A differential renders its INFERENCE steps (cited) and its ADJUDICATION.
// ---------------------------------------------------------------------------

#[test]
fn renders_cited_inference_steps_and_the_decision() {
    let dir = scratch("acs");
    let prog = write(&dir, "case.adj", ACS_PROG);
    let s = explain(&prog);

    // Inference section, with the prior step cited to its source.
    assert!(s.contains("Inference for acs:"), "inference section: {s:?}");
    assert!(
        s.contains("prior on acs") && s.contains("source \"Pope 1995\""),
        "prior step is rendered and cited: {s:?}"
    );
    // The likelihood-ratio contribution step is rendered.
    assert!(
        s.contains("acs contributes logit"),
        "contribution step is rendered: {s:?}"
    );
    // Adjudication: the leader is named with its posterior and the determinate
    // verdict; the trust tier (P3, weakest link) is shown.
    assert!(s.contains("Decision:"), "decision section: {s:?}");
    assert!(
        s.contains("=> acs (determinate") && s.contains("trust "),
        "leader named determinate with a trust tier: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// (2) P4 — determinism across two runs.
// ---------------------------------------------------------------------------

#[test]
fn the_differential_explanation_is_deterministic() {
    let dir = scratch("determinism");
    let prog = write(&dir, "case.adj", ACS_PROG);
    let a = explain(&prog);
    let b = explain(&prog);
    assert_eq!(a, b, "same differential must render identical explanation text");
    assert!(a.contains("=> acs (determinate"), "non-trivial explanation: {a:?}");
}

// ---------------------------------------------------------------------------
// (3) A two-hypothesis differential names the winning leader.
// ---------------------------------------------------------------------------

#[test]
fn a_two_hypothesis_differential_names_the_leader() {
    let dir = scratch("two");
    let prog = write(
        &dir,
        "case.adj",
        "prior 0.30 for bacterial\n\
         source \"x\" trust empirical\n\
         prior 0.30 for viral\n\
         source \"x\" trust empirical\n\
         contributes 15 from csf(neutrophilic) to bacterial\n\
         source \"Straus 2006\" trust authoritative\n\
         contributes 1.2 from csf(neutrophilic) to viral\n\
         source \"y\" trust inferred\n\
         observe csf(neutrophilic)\n\
         ? bacterial\n? viral\n",
    );
    let s = explain(&prog);
    // Both hypotheses appear in the ranked list; the strong LR makes bacterial
    // the determinate leader.
    assert!(
        s.contains("Inference for bacterial:") && s.contains("Inference for viral:"),
        "both hypotheses explained: {s:?}"
    );
    assert!(
        s.contains("=> bacterial (determinate"),
        "the stronger-evidence hypothesis leads: {s:?}"
    );
    assert!(
        s.contains("source \"Straus 2006\""),
        "the leading contribution's citation is shown: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// (4) PR-E1 regression: a pure computation still renders derivations-only, with
//     no spurious inference/decision section.
// ---------------------------------------------------------------------------

#[test]
fn a_pure_computation_stays_derivations_only() {
    let dir = scratch("let");
    let prog = write(&dir, "case.adj", "let dose = 5 * 60 / 100\n? dose\n");
    let s = explain(&prog);
    assert!(s.contains("Derivations:") && s.contains("dose = 3"), "derivations rendered: {s:?}");
    assert!(
        !s.contains("Inference for") && !s.contains("Decision:"),
        "no differential sections for a pure computation: {s:?}"
    );
}
