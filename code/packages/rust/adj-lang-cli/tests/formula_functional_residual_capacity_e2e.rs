//! End-to-end tests for the `clinical/functional-residual-capacity.adj` library — the
//! definition of the functional residual capacity (FRC = residual volume + expiratory
//! reserve volume) and its two exact rearrangements (each component = the total minus the
//! other) — driven through the built CLI binary against the SHIPPED stdlib. A consumer
//! states NO arithmetic; it imports the grounded library, binds the measured sub-volumes
//! with `observe`, and the engine applies the cited relation on the CPU, computing the
//! EXACT value and rendering the relation's citation and trust tier in the `derived`
//! section (the auditable answer). The three formulas INVERT around the worked case
//! RV = 2, ERV = 4: 2 + 4 = 6, and 6 minus either sub-volume recovers the other.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped functional-residual-capacity library, resolved from this
/// crate's manifest dir so the test is location-independent.
fn shipped_frc_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/functional-residual-capacity.adj")
        .canonicalize()
        .expect("shipped functional-residual-capacity.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_frc_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_frc_lib()).unwrap();
    std::fs::write(dir.join("functional-residual-capacity.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// functional_residual_capacity — the definition: the sum of the two sub-volumes.
// ---------------------------------------------------------------------------

#[test]
fn imports_frc_library_and_computes_total_with_citation() {
    let dir = scratch("total");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"functional-residual-capacity.adj\"\n\
         observe residual_volume(2)\n\
         observe expiratory_reserve_volume(4)\n\
         ? functional_residual_capacity(residual_volume, expiratory_reserve_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 2 + 4 = 6.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"functional_residual_capacity\"") && s.contains("\"value\":6"),
        "functional_residual_capacity(2, 4) = 6: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// residual_volume — the same relation solved for RV: FRC − ERV, which INVERTS the total.
// ---------------------------------------------------------------------------

#[test]
fn computes_residual_volume_from_total_and_erv_with_citation() {
    let dir = scratch("rv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"functional-residual-capacity.adj\"\n\
         observe functional_residual_capacity(6)\n\
         observe expiratory_reserve_volume(4)\n\
         ? residual_volume(functional_residual_capacity, expiratory_reserve_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 - 4 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"residual_volume\"") && s.contains("\"value\":2"),
        "residual_volume(6, 4) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "residual_volume carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// expiratory_reserve_volume — the same relation solved for ERV: FRC − RV.
// ---------------------------------------------------------------------------

#[test]
fn computes_expiratory_reserve_volume_from_total_and_rv_with_citation() {
    let dir = scratch("erv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"functional-residual-capacity.adj\"\n\
         observe functional_residual_capacity(6)\n\
         observe residual_volume(2)\n\
         ? expiratory_reserve_volume(functional_residual_capacity, residual_volume)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 - 2 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"expiratory_reserve_volume\"") && s.contains("\"value\":4"),
        "expiratory_reserve_volume(6, 2) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "expiratory_reserve_volume carries its StatPearls citation: {s}"
    );
}
