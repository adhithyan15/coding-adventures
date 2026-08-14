//! End-to-end test for the oceanography FACTS library
//! (`adj-facts-stdlib/oceanography/ocean-current-surface-position.adj`)
//! driven through the built CLI: a native `table` naming WHERE wind-driven
//! currents occur (at or near the ocean's surface), decoded from a span
//! already sitting unused inside the SAME NOAA "What is a current?" quote
//! `ocean-current-drivers.adj` already cites -- a sibling to that table.
//! Resolves binding-query recall (both directions) with the source's
//! citation, and abstains on a current type (thermohaline_circulation)
//! whose cited span names no position -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_oceancurrentsurfaceposition_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("oceanography/ocean-current-surface-position.adj");
    std::fs::copy(&src, dir.join("ocean-current-surface-position.adj"))
        .expect("copy shipped ocean-current-surface-position.adj");
}

#[test]
fn ocean_current_surface_position_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-current-surface-position.adj\"\n\
         ? ocean_current_surface_position(wind_driven_currents, $Position)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"ocean_current_surface_position(wind_driven_currents, at_or_near_the_oceans_surface)\""),
        "wind-driven currents occur at or near the surface: {out}"
    );
    assert!(
        out.contains("oceanservice.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA citation: {out}"
    );
}

#[test]
fn ocean_current_surface_position_recalls_backward_from_a_bound_position() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-current-surface-position.adj\"\n\
         ? ocean_current_surface_position($CurrentType, at_or_near_the_oceans_surface)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"ocean_current_surface_position(wind_driven_currents, at_or_near_the_oceans_surface)\""),
        "the surface position names wind-driven currents: {out}"
    );
}

#[test]
fn ocean_current_surface_position_abstains_honestly_on_thermohaline_circulation() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-current-surface-position.adj\"\n\
         ? ocean_current_surface_position(thermohaline_circulation, $Position)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the cited thermohaline-circulation span names no position -- honest abstention: {out}"
    );
}
