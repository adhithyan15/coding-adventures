//! End-to-end tests for the `clinical/drug-clearance.adj` library — the definition of drug
//! clearance (CL = rate of drug removal ÷ plasma concentration) and its two exact rearrangements —
//! driven through the built CLI binary against the SHIPPED stdlib. The same invariant as every
//! other formula library: a consumer states NO arithmetic; it imports the grounded library, binds
//! the measured quantities with `observe`, and the engine applies the cited relation on the CPU,
//! computing the EXACT value and rendering the relation's citation and trust tier in the `derived`
//! section (the auditable answer). The three formulas INVERT around the worked case rate = 100
//! mg/min, Cp = 5 mg/mL: 100 ÷ 5 = 20, 20 × 5 = 100, and 100 ÷ 20 = 5. The three asserted values
//! (20, 100, 5) are chosen so none is a colon-anchored prefix of another rendered value. This is
//! the second pharmacokinetics library, the companion of volume-of-distribution.adj (Vd and CL are
//! the two independent primary PK parameters).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped drug-clearance library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_cl_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/drug-clearance.adj")
        .canonicalize()
        .expect("shipped drug-clearance.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_cl_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_cl_lib()).unwrap();
    std::fs::write(dir.join("drug-clearance.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// clearance — the definition: rate of drug removal divided by plasma concentration.
// ---------------------------------------------------------------------------

#[test]
fn imports_drug_clearance_library_and_computes_it_with_citation() {
    let dir = scratch("cl");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"drug-clearance.adj\"\n\
         observe rate_of_drug_removal(100)\n\
         observe plasma_concentration(5)\n\
         ? clearance(rate_of_drug_removal, plasma_concentration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 100 ÷ 5 = 20.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"clearance\"") && s.contains("\"value\":20"),
        "clearance(100, 5) = 20: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// rate_of_drug_removal — the same relation solved for the rate: CL × Cp.
// ---------------------------------------------------------------------------

#[test]
fn computes_rate_from_clearance_and_concentration_with_citation() {
    let dir = scratch("rate");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"drug-clearance.adj\"\n\
         observe clearance(20)\n\
         observe plasma_concentration(5)\n\
         ? rate_of_drug_removal(clearance, plasma_concentration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 20 × 5 = 100, computed on the CPU.
    assert!(
        s.contains("\"name\":\"rate_of_drug_removal\"") && s.contains("\"value\":100"),
        "rate_of_drug_removal(20, 5) = 100: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "rate_of_drug_removal carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// plasma_concentration — the same relation solved for the concentration: rate ÷ CL, the third
// reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_concentration_from_rate_and_clearance_with_citation() {
    let dir = scratch("cp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"drug-clearance.adj\"\n\
         observe rate_of_drug_removal(100)\n\
         observe clearance(20)\n\
         ? plasma_concentration(rate_of_drug_removal, clearance)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 100 ÷ 20 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"plasma_concentration\"") && s.contains("\"value\":5"),
        "plasma_concentration(100, 20) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "plasma_concentration carries its cited provenance: {s}"
    );
}
