//! End-to-end tests for the `clinical/metabolic-alkalosis-compensation.adj` library — the expected
//! respiratory compensation for metabolic alkalosis (expected PaCO2 = 40 + 0.6 × (HCO3 − 24)) and its one
//! exact rearrangement — driven through the built CLI binary against the SHIPPED stdlib. The same invariant as
//! every other formula library: a consumer states NO arithmetic; it imports the grounded library, binds the
//! serum bicarbonate with `observe`, and the engine applies the cited formula on the CPU, computing the EXACT
//! value (over exact rationals) and rendering the citation and trust tier in the `derived` section (the
//! auditable answer). The two formulas INVERT around the worked case HCO3 = 44:
//! 40 + 0.6 × (44 − 24) = 52 (expected PaCO2), (52 − 40) × 10 / 6 + 24 = 44 (HCO3).
//!
//! The slope 0.6 is written in the library as 6/10 (its exact integer form) so the computation stays in exact
//! integer arithmetic and every rendered value is an exact integer.
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation carries the constants 40, 6, 10 and 24, so a bare numeric substring could
//! spuriously match. The name-anchored adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped metabolic-alkalosis-compensation library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_mac_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/metabolic-alkalosis-compensation.adj")
        .canonicalize()
        .expect("shipped metabolic-alkalosis-compensation.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mac_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_mac_lib()).unwrap();
    std::fs::write(dir.join("metabolic-alkalosis-compensation.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// expected_paco2 — the compensation: 40 + 0.6 × (HCO3 − 24).
// ---------------------------------------------------------------------------

#[test]
fn imports_metabolic_alkalosis_compensation_library_and_computes_it_with_citation() {
    let dir = scratch("mac");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"metabolic-alkalosis-compensation.adj\"\n\
         observe serum_bicarbonate(44)\n\
         ? expected_paco2(serum_bicarbonate)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 40 + 6 × (44 − 24) / 10 = 40 + 12 = 52,
    // computed EXACTLY over rationals. Match the adjacent name/value pair so the 40/6/10/24 constants cannot
    // spuriously satisfy a bare "value":52.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"expected_paco2\",\"value\":52"),
        "expected_paco2(44) = 52: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_bicarbonate — the same equation solved for HCO3: (PaCO2 − 40) × 10 / 6 + 24.
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_bicarbonate_from_expected_paco2_with_citation() {
    let dir = scratch("hco3");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"metabolic-alkalosis-compensation.adj\"\n\
         observe expected_paco2(52)\n\
         ? serum_bicarbonate(expected_paco2)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (52 − 40) × 10 / 6 + 24 = 120 / 6 + 24 = 20 + 24 = 44, computed on the CPU.
    assert!(
        s.contains("\"name\":\"serum_bicarbonate\",\"value\":44"),
        "serum_bicarbonate(52) = 44: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_bicarbonate carries its cited provenance: {s}"
    );
}
