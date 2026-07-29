//! End-to-end tests for the `clinical/renal-clearance.adj` library — the renal-clearance relation
//! (clearance = urine concentration × urine flow ÷ plasma concentration) and its two exact
//! rearrangements — driven through the built CLI binary against the SHIPPED stdlib. The same
//! invariant as every other formula library: a consumer states NO arithmetic; it imports the grounded
//! library, binds the measured concentrations and urine flow with `observe`, and the engine applies
//! the cited equation on the CPU, computing the EXACT value and rendering the citation and trust tier
//! in the `derived` section (the auditable answer). The three formulas INVERT around the worked case
//! urine concentration = 90, urine flow = 2, plasma concentration = 3: 90 × 2 ÷ 3 = 60 (clearance),
//! 60 × 3 ÷ 2 = 90 (urine conc), 90 × 2 ÷ 60 = 3 (plasma conc). The four asserted values (60, 90, 3,
//! 2) are distinct, none a colon-anchored prefix of another rendered value.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped renal-clearance library, resolved from this crate's manifest dir so
/// the test is location-independent.
fn shipped_rc_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/renal-clearance.adj")
        .canonicalize()
        .expect("shipped renal-clearance.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rc_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_rc_lib()).unwrap();
    std::fs::write(dir.join("renal-clearance.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// renal_clearance — the relation: urine concentration × urine flow ÷ plasma concentration.
// ---------------------------------------------------------------------------

#[test]
fn imports_renal_clearance_library_and_computes_it_with_citation() {
    let dir = scratch("rc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"renal-clearance.adj\"\n\
         observe urine_concentration(90)\n\
         observe urine_flow(2)\n\
         observe plasma_concentration(3)\n\
         ? renal_clearance(urine_concentration, urine_flow, plasma_concentration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied equation's result: 90 × 2 ÷ 3 = 60.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"renal_clearance\"") && s.contains("\"value\":60"),
        "renal_clearance(90, 2, 3) = 60: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// urine_concentration — the same definition solved for the urine concentration: Cx × Px ÷ V.
// ---------------------------------------------------------------------------

#[test]
fn computes_urine_concentration_from_clearance_with_citation() {
    let dir = scratch("ux");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"renal-clearance.adj\"\n\
         observe renal_clearance(60)\n\
         observe urine_flow(2)\n\
         observe plasma_concentration(3)\n\
         ? urine_concentration(renal_clearance, urine_flow, plasma_concentration)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 60 × 3 ÷ 2 = 90, computed on the CPU.
    assert!(
        s.contains("\"name\":\"urine_concentration\"") && s.contains("\"value\":90"),
        "urine_concentration(60, 2, 3) = 90: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "urine_concentration carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// plasma_concentration — the same definition solved for the plasma concentration: Ux × V ÷ Cx, the
// third reading of the one equation.
// ---------------------------------------------------------------------------

#[test]
fn computes_plasma_concentration_from_clearance_with_citation() {
    let dir = scratch("px");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"renal-clearance.adj\"\n\
         observe urine_concentration(90)\n\
         observe urine_flow(2)\n\
         observe renal_clearance(60)\n\
         ? plasma_concentration(urine_concentration, urine_flow, renal_clearance)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 90 × 2 ÷ 60 = 3, computed on the CPU.
    assert!(
        s.contains("\"name\":\"plasma_concentration\"") && s.contains("\"value\":3"),
        "plasma_concentration(90, 2, 60) = 3: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "plasma_concentration carries its cited provenance: {s}"
    );
}
