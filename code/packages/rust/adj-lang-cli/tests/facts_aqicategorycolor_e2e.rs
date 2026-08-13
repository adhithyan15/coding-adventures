//! End-to-end test for the environment FACTS library
//! (`adj-facts-stdlib/environment/aqi-category-color.adj`) driven through
//! the built CLI: a native `table` naming the color the SAME AirNow source
//! spans already state for each AQI category -- a sibling to the
//! already-shipped `air-quality-index.adj` (which only carries each
//! band's numeric breakpoint and category), decoding the color half of
//! spans already sitting unused inside that table's own per-row
//! provenance block. Resolves binding-query recall (both directions) with
//! the source's citation, and covers the full category domain with no
//! abstention -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_aqicategorycolor_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("environment/aqi-category-color.adj");
    std::fs::copy(&src, dir.join("aqi-category-color.adj"))
        .expect("copy shipped aqi-category-color.adj");
}

#[test]
fn aqi_category_color_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"aqi-category-color.adj\"\n\
         ? aqi_category_color(hazardous, $Color)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"aqi_category_color(hazardous, maroon)\""),
        "hazardous is maroon: {out}"
    );
    assert!(
        out.contains("airnow.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the AirNow citation: {out}"
    );
}

#[test]
fn aqi_category_color_recalls_backward_from_a_bound_color() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"aqi-category-color.adj\"\n\
         ? aqi_category_color($Category, orange)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"aqi_category_color(unhealthy_for_sensitive_groups, orange)\""),
        "orange names the sensitive-groups category: {out}"
    );
}

#[test]
fn aqi_category_color_covers_the_full_domain_without_abstention() {
    let dir = scratch("full");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"aqi-category-color.adj\"\n\
         ? aqi_category_color(good, $Color)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"aqi_category_color(good, green)\""),
        "good is green: {out}"
    );
    assert!(
        !out.contains("\"abstained\":true"),
        "every category has a color -- no abstention expected: {out}"
    );
}
