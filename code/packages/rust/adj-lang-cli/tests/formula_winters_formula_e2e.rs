//! End-to-end tests for the `clinical/winters-formula.adj` library — Winter's formula (expected PaCO₂
//! = 1.5 × HCO₃ + 8) and its exact rearrangement — driven through the built CLI binary against the
//! SHIPPED stdlib. The same invariant as every other formula library: a consumer states NO arithmetic;
//! it imports the grounded library, binds the measured bicarbonate with `observe`, and the engine
//! applies the cited formula on the CPU, computing the EXACT value (over exact rationals — 1.5 = 3/2)
//! and rendering the citation and trust tier in the `derived` section (the auditable answer). The two
//! formulas INVERT around the worked case HCO₃ = 10: 1.5 × 10 + 8 = 23 (expected PaCO₂), (23 − 8) / 1.5
//! = 10 (bicarbonate). The two asserted values (23, 10) are distinct, neither a colon-anchored prefix
//! of the other.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped winters-formula library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_wf_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/winters-formula.adj")
        .canonicalize()
        .expect("shipped winters-formula.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_wf_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

/// Copy the shipped library next to a consumer that imports it, so the CLI's
/// sandbox-checked relative import resolves.
fn place_lib(dir: &Path) {
    let lib = std::fs::read_to_string(shipped_wf_lib()).unwrap();
    std::fs::write(dir.join("winters-formula.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// expected_paco2 — Winter's formula: 1.5 × HCO₃ + 8.
// ---------------------------------------------------------------------------

#[test]
fn imports_winters_formula_library_and_computes_expected_paco2_with_citation() {
    let dir = scratch("wf");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"winters-formula.adj\"\n\
         observe hco3(10)\n\
         ? expected_paco2(hco3)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 1.5 × 10 + 8 = 23, computed
    // EXACTLY over rationals (1.5 = 3/2), not as a rounded float.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"expected_paco2\"") && s.contains("\"value\":23"),
        "expected_paco2(10) = 23: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// hco3 — the same formula solved for the bicarbonate: (expected PaCO₂ − 8) / 1.5.
// ---------------------------------------------------------------------------

#[test]
fn computes_hco3_from_expected_paco2_with_citation() {
    let dir = scratch("hco3");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"winters-formula.adj\"\n\
         observe expected_paco2(23)\n\
         ? hco3(expected_paco2)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (23 − 8) / 1.5 = 15 / 1.5 = 10, computed exactly on the CPU.
    assert!(
        s.contains("\"name\":\"hco3\"") && s.contains("\"value\":10"),
        "hco3(23) = 10: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "hco3 carries its cited provenance: {s}"
    );
}
