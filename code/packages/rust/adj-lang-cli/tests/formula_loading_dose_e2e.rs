//! End-to-end tests for the `clinical/loading-dose.adj` library — the loading-dose relation
//! (Loading dose = (Cp × Vd) / F) and its three exact rearrangements — driven through the built CLI
//! binary against the SHIPPED stdlib. The same invariant as every other formula library: a consumer
//! states NO arithmetic; it imports the grounded library, binds the measured/target quantities with
//! `observe`, and the engine applies the cited relation on the CPU, computing the EXACT value and
//! rendering the relation's citation and trust tier in the `derived` section (the auditable answer).
//! This is the FIRST four-quantity formulabook in the clinical track — the dosing payoff of
//! volume-of-distribution.adj. The four formulas INVERT around the worked case Cp = 5 mg/L, Vd =
//! 40 L, F = 1: (5 × 40) / 1 = 200, (200 × 1) / 40 = 5, (200 × 1) / 5 = 40, and (5 × 40) / 200 = 1.
//! The four asserted values (200, 5, 40, 1) are chosen so none is a colon-anchored prefix of another
//! rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped loading-dose library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_ld_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/loading-dose.adj")
        .canonicalize()
        .expect("shipped loading-dose.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ld_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_ld_lib()).unwrap();
    std::fs::write(dir.join("loading-dose.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// loading_dose — the relation: (concentration × volume of distribution) / bioavailability.
// ---------------------------------------------------------------------------

#[test]
fn imports_loading_dose_library_and_computes_it_with_citation() {
    let dir = scratch("ld");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"loading-dose.adj\"\n\
         observe desired_plasma_concentration(5)\n\
         observe volume_of_distribution(40)\n\
         observe bioavailability(1)\n\
         ? loading_dose(desired_plasma_concentration, volume_of_distribution, bioavailability)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: (5 × 40) / 1 = 200.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"loading_dose\"") && s.contains("\"value\":200"),
        "loading_dose(5, 40, 1) = 200: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// desired_plasma_concentration — the same relation solved for Cp: (LD × F) / Vd.
// ---------------------------------------------------------------------------

#[test]
fn computes_concentration_from_dose_volume_and_bioavailability_with_citation() {
    let dir = scratch("cp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"loading-dose.adj\"\n\
         observe loading_dose(200)\n\
         observe volume_of_distribution(40)\n\
         observe bioavailability(1)\n\
         ? desired_plasma_concentration(loading_dose, volume_of_distribution, bioavailability)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (200 × 1) / 40 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"desired_plasma_concentration\"") && s.contains("\"value\":5"),
        "desired_plasma_concentration(200, 40, 1) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "desired_plasma_concentration carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// volume_of_distribution — the same relation solved for Vd: (LD × F) / Cp.
// ---------------------------------------------------------------------------

#[test]
fn computes_volume_from_dose_concentration_and_bioavailability_with_citation() {
    let dir = scratch("vd");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"loading-dose.adj\"\n\
         observe loading_dose(200)\n\
         observe desired_plasma_concentration(5)\n\
         observe bioavailability(1)\n\
         ? volume_of_distribution(loading_dose, desired_plasma_concentration, bioavailability)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (200 × 1) / 5 = 40, computed on the CPU.
    assert!(
        s.contains("\"name\":\"volume_of_distribution\"") && s.contains("\"value\":40"),
        "volume_of_distribution(200, 5, 1) = 40: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "volume_of_distribution carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// bioavailability — the same relation solved for F: (Cp × Vd) / LD, the fourth reading of the one
// relation.
// ---------------------------------------------------------------------------

#[test]
fn computes_bioavailability_from_concentration_volume_and_dose_with_citation() {
    let dir = scratch("f");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"loading-dose.adj\"\n\
         observe desired_plasma_concentration(5)\n\
         observe volume_of_distribution(40)\n\
         observe loading_dose(200)\n\
         ? bioavailability(loading_dose, desired_plasma_concentration, volume_of_distribution)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (5 × 40) / 200 = 1, computed on the CPU.
    assert!(
        s.contains("\"name\":\"bioavailability\"") && s.contains("\"value\":1"),
        "bioavailability(200, 5, 40) = 1: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "bioavailability carries its cited provenance: {s}"
    );
}
