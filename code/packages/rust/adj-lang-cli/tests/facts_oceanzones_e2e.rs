//! End-to-end test for the oceanography FACTS library
//! (`adj-facts-stdlib/oceanography/ocean-zones.adj`) driven through the
//! built CLI: a native `table` naming each of the ocean's first three
//! depth zones as a number, quoted verbatim from the Woods Hole
//! Oceanographic Institution's "Ocean Zones" page -- a sibling library to
//! the already-shipped `plant-life-cycle.adj`/`frog-life-cycle.adj`,
//! applying the SAME plain numbered ordered-sequence recall shape to a
//! different domain. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_oceanzones_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("oceanography/ocean-zones.adj");
    std::fs::copy(&src, dir.join("ocean-zones.adj"))
        .expect("copy shipped ocean-zones.adj");
}

#[test]
fn ocean_zone_recall_binds_the_order_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-zones.adj\"\n\
         ? ocean_zone(twilight_zone, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"N\":\"2\""),
        "the twilight zone is the second depth zone: {out}"
    );
    assert!(
        out.contains("whoi.edu") && out.contains("\"trust\":\"consensus\""),
        "carries the WHOI citation: {out}"
    );
}

#[test]
fn ocean_zone_reverse_binds_the_zone_for_that_order() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-zones.adj\"\n\
         ? ocean_zone($Z, 1)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Z\":\"sunlight_zone\""),
        "the sunlight zone is the shallowest, first depth zone: {out}"
    );
}

#[test]
fn ocean_zone_abstains_honestly_on_an_untabled_zone_name() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-zones.adj\"\n\
         ? ocean_zone(abyssal_zone, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"abyssal_zone\" is a real deeper zone the source names but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
