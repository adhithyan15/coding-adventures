//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/heat-causes-phase-change.adj`) driven through
//! the built CLI: a native `table` naming which direction heat flows for four
//! everyday phase changes, composed via a `rule` with the ALREADY-SHIPPED
//! `phase_change_name` table (from the sibling `states-of-matter.adj`) to
//! DERIVE `causes_phase_change($Direction, $Name)` — the first `rule`-based
//! CAUSAL-EXPLANATION fact in this loop's science curriculum sweep, mirroring
//! the discipline `shape-composition.adj` and `word-families.adj` already
//! established. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_heatphase_{tag}_{}", std::process::id()));
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

/// Copy BOTH shipped libraries beside the entry program: `heat-causes-phase-
/// change.adj` transitively imports its sibling `states-of-matter.adj`, so
/// the CLI's sandbox-checked relative import needs both present.
fn place_libs(dir: &Path) {
    let physics = facts_stdlib().join("physics");
    std::fs::copy(
        physics.join("states-of-matter.adj"),
        dir.join("states-of-matter.adj"),
    )
    .expect("copy shipped physics/states-of-matter.adj");
    std::fs::copy(
        physics.join("heat-causes-phase-change.adj"),
        dir.join("heat-causes-phase-change.adj"),
    )
    .expect("copy shipped physics/heat-causes-phase-change.adj");
}

#[test]
fn heating_derives_melting_and_vaporization_with_dual_citations() {
    let dir = scratch("heating");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heat-causes-phase-change.adj\"\n\
         ? causes_phase_change(heating, $Name)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"Name\":\"melting\""), "heating causes melting: {out}");
    assert!(
        out.contains("\"Name\":\"vaporization\""),
        "heating causes vaporization: {out}"
    );
    // The derivation composes TWO citations: the rule's general heat-direction
    // principle AND the sibling table's specific phase-change-name fact.
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the causal fact is DERIVED, not a direct table row -- both a rule step and fact steps appear: {out}"
    );
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the LibreTexts citation: {out}"
    );
}

#[test]
fn cooling_derives_freezing_and_condensation() {
    let dir = scratch("cooling");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heat-causes-phase-change.adj\"\n\
         ? causes_phase_change(cooling, $Name)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"Name\":\"freezing\""), "cooling causes freezing: {out}");
    assert!(
        out.contains("\"Name\":\"condensation\""),
        "cooling causes condensation: {out}"
    );
}
