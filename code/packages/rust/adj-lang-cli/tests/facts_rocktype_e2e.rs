//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/rock-type.adj`) driven through the built CLI:
//! a native `table` naming the three basic rock-type classes and the
//! process by which each one forms, per three separate USGS "What are ___
//! rocks?" FAQ pages. The TWELFTH science slice in this loop's sweep. 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rocktype_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geology/rock-type.adj");
    std::fs::copy(&src, dir.join("rock-type.adj")).expect("copy shipped rock-type.adj");
}

#[test]
fn rock_type_recall_binds_the_formation_process_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rock-type.adj\"\n\
         ? rock_type(igneous, $Process)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Process\":\"crystallized_molten_rock\""),
        "igneous rocks form from crystallized molten rock: {out}"
    );
    assert!(
        out.contains("usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}

#[test]
fn rock_type_reverse_binds_the_rock_for_that_formation_process() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rock-type.adj\"\n\
         ? rock_type($R, heat_and_pressure_transformation)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"R\":\"metamorphic\""),
        "metamorphic rocks form through heat and pressure transformation: {out}"
    );
}

#[test]
fn rock_type_abstains_honestly_on_an_untabled_rock() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rock-type.adj\"\n\
         ? rock_type(coal, $Process)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "coal is a real rock but not one of the three rock-type classes tabled here -- honest abstention, never invented: {out}"
    );
}
