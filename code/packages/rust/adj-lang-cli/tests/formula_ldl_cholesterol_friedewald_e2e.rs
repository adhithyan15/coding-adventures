//! End-to-end tests for the `clinical/ldl-cholesterol-friedewald.adj` library — the Friedewald
//! estimate of LDL cholesterol (LDL-C = total cholesterol − HDL − triglycerides/5) and its three
//! exact rearrangements — driven through the built CLI binary against the SHIPPED stdlib. This
//! is the first clinical inverter whose definition carries an EMBEDDED NUMERIC CONSTANT (the
//! divide-by-5, and its inverse ×5 when solved for the triglycerides): the "/5" is part of the
//! cited equation, evaluated exactly over rationals. The same invariant as every other formula
//! library: a consumer states NO arithmetic; it imports the grounded library, binds the measured
//! panel values with `observe`, and the engine applies the cited relation on the CPU, computing
//! the EXACT value and rendering the relation's citation and trust tier in the `derived` section.
//! The four formulas INVERT around the worked case TC = 200, HDL = 50, TG = 150:
//! 200 − 50 − 150/5 = 120; 120 + 50 + 150/5 = 200; 200 − 120 − 150/5 = 50; 5·(200 − 50 − 120) = 150.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped ldl-cholesterol-friedewald library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_friedewald_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/ldl-cholesterol-friedewald.adj")
        .canonicalize()
        .expect("shipped ldl-cholesterol-friedewald.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_friedewald_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_friedewald_lib()).unwrap();
    std::fs::write(dir.join("ldl-cholesterol-friedewald.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// ldl_cholesterol — the Friedewald equation: total − HDL − triglycerides/5.
// ---------------------------------------------------------------------------

#[test]
fn imports_friedewald_library_and_estimates_ldl_with_citation() {
    let dir = scratch("ldl");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ldl-cholesterol-friedewald.adj\"\n\
         observe total_cholesterol(200)\n\
         observe hdl_cholesterol(50)\n\
         observe triglycerides(150)\n\
         ? ldl_cholesterol(total_cholesterol, hdl_cholesterol, triglycerides)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied equation's result: 200 − 50 − 150/5 = 120.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"ldl_cholesterol\"") && s.contains("\"value\":120"),
        "ldl_cholesterol(200, 50, 150) = 120: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied equation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// total_cholesterol — the same equation solved for TC: LDL + HDL + triglycerides/5.
// ---------------------------------------------------------------------------

#[test]
fn computes_total_cholesterol_from_ldl_hdl_tg_with_citation() {
    let dir = scratch("tc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ldl-cholesterol-friedewald.adj\"\n\
         observe ldl_cholesterol(120)\n\
         observe hdl_cholesterol(50)\n\
         observe triglycerides(150)\n\
         ? total_cholesterol(ldl_cholesterol, hdl_cholesterol, triglycerides)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 120 + 50 + 150/5 = 200, computed on the CPU.
    assert!(
        s.contains("\"name\":\"total_cholesterol\"") && s.contains("\"value\":200"),
        "total_cholesterol(120, 50, 150) = 200: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "total_cholesterol carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// hdl_cholesterol — the same equation solved for HDL: total − LDL − triglycerides/5.
// ---------------------------------------------------------------------------

#[test]
fn computes_hdl_cholesterol_from_total_ldl_tg_with_citation() {
    let dir = scratch("hdl");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ldl-cholesterol-friedewald.adj\"\n\
         observe total_cholesterol(200)\n\
         observe ldl_cholesterol(120)\n\
         observe triglycerides(150)\n\
         ? hdl_cholesterol(total_cholesterol, ldl_cholesterol, triglycerides)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 200 − 120 − 150/5 = 50, computed on the CPU.
    assert!(
        s.contains("\"name\":\"hdl_cholesterol\"") && s.contains("\"value\":50"),
        "hdl_cholesterol(200, 120, 150) = 50: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "hdl_cholesterol carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// triglycerides — the same equation solved for TG: 5·(total − HDL − LDL). Here the /5 becomes a
// ×5, the fourth reading of the one equation.
// ---------------------------------------------------------------------------

#[test]
fn computes_triglycerides_from_total_hdl_ldl_with_citation() {
    let dir = scratch("tg");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ldl-cholesterol-friedewald.adj\"\n\
         observe total_cholesterol(200)\n\
         observe hdl_cholesterol(50)\n\
         observe ldl_cholesterol(120)\n\
         ? triglycerides(total_cholesterol, hdl_cholesterol, ldl_cholesterol)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 5·(200 − 50 − 120) = 5·30 = 150, computed on the CPU.
    assert!(
        s.contains("\"name\":\"triglycerides\"") && s.contains("\"value\":150"),
        "triglycerides(200, 50, 120) = 150: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "triglycerides carries its StatPearls citation: {s}"
    );
}
