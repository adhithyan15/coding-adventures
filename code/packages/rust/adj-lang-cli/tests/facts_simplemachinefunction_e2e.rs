//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/simple-machine-function.adj`) driven through
//! the built CLI: a native `table` recording, for each of the six classical
//! simple machines, the FUNCTION clause the same already-cited NASA
//! sentence states before its example -- a sibling decoding the leading
//! function clause of each already-verified quote used by
//! `simple-machines.adj` and `simple-machine-alt-example.adj`. Resolves
//! forward and backward recall queries with the source's citation, plus
//! honest abstention on a non-simple-machine word -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_simplemachinefunction_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/simple-machine-function.adj");
    std::fs::copy(&src, dir.join("simple-machine-function.adj"))
        .expect("copy shipped simple-machine-function.adj");
}

#[test]
fn simple_machine_function_recalls_lever_function_with_citation() {
    let dir = scratch("lever");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simple-machine-function.adj\"\n\
         ? simple_machine_function(lever, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"simple_machine_function(lever, moves_object_over_fulcrum)\""),
        "lever should recall its cited function: {out}"
    );
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn simple_machine_function_backward_recalls_screw_for_fastening() {
    let dir = scratch("screw");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simple-machine-function.adj\"\n\
         ? simple_machine_function($M, fastens_objects_together)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"simple_machine_function(screw, fastens_objects_together)\""),
        "screw should be the only recalled fastening machine: {out}"
    );
    assert!(
        !out.contains("simple_machine_function(wedge, fastens_objects_together)"),
        "wedge's function is splitting, not fastening: {out}"
    );
}

#[test]
fn simple_machine_function_abstains_on_gear() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simple-machine-function.adj\"\n\
         ? simple_machine_function(gear, $FunctionGear)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a gear is not one of the six classical simple machines -- honest abstention expected: {out}"
    );
}
