//! End-to-end tests for the `clinical/filtration-fraction.adj` library — the definition of
//! the renal filtration fraction (FF = glomerular filtration rate ÷ renal plasma flow) and
//! its two exact rearrangements (GFR = FF × RPF, RPF = GFR ÷ FF) — driven through the built
//! CLI binary against the SHIPPED stdlib. Each proves the same invariant as the other formula
//! libraries: a consumer states NO arithmetic; it imports the grounded library, binds the
//! measured quantities with `observe`, and the engine applies the cited relation on the CPU,
//! computing the EXACT value and rendering the relation's citation and trust tier in the
//! `derived` section (the auditable answer). The three formulas INVERT around the worked case
//! GFR = 120 mL/min, RPF = 600 mL/min: 120 ÷ 600 = 0.2, and both 0.2 × 600 = 120 and
//! 120 ÷ 0.2 = 600 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped filtration-fraction library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_ff_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/filtration-fraction.adj")
        .canonicalize()
        .expect("shipped filtration-fraction.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ff_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_ff_lib()).unwrap();
    std::fs::write(dir.join("filtration-fraction.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// filtration_fraction — the definition: the glomerular filtration rate over the renal plasma
// flow.
// ---------------------------------------------------------------------------

#[test]
fn imports_ff_library_and_computes_filtration_fraction_with_citation() {
    let dir = scratch("ff");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"filtration-fraction.adj\"\n\
         observe glomerular_filtration_rate(120)\n\
         observe renal_plasma_flow(600)\n\
         ? filtration_fraction(glomerular_filtration_rate, renal_plasma_flow)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 120 / 600 = 0.2.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"filtration_fraction\"") && s.contains("\"value\":0.2"),
        "filtration_fraction(120, 600) = 0.2: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// glomerular_filtration_rate — the same relation solved for GFR: FF × RPF, which INVERTS the
// filtration fraction just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_gfr_from_ff_and_rpf_with_citation() {
    let dir = scratch("gfr");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"filtration-fraction.adj\"\n\
         observe filtration_fraction(0.2)\n\
         observe renal_plasma_flow(600)\n\
         ? glomerular_filtration_rate(filtration_fraction, renal_plasma_flow)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 0.2 * 600 = 120, computed on the CPU.
    assert!(
        s.contains("\"name\":\"glomerular_filtration_rate\"") && s.contains("\"value\":120"),
        "glomerular_filtration_rate(0.2, 600) = 120: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "glomerular_filtration_rate carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// renal_plasma_flow — the same relation solved for RPF: GFR ÷ FF, the third exact reading of
// the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_rpf_from_gfr_and_ff_with_citation() {
    let dir = scratch("rpf");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"filtration-fraction.adj\"\n\
         observe glomerular_filtration_rate(120)\n\
         observe filtration_fraction(0.2)\n\
         ? renal_plasma_flow(glomerular_filtration_rate, filtration_fraction)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 120 / 0.2 = 600, computed on the CPU.
    assert!(
        s.contains("\"name\":\"renal_plasma_flow\"") && s.contains("\"value\":600"),
        "renal_plasma_flow(120, 0.2) = 600: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "renal_plasma_flow carries its StatPearls citation: {s}"
    );
}
