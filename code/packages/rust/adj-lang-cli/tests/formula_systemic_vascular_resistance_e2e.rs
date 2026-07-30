//! End-to-end tests for the `clinical/systemic-vascular-resistance.adj` library — systemic vascular
//! resistance (SVR = 80 × (mean arterial pressure − mean venous pressure) / cardiac output) and its two
//! exact rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same invariant
//! as every other formula library: a consumer states NO arithmetic; it imports the grounded library, binds
//! the mean arterial pressure, the mean venous pressure, and the cardiac output with `observe`, and the
//! engine applies the cited formula on the CPU, computing the EXACT value (over exact rationals) and
//! rendering the citation and trust tier in the `derived` section (the auditable answer). The three formulas
//! INVERT around the worked case MAP = 90, MVP = 10, CO = 5: 80 × (90 − 10) / 5 = 1280 (SVR),
//! 80 × (90 − 10) / 1280 = 5 (CO), 1280 × 5 / 80 + 10 = 90 (MAP).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation tree carries the unit constant 80 and the intermediates 80 (= 90 − 10) and
//! 6400 (= 80 × 80), so a bare numeric substring could spuriously match. The name-anchored adjacent form is
//! collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped systemic-vascular-resistance library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_svr_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/systemic-vascular-resistance.adj")
        .canonicalize()
        .expect("shipped systemic-vascular-resistance.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_svr_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_svr_lib()).unwrap();
    std::fs::write(dir.join("systemic-vascular-resistance.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// systemic_vascular_resistance — the resistance: 80 × (MAP − MVP) / CO.
// ---------------------------------------------------------------------------

#[test]
fn imports_systemic_vascular_resistance_library_and_computes_it_with_citation() {
    let dir = scratch("svr");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"systemic-vascular-resistance.adj\"\n\
         observe mean_arterial_pressure(90)\n\
         observe mean_venous_pressure(10)\n\
         observe cardiac_output(5)\n\
         ? systemic_vascular_resistance(mean_arterial_pressure, mean_venous_pressure, cardiac_output)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 80 × (90 − 10) / 5 = 1280, computed
    // EXACTLY over rationals. Match the adjacent name/value pair so the 80 constant and the 80/6400
    // intermediates in the derivation cannot spuriously satisfy a bare "value":1280.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"systemic_vascular_resistance\",\"value\":1280"),
        "systemic_vascular_resistance(90, 10, 5) = 1280: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// cardiac_output — the same equation solved for the flow: 80 × (MAP − MVP) / SVR.
// ---------------------------------------------------------------------------

#[test]
fn computes_cardiac_output_from_svr_with_citation() {
    let dir = scratch("co");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"systemic-vascular-resistance.adj\"\n\
         observe systemic_vascular_resistance(1280)\n\
         observe mean_arterial_pressure(90)\n\
         observe mean_venous_pressure(10)\n\
         ? cardiac_output(systemic_vascular_resistance, mean_arterial_pressure, mean_venous_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 80 × (90 − 10) / 1280 = 6400 / 1280 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"cardiac_output\",\"value\":5"),
        "cardiac_output(1280, 90, 10) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "cardiac_output carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// mean_arterial_pressure — the same equation solved for MAP: SVR × CO / 80 + MVP, the third reading of the
// one law.
// ---------------------------------------------------------------------------

#[test]
fn computes_mean_arterial_pressure_from_svr_with_citation() {
    let dir = scratch("map");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"systemic-vascular-resistance.adj\"\n\
         observe systemic_vascular_resistance(1280)\n\
         observe cardiac_output(5)\n\
         observe mean_venous_pressure(10)\n\
         ? mean_arterial_pressure(systemic_vascular_resistance, cardiac_output, mean_venous_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 1280 × 5 / 80 + 10 = 6400 / 80 + 10 = 80 + 10 = 90, computed on the CPU.
    assert!(
        s.contains("\"name\":\"mean_arterial_pressure\",\"value\":90"),
        "mean_arterial_pressure(1280, 5, 10) = 90: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "mean_arterial_pressure carries its cited provenance: {s}"
    );
}
