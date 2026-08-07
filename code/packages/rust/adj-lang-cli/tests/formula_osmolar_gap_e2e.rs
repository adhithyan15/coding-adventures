//! End-to-end tests for the `clinical/osmolar-gap.adj` library — the definition of the osmol gap
//! (measured osmolality − calculated osmolality) and its two exact rearrangements — driven through
//! the built CLI binary against the SHIPPED stdlib. The same invariant as every other formula
//! library: a consumer states NO arithmetic; it imports the grounded library, binds the measured
//! osmolalities with `observe`, and the engine applies the cited relation on the CPU, computing the
//! EXACT value and rendering the relation's citation and trust tier in the `derived` section (the
//! auditable answer). The three formulas INVERT around the worked case measured = 290 mOsm/kg,
//! calculated = 280 mOsm/kg: 290 − 280 = 10, 10 + 280 = 290, and 290 − 10 = 280. The three asserted
//! values (10, 290, 280) are chosen so none is a colon-anchored prefix of another rendered value.
//! This is the osmolality (difference-type) cousin of the shipped anion-gap.adj.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped osmolar-gap library, resolved from this crate's manifest dir so the
/// test is location-independent.
fn shipped_og_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/clinical/osmolar-gap.adj")
        .canonicalize()
        .expect("shipped osmolar-gap.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_og_{tag}_{}", std::process::id()));
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
    let lib = std::fs::read_to_string(shipped_og_lib()).unwrap();
    std::fs::write(dir.join("osmolar-gap.adj"), lib).unwrap();
}

// ---------------------------------------------------------------------------
// osmol_gap — the definition: measured osmolality minus calculated osmolality.
// ---------------------------------------------------------------------------

#[test]
fn imports_osmolar_gap_library_and_computes_it_with_citation() {
    let dir = scratch("og");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"osmolar-gap.adj\"\n\
         observe measured_osmolality(290)\n\
         observe calculated_osmolality(280)\n\
         ? osmol_gap(measured_osmolality, calculated_osmolality)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // The derived value section carries the applied relation's result: 290 − 280 = 10.
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"osmol_gap\"") && s.contains("\"value\":10"),
        "osmol_gap(290, 280) = 10: {s}"
    );
    // … AND the article citation and trust tier, so the answer is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "applied relation carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// measured_osmolality — the same relation solved for the measured value: gap + calculated.
// ---------------------------------------------------------------------------

#[test]
fn computes_measured_from_gap_and_calculated_with_citation() {
    let dir = scratch("meas");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"osmolar-gap.adj\"\n\
         observe osmol_gap(10)\n\
         observe calculated_osmolality(280)\n\
         ? measured_osmolality(osmol_gap, calculated_osmolality)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 10 + 280 = 290, computed on the CPU.
    assert!(
        s.contains("\"name\":\"measured_osmolality\"") && s.contains("\"value\":290"),
        "measured_osmolality(10, 280) = 290: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "measured_osmolality carries its cited provenance: {s}"
    );
}

// ---------------------------------------------------------------------------
// calculated_osmolality — the same relation solved for the calculated value: measured − gap, the
// third reading of the one definition.
// ---------------------------------------------------------------------------

#[test]
fn computes_calculated_from_measured_and_gap_with_citation() {
    let dir = scratch("calc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"osmolar-gap.adj\"\n\
         observe measured_osmolality(290)\n\
         observe osmol_gap(10)\n\
         ? calculated_osmolality(measured_osmolality, osmol_gap)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 290 − 10 = 280, computed on the CPU.
    assert!(
        s.contains("\"name\":\"calculated_osmolality\"") && s.contains("\"value\":280"),
        "calculated_osmolality(290, 10) = 280: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("ncbi.nlm.nih.gov"),
        "calculated_osmolality carries its cited provenance: {s}"
    );
}
