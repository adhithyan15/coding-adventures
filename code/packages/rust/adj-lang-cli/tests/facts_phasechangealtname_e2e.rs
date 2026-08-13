//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/phase-change-alt-name.adj`) driven through
//! the built CLI: a native `table` naming the older/alternate word the
//! source states for a phase-change direction, where the source states one
//! -- a sibling to the already-shipped `states-of-matter.adj` (which only
//! carries ONE primary name per direction), decoding spans already sitting
//! unused inside that table's own header and provenance block. Resolves
//! binding-query recall (both directions) with the source's citation, and
//! abstains on a direction (gas_to_liquid) the cited spans give no
//! alternate name for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_phasechangealtname_{tag}_{}", std::process::id()));
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

fn place_lib(dir: &Path) {
    let src = facts_stdlib().join("physics/phase-change-alt-name.adj");
    std::fs::copy(&src, dir.join("phase-change-alt-name.adj"))
        .expect("copy shipped phase-change-alt-name.adj");
}

#[test]
fn phase_change_alt_name_recalls_all_three_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phase-change-alt-name.adj\"\n\
         ? phase_change_alt_name(solid_to_liquid, $AltName)\n\
         ? phase_change_alt_name(liquid_to_solid, $AltName)\n\
         ? phase_change_alt_name(liquid_to_gas, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"phase_change_alt_name(solid_to_liquid, fusion)\""),
        "melting's alt name is fusion: {out}"
    );
    assert!(
        out.contains("\"term\":\"phase_change_alt_name(liquid_to_solid, solidification)\""),
        "freezing's alt name is solidification: {out}"
    );
    assert!(
        out.contains("\"term\":\"phase_change_alt_name(liquid_to_gas, boiling)\""),
        "vaporization's alt name is boiling: {out}"
    );
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the LibreTexts citation: {out}"
    );
}

#[test]
fn phase_change_alt_name_recalls_backward_from_a_bound_alt_name() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phase-change-alt-name.adj\"\n\
         ? phase_change_alt_name($Change, boiling)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"phase_change_alt_name(liquid_to_gas, boiling)\""),
        "boiling names liquid_to_gas: {out}"
    );
}

#[test]
fn phase_change_alt_name_abstains_honestly_on_condensation() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phase-change-alt-name.adj\"\n\
         ? phase_change_alt_name(gas_to_liquid, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "condensation's cited span gives no alternate name -- honest abstention: {out}"
    );
}
