//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/circuit-parts.adj`) driven through the built CLI:
//! a native `table` of basic circuit part → the role its source states resolves
//! a binding-query recall with the MIT K-12 Maker citation, runs backward
//! (role → part), and abstains on something that is not one of the basic parts
//! (a capacitor) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factscp_{tag}_{}", std::process::id()));
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
fn physics_circuit_parts_recall_binds_role_with_citation() {
    let dir = scratch("circuitparts");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/circuit-parts.adj");
    std::fs::copy(&src, dir.join("circuit-parts.adj")).expect("copy shipped circuit-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"circuit-parts.adj\"\n\
         ? circuit_part_role(battery, $Role)\n\
         ? circuit_part_role(switch, $Role)\n\
         ? circuit_part_role(resistor, $Role)\n\
         ? circuit_part_role($Part, carries_current)\n\
         ? circuit_part_role(capacitor, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each part to the role the MIT handout states.
    assert!(
        out.contains("\"Role\":\"provides_dc_power\""),
        "battery → provides_dc_power: {out}"
    );
    assert!(
        out.contains("\"Role\":\"opens_or_closes\""),
        "switch → opens_or_closes: {out}"
    );
    assert!(
        out.contains("\"Role\":\"slows_current\""),
        "resistor → slows_current: {out}"
    );
    assert!(
        out.contains("circuit_part_role(battery, provides_dc_power)"),
        "battery is governing-bound to provides_dc_power: {out}"
    );
    // The relation runs BACKWARD: bind the role `carries_current`, recall the part.
    assert!(
        out.contains("\"Part\":\"wire\""),
        "carries_current → wire (reverse recall): {out}"
    );
    // The answer carries the MIT K-12 Maker locator + trust tier as its proof.
    assert!(
        out.contains("k12maker.mit.edu") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // A capacitor is NOT one of the basic parts in this table — honest abstention.
    assert!(out.contains("\"abstained\":true"), "capacitor abstains: {out}");
}
