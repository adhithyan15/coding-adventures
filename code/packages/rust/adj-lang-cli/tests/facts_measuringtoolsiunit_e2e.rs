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

// THE WARRANT IS PINNED VIA `steps`, NOT `citations` -- AND THE FIRST ATTEMPT
// AT THIS PIN WAS WRONG IN AN INSTRUCTIVE WAY.
//
// installment 4a re-grounded this rule's `source` on the page's whole sentence.
// A citation-only pin was written first, and the RESTORE-THE-FRAGMENT mutation
// came back GREEN -- it did not fail when the value was un-repaired. I recorded
// that as "the two citations are indistinguishable, so no pin is possible",
// because this rule imports ../metrology/si-base-units.adj whose `source`,
// `locator` and `trust` are byte-identical to this rule's warrant.
//
// THE BYTE-IDENTITY IS REAL; THE CONCLUSION WAS NOT. Review disproved it against
// the binary. `citations` carries LEAF-FACT citations only -- the rule's own
// warrant never appears there at all, which is why the citation-only pin could
// not see the mutation. The warrant IS emitted in `steps`, tagged
// "kind":"rule" with the rule's own goal, while the imported table's identical
// sentence appears as "kind":"fact" with goal si_base_unit(...). The two are
// perfectly distinguishable there.
//
// This library's own header says so at :39-43 ("A query's `steps` trail
// therefore shows THREE entries..."). The excuse contradicted the file it was
// written in.
const MEASURING_TOOL_SI_UNIT_WARRANT_PIN: &str = r#"{"kind":"rule","step":0,"depth":0,"goal":"measuring_tool_si_unit(thermometer, U, S)","source":"The SI is made up of 7 base units that define the 22 derived units with special names and symbols, which are illustrated in NIST SP 1247, SI Base Units Relationship Poster.","locator":"https://www.nist.gov/pml/owm/metric-si/si-units","trust":"authoritative""#;

#[test]
fn measuring_tool_si_unit_warrant_is_the_pages_whole_sentence() {
    let dir = scratch("reground");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"chemistry/measuring-tool-si-unit.adj\"
         ? measuring_tool_si_unit(thermometer, $U, $S)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Anchored on the RULE step -- kind, goal, source, locator and trust in one
    // contiguous span -- so the imported table's identical sentence cannot
    // satisfy it. Restoring the fragment reddens this; it did not redden the
    // citation-only pin.
    assert!(
        out.contains(MEASURING_TOOL_SI_UNIT_WARRANT_PIN),
        "the rule's own warrant is the page's whole sentence: {out}"
    );
}
