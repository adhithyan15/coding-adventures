//! End-to-end tests for the `clinical/non-hdl-cholesterol.adj` library — the definition of
//! non-HDL cholesterol (non-HDL-C = total cholesterol − HDL cholesterol) and its two exact
//! rearrangements (total = non-HDL + HDL; HDL = total − non-HDL) — driven through the built CLI
//! binary against the SHIPPED stdlib. The same invariant as every other formula library: a
//! consumer states NO arithmetic; it imports the grounded library, binds the measured
//! cholesterols with `observe`, and the engine applies the cited relation on the CPU, computing
//! the EXACT value and rendering the relation's citation and trust tier in the `derived` section
//! (the auditable answer). The three formulas INVERT around the worked case total = 190 mg/dL,
//! HDL = 40 mg/dL: 190 − 40 = 150, 150 + 40 = 190, and 190 − 150 = 40.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped non-hdl-cholesterol library, resolved from this crate's manifest
/// dir so the test is location-independent.
fn shipped_non_hdl_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/non-hdl-cholesterol.adj")
        .canonicalize()
        .expect("shipped non-hdl-cholesterol.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_nonhdl_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_non_hdl_lib()).unwrap();
    std::fs::write(dir.join("non-hdl-cholesterol.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// non_hdl_cholesterol — the definition: total cholesterol minus HDL cholesterol.
// ---------------------------------------------------------------------------

#[test]
fn imports_non_hdl_library_and_computes_it_with_citation() {
    let dir = scratch("total");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"non-hdl-cholesterol.adj\"\n\
         observe total_cholesterol(190)\n\
         observe hdl_cholesterol(40)\n\
         ? non_hdl_cholesterol(total_cholesterol, hdl_cholesterol)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 190 − 40 = 150.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"non_hdl_cholesterol\"") && s.contains("\"value\":150"),
        "non_hdl_cholesterol(190, 40) = 150: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// total_cholesterol — the same relation solved for TC: non-HDL + HDL, which INVERTS the
// difference.
// ---------------------------------------------------------------------------

#[test]
fn computes_total_cholesterol_from_non_hdl_and_hdl_with_citation() {
    let dir = scratch("tc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"non-hdl-cholesterol.adj\"\n\
         observe non_hdl_cholesterol(150)\n\
         observe hdl_cholesterol(40)\n\
         ? total_cholesterol(non_hdl_cholesterol, hdl_cholesterol)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 150 + 40 = 190, computed on the CPU.
    assert!(
        s.contains("\"name\":\"total_cholesterol\"") && s.contains("\"value\":190"),
        "total_cholesterol(150, 40) = 190: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "total_cholesterol carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// hdl_cholesterol — the same relation solved for HDL: total − non-HDL, the third reading of the
// one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_hdl_cholesterol_from_total_and_non_hdl_with_citation() {
    let dir = scratch("hdl");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"non-hdl-cholesterol.adj\"\n\
         observe total_cholesterol(190)\n\
         observe non_hdl_cholesterol(150)\n\
         ? hdl_cholesterol(total_cholesterol, non_hdl_cholesterol)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 190 − 150 = 40, computed on the CPU.
    assert!(
        s.contains("\"name\":\"hdl_cholesterol\"") && s.contains("\"value\":40"),
        "hdl_cholesterol(190, 150) = 40: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "hdl_cholesterol carries its StatPearls citation: {s}"
    );
}
