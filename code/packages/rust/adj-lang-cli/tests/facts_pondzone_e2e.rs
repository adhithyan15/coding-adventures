//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/pond-zone.adj`) driven through the built
//! CLI: a native `table` naming three zones of a freshwater lake or pond
//! and what each actually is, quoted verbatim from three separate
//! Wikipedia articles. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_pond_zone_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/pond-zone.adj");
    std::fs::copy(&src, dir.join("pond-zone.adj")).expect("copy shipped pond-zone.adj");
}

#[test]
fn pond_zone_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pond-zone.adj\"\n\
         ? pond_zone(littoral_zone, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"close_to_the_shore\""),
        "littoral_zone means close_to_the_shore: {out}"
    );
    assert!(
        out.contains("en.wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wikipedia citation: {out}"
    );
}

#[test]
fn pond_zone_reverse_binds_the_zone_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pond-zone.adj\"\n\
         ? pond_zone($Z, deep_zone_located_below_the_range_of_effective_light_penetration)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Z\":\"profundal_zone\""),
        "the shipped deep_zone_located_below_the_range_of_effective_light_penetration example is profundal_zone: {out}"
    );
}

#[test]
fn pond_zone_abstains_honestly_on_an_untabled_zone() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pond-zone.adj\"\n\
         ? pond_zone(benthic_zone, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "benthic_zone is a real freshwater-zone term, but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
