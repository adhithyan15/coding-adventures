//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/igneous-rock-type.adj`) driven through the
//! built CLI: a native `table` naming the two broad types of igneous rock
//! and what actually defines each, quoted verbatim from the U.S. National
//! Park Service's "Igneous Rocks" geology page. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_igneous_rock_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geology/igneous-rock-type.adj");
    std::fs::copy(&src, dir.join("igneous-rock-type.adj")).expect("copy shipped igneous-rock-type.adj");
}

#[test]
fn igneous_rock_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"igneous-rock-type.adj\"\n\
         ? igneous_rock_type(intrusive, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"solidifies_within_earth\""),
        "intrusive means solidifies_within_earth: {out}"
    );
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn igneous_rock_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"igneous-rock-type.adj\"\n\
         ? igneous_rock_type($T, erupted_onto_the_surface_or_into_the_atmosphere)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"extrusive\""),
        "the shipped erupted_onto_the_surface_or_into_the_atmosphere example is extrusive: {out}"
    );
}

#[test]
fn igneous_rock_type_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"igneous-rock-type.adj\"\n\
         ? igneous_rock_type(hypabyssal, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "hypabyssal is a real geological term for shallow-depth cooling, but not one of the two broad types this source names -- honest abstention, never invented: {out}"
    );
}
