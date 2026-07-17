//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/states-of-matter.adj`) driven through the built
//! CLI: a native `table` of phase-change direction -> its name resolves a
//! binding-query recall with the source's citation, runs BACKWARD (bind the
//! name, recall the direction), and abstains on a direction not in the table —
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_physsom_{tag}_{}", std::process::id()));
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

#[test]
fn physics_phase_change_recall_binds_name_with_citation_and_abstains() {
    let dir = scratch("statesofmatter");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/states-of-matter.adj");
    std::fs::copy(&src, dir.join("states-of-matter.adj"))
        .expect("copy shipped physics states-of-matter.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"states-of-matter.adj\"\n\
         ? phase_change_name(solid_to_liquid, $Name)\n\
         ? phase_change_name(gas_to_liquid, $Name)\n\
         ? phase_change_name($Change, sublimation)\n\
         ? phase_change_name(solid_to_plasma, $Name)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A solid becoming a liquid is melting; a gas becoming a liquid is condensation.
    assert!(out.contains("\"Name\":\"melting\""), "solid_to_liquid -> melting: {out}");
    assert!(out.contains("\"Name\":\"condensation\""), "gas_to_liquid -> condensation: {out}");
    // The query runs backward: bind the name, recall the direction.
    assert!(
        out.contains("\"Change\":\"solid_to_gas\""),
        "sublimation is the solid_to_gas change (reverse recall): {out}"
    );
    // The answer carries the LibreTexts citation as its proof.
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // solid_to_plasma is not one of the six phase changes — honest abstention.
    assert!(out.contains("\"abstained\":true"), "solid_to_plasma abstains: {out}");
}
