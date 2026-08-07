//! End-to-end tests for the `clinical/maintenance-dose.adj` library — the pharmacokinetic maintenance dose
//! (MD = steady-state concentration × clearance × dosing interval / bioavailability) and its two exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same invariant as
//! every other formula library: a consumer states NO arithmetic; it imports the grounded library, binds the
//! four pharmacokinetic quantities with `observe`, and the engine applies the cited formula on the CPU,
//! computing the EXACT value (over exact rationals) and rendering the citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case SSC = 10,
//! CL = 5, DI = 8, B = 1: 10 × 5 × 8 / 1 = 400 (MD), 400 × 1 / (5 × 8) = 10 (SSC), 400 × 1 / (10 × 8) = 5
//! (CL).
//!
//! The assertions match the ADJACENT `"name":...,"value":...` pair the engine renders, rather than a bare
//! `"value":N`: the derivation tree contains the intermediates 40 and 80 (and the B = 1 input), so a bare
//! `"value":40`-style substring could spuriously match the leading digits of a longer number. The adjacent
//! form is collision-proof.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped maintenance-dose library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_md_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/maintenance-dose.adj")
        .canonicalize()
        .expect("shipped maintenance-dose.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_md_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_md_lib()).unwrap();
    std::fs::write(dir.join("maintenance-dose.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// md — the dose: SSC × CL × DI / B.
// ---------------------------------------------------------------------------

#[test]
fn imports_maintenance_dose_library_and_computes_it_with_citation() {
    let dir = scratch("md");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"maintenance-dose.adj\"\n\
         observe ssc(10)\n\
         observe cl(5)\n\
         observe di(8)\n\
         observe b(1)\n\
         ? md(ssc, cl, di, b)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied formula's result: 10 × 5 × 8 / 1 = 400, computed
    // EXACTLY over rationals. Match the adjacent name/value pair so intermediates in the derivation cannot
    // spuriously satisfy a bare "value":400.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"md\",\"value\":400"),
        "md(10, 5, 8, 1) = 400: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied formula carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// ssc — the same equation solved for the steady-state concentration: MD × B / (CL × DI).
// ---------------------------------------------------------------------------

#[test]
fn computes_ssc_from_maintenance_dose_with_citation() {
    let dir = scratch("ssc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"maintenance-dose.adj\"\n\
         observe md(400)\n\
         observe cl(5)\n\
         observe di(8)\n\
         observe b(1)\n\
         ? ssc(md, cl, di, b)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 400 × 1 / (5 × 8) = 400 / 40 = 10, computed on the CPU.
    assert!(
        s.contains("\"name\":\"ssc\",\"value\":10"),
        "ssc(400, 5, 8, 1) = 10: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "ssc carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// cl — the same equation solved for the clearance: MD × B / (SSC × DI), the third reading of the one dose.
// ---------------------------------------------------------------------------

#[test]
fn computes_cl_from_maintenance_dose_with_citation() {
    let dir = scratch("cl");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"maintenance-dose.adj\"\n\
         observe md(400)\n\
         observe ssc(10)\n\
         observe di(8)\n\
         observe b(1)\n\
         ? cl(md, ssc, di, b)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 400 × 1 / (10 × 8) = 400 / 80 = 5, computed on the CPU.
    assert!(
        s.contains("\"name\":\"cl\",\"value\":5"),
        "cl(400, 10, 8, 1) = 5: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "cl carries its cited provenance: {s}"
    );
}
