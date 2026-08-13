//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/simple-machine-alt-example.adj`) driven
//! through the built CLI: a native `table` naming the SECOND everyday
//! example NASA's source states for a simple machine, where the source
//! names two -- a sibling to the already-shipped `simple-machines.adj`
//! (which only carries the FIRST everyday example per machine), decoding
//! spans already sitting unused inside that table's own header and
//! provenance block. Resolves binding-query recall (both directions) with
//! the source's citation, and abstains on a machine (screw) the cited spans
//! give no distinct second example for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_simplemachinealtexample_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/simple-machine-alt-example.adj");
    std::fs::copy(&src, dir.join("simple-machine-alt-example.adj"))
        .expect("copy shipped simple-machine-alt-example.adj");
}

#[test]
fn simple_machine_alt_example_recalls_all_five_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simple-machine-alt-example.adj\"\n\
         ? simple_machine_alt_example(lever, $AltEx)\n\
         ? simple_machine_alt_example(inclined_plane, $AltEx)\n\
         ? simple_machine_alt_example(wedge, $AltEx)\n\
         ? simple_machine_alt_example(wheel_and_axle, $AltEx)\n\
         ? simple_machine_alt_example(pulley, $AltEx)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"simple_machine_alt_example(lever, scissors)\""),
        "lever's alt example is scissors: {out}"
    );
    assert!(
        out.contains("\"term\":\"simple_machine_alt_example(inclined_plane, stairs)\""),
        "inclined_plane's alt example is stairs: {out}"
    );
    assert!(
        out.contains("\"term\":\"simple_machine_alt_example(wedge, knife)\""),
        "wedge's alt example is knife: {out}"
    );
    assert!(
        out.contains("\"term\":\"simple_machine_alt_example(wheel_and_axle, clock)\""),
        "wheel_and_axle's alt example is clock: {out}"
    );
    assert!(
        out.contains("\"term\":\"simple_machine_alt_example(pulley, water_well)\""),
        "pulley's alt example is water_well: {out}"
    );
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn simple_machine_alt_example_recalls_backward_from_a_bound_alt_example() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simple-machine-alt-example.adj\"\n\
         ? simple_machine_alt_example($M, stairs)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"simple_machine_alt_example(inclined_plane, stairs)\""),
        "stairs names inclined_plane: {out}"
    );
}

#[test]
fn simple_machine_alt_example_abstains_honestly_on_screw() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simple-machine-alt-example.adj\"\n\
         ? simple_machine_alt_example(screw, $AltEx)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "screw has no distinct second example in the cited spans -- honest abstention: {out}"
    );
}
