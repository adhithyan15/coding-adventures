//! End-to-end tests for the `clinical/inspiratory-capacity.adj` library — the FRC/IC
//! partition of the total lung capacity (total lung capacity = functional residual
//! capacity + inspiratory capacity) and its two exact rearrangements (inspiratory
//! capacity = TLC − FRC, functional residual capacity = TLC − IC) — driven through the
//! built CLI binary against the SHIPPED stdlib. Each proves the same invariant as the
//! other formula libraries: a consumer states NO arithmetic; it imports the grounded
//! library, binds the measured quantities with `observe`, and the engine applies the
//! cited relation on the CPU, computing the EXACT value and rendering the relation's
//! citation and trust tier in the `derived` section (the auditable answer). The three
//! formulas INVERT around the worked case FRC = 2 L, IC = 4 L: 2 + 4 = 6, and both
//! 6 − 2 = 4 and 6 − 4 = 2 recover the inputs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped inspiratory-capacity library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_ic_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/inspiratory-capacity.adj")
        .canonicalize()
        .expect("shipped inspiratory-capacity.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ic_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_ic_lib()).unwrap();
    std::fs::write(dir.join("inspiratory-capacity.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// total_lung_capacity — the definition: the functional residual capacity plus the
// inspiratory capacity.
// ---------------------------------------------------------------------------

#[test]
fn imports_ic_library_and_computes_total_lung_capacity_with_citation() {
    let dir = scratch("tlc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"inspiratory-capacity.adj\"\n\
         observe functional_residual_capacity(2)\n\
         observe inspiratory_capacity(4)\n\
         ? total_lung_capacity(functional_residual_capacity, inspiratory_capacity)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 2 + 4 = 6.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"total_lung_capacity\"") && s.contains("\"value\":6"),
        "total_lung_capacity(2, 4) = 6: {s}"
    );
    // … AND the StatPearls/NCBI Bookshelf citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// inspiratory_capacity — the same relation solved for IC: TLC − FRC, which INVERTS the
// total lung capacity just produced.
// ---------------------------------------------------------------------------

#[test]
fn computes_inspiratory_capacity_from_tlc_and_frc_with_citation() {
    let dir = scratch("ic");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"inspiratory-capacity.adj\"\n\
         observe total_lung_capacity(6)\n\
         observe functional_residual_capacity(2)\n\
         ? inspiratory_capacity(total_lung_capacity, functional_residual_capacity)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 - 2 = 4, computed on the CPU.
    assert!(
        s.contains("\"name\":\"inspiratory_capacity\"") && s.contains("\"value\":4"),
        "inspiratory_capacity(6, 2) = 4: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "inspiratory_capacity carries its StatPearls citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// functional_residual_capacity — the same relation solved for FRC: TLC − IC, the third
// exact reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_functional_residual_capacity_from_tlc_and_ic_with_citation() {
    let dir = scratch("frc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"inspiratory-capacity.adj\"\n\
         observe total_lung_capacity(6)\n\
         observe inspiratory_capacity(4)\n\
         ? functional_residual_capacity(total_lung_capacity, inspiratory_capacity)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 6 - 4 = 2, computed on the CPU.
    assert!(
        s.contains("\"name\":\"functional_residual_capacity\"") && s.contains("\"value\":2"),
        "functional_residual_capacity(6, 4) = 2: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "functional_residual_capacity carries its StatPearls citation: {s}"
    );
}
