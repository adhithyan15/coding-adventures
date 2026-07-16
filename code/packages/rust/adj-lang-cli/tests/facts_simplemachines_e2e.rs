//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/simple-machines.adj`) driven through the built CLI:
//! a native `table` of simple-machine → everyday example resolves a binding
//! query recall with the NASA citation, runs backward (example → machine), and
//! abstains on something that is not one of the six simple machines — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factssm_{tag}_{}", std::process::id()));
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
fn physics_simple_machines_recall_binds_example_with_citation() {
    let dir = scratch("simplemachines");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/simple-machines.adj");
    std::fs::copy(&src, dir.join("simple-machines.adj")).expect("copy shipped simple-machines.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"simple-machines.adj\"\n\
         ? simple_machine_example(lever, $Ex)\n\
         ? simple_machine_example(inclined_plane, $Ex)\n\
         ? simple_machine_example(pulley, $Ex)\n\
         ? simple_machine_example($M, seesaw)\n\
         ? simple_machine_example(gear, $Ex)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each machine to the everyday example NASA names.
    assert!(out.contains("\"Ex\":\"seesaw\""), "lever binds to seesaw: {out}");
    assert!(
        out.contains("simple_machine_example(lever, seesaw)"),
        "lever is governing-bound to seesaw: {out}"
    );
    assert!(
        out.contains("simple_machine_example(inclined_plane, ramp)"),
        "inclined_plane is governing-bound to ramp: {out}"
    );
    assert!(
        out.contains("simple_machine_example(pulley, elevator)"),
        "pulley is governing-bound to elevator: {out}"
    );
    // The relation runs BACKWARD: bind the example, recall the machine.
    assert!(
        out.contains("simple_machine_example(lever, seesaw)"),
        "reverse recall binds M=lever from seesaw: {out}"
    );
    // The answer carries the NASA locator + trust tier as its proof.
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A gear is NOT one of the six simple machines — honest abstention.
    assert!(out.contains("\"abstained\":true"), "gear abstains: {out}");
}
