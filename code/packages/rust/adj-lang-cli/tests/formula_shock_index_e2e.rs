//! End-to-end tests for the `clinical/shock-index.adj` library — the definition of the shock
//! index (SI = heart rate ÷ systolic blood pressure) and its two exact rearrangements — driven
//! through the built CLI binary against the SHIPPED stdlib. The same invariant as every other
//! formula library: a consumer states NO arithmetic; it imports the grounded library, binds the
//! measured vitals with `observe`, and the engine applies the cited relation on the CPU,
//! computing the EXACT value and rendering the relation's citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case
//! HR = 140 /min, SBP = 70 mmHg: 140 ÷ 70 = 2, 2 × 70 = 140, and 140 ÷ 2 = 70. The three
//! asserted values (2, 140, 70) are chosen so none is a substring of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped shock-index library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_si_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/shock-index.adj")
        .canonicalize()
        .expect("shipped shock-index.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_si_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_si_lib()).unwrap();
    std::fs::write(dir.join("shock-index.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// shock_index — the definition: heart rate divided by systolic blood pressure.
// ---------------------------------------------------------------------------

#[test]
fn imports_shock_index_library_and_computes_it_with_citation() {
    let dir = scratch("si");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"shock-index.adj\"\n\
         observe heart_rate(140)\n\
         observe systolic_blood_pressure(70)\n\
         ? shock_index(heart_rate, systolic_blood_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 140 ÷ 70 = 2.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"shock_index\"") && s.contains("\"value\":2"),
        "shock_index(140, 70) = 2: {s}"
    );
    // … AND the review-article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// heart_rate — the same relation solved for HR: SI × SBP.
// ---------------------------------------------------------------------------

#[test]
fn computes_heart_rate_from_shock_index_and_sbp_with_citation() {
    let dir = scratch("hr");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"shock-index.adj\"\n\
         observe shock_index(2)\n\
         observe systolic_blood_pressure(70)\n\
         ? heart_rate(shock_index, systolic_blood_pressure)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 2 × 70 = 140, computed on the CPU.
    assert!(
        s.contains("\"name\":\"heart_rate\"") && s.contains("\"value\":140"),
        "heart_rate(2, 70) = 140: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "heart_rate carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// systolic_blood_pressure — the same relation solved for SBP: HR ÷ SI, the third reading of the
// one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_sbp_from_heart_rate_and_shock_index_with_citation() {
    let dir = scratch("sbp");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"shock-index.adj\"\n\
         observe heart_rate(140)\n\
         observe shock_index(2)\n\
         ? systolic_blood_pressure(heart_rate, shock_index)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 140 ÷ 2 = 70, computed on the CPU.
    assert!(
        s.contains("\"name\":\"systolic_blood_pressure\"") && s.contains("\"value\":70"),
        "systolic_blood_pressure(140, 2) = 70: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "systolic_blood_pressure carries its cited provenance: {s}"
    );
}
