//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/heat-transfer.adj`) driven through the built CLI:
//! a native `table` of the three modes of thermal-energy transfer -> the
//! mechanism the source states each moves heat by resolves binding-query recalls
//! (forward AND backward) with the source's NASA Next Gen STEM citation, and
//! abstains on a word that is not one of the three modes (evaporation) — 0 model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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
fn physics_heat_transfer_recall_binds_mechanism_with_citation() {
    let dir = scratch("heattransfer");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/heat-transfer.adj");
    std::fs::copy(&src, dir.join("heat-transfer.adj")).expect("copy shipped heat-transfer.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"heat-transfer.adj\"\n\
         ? heat_transfer_mode(conduction, $Mechanism)\n\
         ? heat_transfer_mode(convection, $Mechanism)\n\
         ? heat_transfer_mode(radiation, $Mechanism)\n\
         ? heat_transfer_mode($Mode, light_waves)\n\
         ? heat_transfer_mode(evaporation, $Mechanism)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Conduction moves heat by direct contact, convection by the motion of
    // gasses and liquids, radiation by light waves — the recalled mechanisms
    // (forward binds).
    assert!(
        out.contains("\"Mechanism\":\"direct_contact\""),
        "conduction → direct_contact: {out}"
    );
    assert!(
        out.contains("\"Mechanism\":\"motion_of_gasses_and_liquids\""),
        "convection → motion_of_gasses_and_liquids: {out}"
    );
    assert!(
        out.contains("\"Mechanism\":\"light_waves\""),
        "radiation → light_waves: {out}"
    );
    // The relation runs BACKWARD: bind the mechanism `light_waves`, recall its
    // mode.
    assert!(
        out.contains("\"Mode\":\"radiation\""),
        "light_waves → radiation (reverse recall): {out}"
    );
    // The answer carries the NASA Next Gen STEM citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Evaporation is a phase change, not one of the three modes of heat
    // transfer — honest abstention, never a fabricated mechanism.
    assert!(
        out.contains("\"abstained\":true"),
        "evaporation abstains: {out}"
    );
}
