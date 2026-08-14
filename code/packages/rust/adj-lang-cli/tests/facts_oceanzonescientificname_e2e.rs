//! End-to-end test for the oceanography FACTS library
//! (`adj-facts-stdlib/oceanography/ocean-zone-scientific-name.adj`) driven
//! through the built CLI: a native `table` naming the midnight zone's
//! alternate scientific name (bathypelagic), decoded from a span already
//! sitting unused inside the SAME WHOI "Ocean Zones" quote `ocean-zones.adj`
//! and `ocean-zone-depth.adj` already cite -- a sibling to both. Resolves
//! binding-query recall (both directions) with the source's citation, and
//! abstains on a zone (sunlight_zone) whose cited span names no alternate
//! scientific name -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_oceanzonescientificname_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("oceanography/ocean-zone-scientific-name.adj");
    std::fs::copy(&src, dir.join("ocean-zone-scientific-name.adj"))
        .expect("copy shipped ocean-zone-scientific-name.adj");
}

#[test]
fn ocean_zone_scientific_name_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-zone-scientific-name.adj\"\n\
         ? ocean_zone_scientific_name(midnight_zone, $ScientificName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"ocean_zone_scientific_name(midnight_zone, bathypelagic)\""),
        "the midnight zone is also called bathypelagic: {out}"
    );
    assert!(
        out.contains("whoi.edu") && out.contains("\"trust\":\"consensus\""),
        "carries the WHOI citation: {out}"
    );
}

#[test]
fn ocean_zone_scientific_name_recalls_backward_from_a_bound_name() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-zone-scientific-name.adj\"\n\
         ? ocean_zone_scientific_name($Zone, bathypelagic)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"ocean_zone_scientific_name(midnight_zone, bathypelagic)\""),
        "bathypelagic names the midnight zone: {out}"
    );
}

#[test]
fn ocean_zone_scientific_name_abstains_honestly_on_the_sunlight_zone() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-zone-scientific-name.adj\"\n\
         ? ocean_zone_scientific_name(sunlight_zone, $ScientificName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the cited sunlight-zone span names no alternate scientific name -- honest abstention: {out}"
    );
}
