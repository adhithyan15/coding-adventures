//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/measuring-tool-si-unit.adj`) driven through
//! the built CLI: a `rule` composing the already-shipped `measuring_tool`
//! table (`chemistry/measuring-tools.adj`) with the already-shipped
//! `si_base_unit` table (`metrology/si-base-units.adj`, a CROSS-DIRECTORY
//! import via `../metrology/si-base-units.adj`, the same shape
//! `earth-layer-matter-behavior.adj` already established) to DERIVE
//! `measuring_tool_si_unit($Tool, $Unit, $Symbol)` -- the FOURTH `rule`-based
//! CAUSAL-COMPOSITION fact in this loop's science curriculum sweep, mirroring
//! the discipline `heat-causes-phase-change.adj`, `force-causes-
//! acceleration.adj`, and `earth-layer-matter-behavior.adj` already
//! established, applied here to the "observation and measurement" gap (ADJ-
//! STDLIB-COVERAGE.md 5.1). 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_measuringtoolsiunit_{tag}_{}", std::process::id()));
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

/// Copy BOTH shipped libraries, preserving their real relative directory
/// structure: `measuring-tool-si-unit.adj` (in `chemistry/`) imports
/// `measuring-tools.adj` (same dir) and `../metrology/si-base-units.adj`
/// (cross-directory), so the entry program must sit at a root that contains
/// both subtrees.
fn place_libs(dir: &Path) {
    let src = facts_stdlib();
    for (rel_src, rel_dst) in [
        ("chemistry/measuring-tools.adj", "chemistry/measuring-tools.adj"),
        (
            "chemistry/measuring-tool-si-unit.adj",
            "chemistry/measuring-tool-si-unit.adj",
        ),
        (
            "metrology/si-base-units.adj",
            "metrology/si-base-units.adj",
        ),
    ] {
        let dst = dir.join(rel_dst);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src.join(rel_src), &dst)
            .unwrap_or_else(|e| panic!("copy shipped {rel_src}: {e}"));
    }
}

#[test]
fn thermometer_derives_kelvin_with_dual_citations() {
    let dir = scratch("thermometer");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chemistry/measuring-tool-si-unit.adj\"\n\
         ? measuring_tool_si_unit(thermometer, $U, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"U\":\"kelvin\"") && out.contains("\"S\":\"\\\"K\\\"\""),
        "a thermometer measures temperature, whose SI base unit is kelvin (\"K\"): {out}"
    );
    // The derivation composes citations from BOTH sibling libraries:
    // Chemistry LibreTexts (measuring_tool, chemistry) AND NIST
    // (si_base_unit, metrology).
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the derived fact is DERIVED, not a direct table row -- both a rule step and fact steps appear: {out}"
    );
    assert!(
        out.contains("chem.libretexts.org") && out.contains("nist.gov"),
        "carries citations from BOTH composed libraries (measuring-tools.adj and si-base-units.adj): {out}"
    );
}

#[test]
fn kelvin_reverse_binds_to_thermometer() {
    let dir = scratch("reverse");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chemistry/measuring-tool-si-unit.adj\"\n\
         ? measuring_tool_si_unit($T, kelvin, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"thermometer\""),
        "the only tool whose SI base unit is kelvin is the thermometer: {out}"
    );
}

#[test]
fn graduated_cylinder_abstains_honestly_as_volume_is_a_derived_not_base_unit() {
    let dir = scratch("abstain");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chemistry/measuring-tool-si-unit.adj\"\n\
         ? measuring_tool_si_unit(graduated_cylinder, $U, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a graduated cylinder measures volume, a DERIVED SI quantity (m3), not one of the 7 BASE quantities si_base_unit tables -- honest abstention, never invented: {out}"
    );
}
