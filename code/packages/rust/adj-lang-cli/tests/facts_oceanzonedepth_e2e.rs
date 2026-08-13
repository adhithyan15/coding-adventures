//! End-to-end test for the oceanography FACTS library
//! (`adj-facts-stdlib/oceanography/ocean-zone-depth.adj`) driven through
//! the built CLI: a native `table` naming the approximate depth in meters
//! each of the ocean's first three depth zones reaches down to, where the
//! source states one -- a sibling to the already-shipped `ocean-zones.adj`
//! (which only carries ONE ordinal position per zone), decoding spans
//! already sitting unused inside that table's own header. Resolves
//! binding-query recall (both directions) with the source's citation, and
//! abstains on a zone (abyssal_zone) not among the three tabled here --
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_oceanzonedepth_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("oceanography/ocean-zone-depth.adj");
    std::fs::copy(&src, dir.join("ocean-zone-depth.adj"))
        .expect("copy shipped ocean-zone-depth.adj");
}

#[test]
fn ocean_zone_depth_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-zone-depth.adj\"\n\
         ? ocean_zone_depth(sunlight_zone, $MaxDepthMeters)\n\
         ? ocean_zone_depth(twilight_zone, $MaxDepthMeters)\n\
         ? ocean_zone_depth(midnight_zone, $MaxDepthMeters)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"ocean_zone_depth(sunlight_zone, 200)\""),
        "sunlight zone reaches 200m: {out}"
    );
    assert!(
        out.contains("\"term\":\"ocean_zone_depth(twilight_zone, 1000)\""),
        "twilight zone reaches 1000m: {out}"
    );
    assert!(
        out.contains("\"term\":\"ocean_zone_depth(midnight_zone, 4000)\""),
        "midnight zone reaches 4000m: {out}"
    );
    assert!(
        out.contains("whoi.edu") && out.contains("\"trust\":\"consensus\""),
        "carries the WHOI citation: {out}"
    );
}

#[test]
fn ocean_zone_depth_recalls_backward_from_a_bound_depth() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-zone-depth.adj\"\n\
         ? ocean_zone_depth($Zone, 4000)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"ocean_zone_depth(midnight_zone, 4000)\""),
        "4000m names the midnight zone: {out}"
    );
}

#[test]
fn ocean_zone_depth_abstains_honestly_on_the_abyssal_zone() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-zone-depth.adj\"\n\
         ? ocean_zone_depth(abyssal_zone, $MaxDepthMeters)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the abyssal zone is a real deeper zone the source names, but not one of the three tabled here -- honest abstention: {out}"
    );
}
