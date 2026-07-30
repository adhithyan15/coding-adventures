//! End-to-end tests for the `clinical/fractional-excretion-of-calcium.adj` library — the fractional excretion
//! of calcium (FeCa = (UCa × PCr) / (PCa × UCr)) and its four exact rearrangements — driven through the built
//! CLI binary against the SHIPPED stdlib. The same invariant as every other formula library: a consumer states
//! NO arithmetic; it imports the grounded library, binds the four measured values with `observe`, and the
//! engine applies the cited formula on the CPU, computing the EXACT value (over exact rationals) and rendering
//! the citation and trust tier in the `derived` section (the auditable answer). The five formulas INVERT
//! around the worked case UCa = 1, PCr = 1, PCa = 8, UCr = 8 (FeCa = 1/64 = 0.015625).
//!
//! This source types FeCa as the bare fraction (NO × 100), so the result is a small fraction; the worked FeCa
//! is the DYADIC value 1/64 (a power-of-2 denominator, exactly representable in f64) so every rendered value —
//! the fraction and the integer inverses — is exact.
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders. The name-anchored
//! adjacent form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped fractional-excretion-of-calcium library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_feca_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/fractional-excretion-of-calcium.adj")
        .canonicalize()
        .expect("shipped fractional-excretion-of-calcium.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_feca_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_feca_lib()).unwrap();
    std::fs::write(dir.join("fractional-excretion-of-calcium.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// fractional_excretion_calcium — the FeCa: (UCa × PCr) / (PCa × UCr).
// ---------------------------------------------------------------------------

#[test]
fn imports_feca_library_and_computes_it_with_citation() {
    let dir = scratch("feca");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-calcium.adj\"\n\
         observe urinary_calcium(1)\n\
         observe serum_creatinine(1)\n\
         observe serum_calcium(8)\n\
         observe urinary_creatinine(8)\n\
         ? fractional_excretion_calcium(urinary_calcium, serum_creatinine, serum_calcium, urinary_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (1 × 1) / (8 × 8) = 1/64 = 0.015625, computed EXACTLY over rationals (a dyadic fraction).
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"fractional_excretion_calcium\",\"value\":0.015625"),
        "FeCa(1, 1, 8, 8) = 0.015625: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// serum_calcium — solved for PCa: (UCa × PCr) / (FeCa × UCr).
// ---------------------------------------------------------------------------

#[test]
fn computes_serum_calcium_from_feca_with_citation() {
    let dir = scratch("pca");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-calcium.adj\"\n\
         observe fractional_excretion_calcium(0.015625)\n\
         observe urinary_calcium(1)\n\
         observe serum_creatinine(1)\n\
         observe urinary_creatinine(8)\n\
         ? serum_calcium(fractional_excretion_calcium, urinary_calcium, serum_creatinine, urinary_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (1 × 1) / (0.015625 × 8) = 1 / 0.125 = 8 (0.015625 = 1/64 is exact in f64).
    assert!(
        s.contains("\"name\":\"serum_calcium\",\"value\":8"),
        "serum_calcium(0.015625, 1, 1, 8) = 8: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "serum_calcium carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urinary_calcium — solved for UCa: FeCa × PCa × UCr / PCr.
// ---------------------------------------------------------------------------

#[test]
fn computes_urinary_calcium_from_feca_with_citation() {
    let dir = scratch("uca");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fractional-excretion-of-calcium.adj\"\n\
         observe fractional_excretion_calcium(0.015625)\n\
         observe serum_creatinine(1)\n\
         observe serum_calcium(8)\n\
         observe urinary_creatinine(8)\n\
         ? urinary_calcium(fractional_excretion_calcium, serum_creatinine, serum_calcium, urinary_creatinine)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 0.015625 × 8 × 8 / 1 = 0.015625 × 64 = 1.
    assert!(
        s.contains("\"name\":\"urinary_calcium\",\"value\":1"),
        "urinary_calcium(0.015625, 1, 8, 8) = 1: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urinary_calcium carries its cited provenance: {s}"
    );
}
