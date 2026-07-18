//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/earth-layers.adj`) driven through the built CLI:
//! a native `table` of Earth's four internal layers → their physical state
//! resolves binding-query recalls (forward AND backward) with the source's USGS
//! "This Dynamic Earth" citation, and abstains on a word that is not one of the
//! four internal layers (the atmosphere) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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

#[test]
fn geology_earth_layers_recall_binds_state_with_citation() {
    let dir = scratch("earthlayers");
    // Copy the shipped geology table beside the entry program and import it.
    let src = facts_stdlib().join("geology/earth-layers.adj");
    std::fs::copy(&src, dir.join("earth-layers.adj")).expect("copy shipped earth-layers.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"earth-layers.adj\"\n\
         ? has_state(outer_core, $State)\n\
         ? has_state(inner_core, $State)\n\
         ? has_state(mantle, $State)\n\
         ? has_state($Layer, liquid)\n\
         ? has_state(atmosphere, $State)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The outer core is liquid, the inner core is solid, the mantle is
    // semi-solid — the recalled states (forward binds).
    assert!(
        out.contains("\"State\":\"liquid\""),
        "outer_core → liquid: {out}"
    );
    assert!(
        out.contains("\"State\":\"solid\""),
        "inner_core → solid: {out}"
    );
    assert!(
        out.contains("\"State\":\"semi_solid\""),
        "mantle → semi_solid: {out}"
    );
    // The relation runs BACKWARD: bind the state `liquid`, recall which layer is
    // in it — the outer core.
    assert!(
        out.contains("\"Layer\":\"outer_core\""),
        "liquid → outer_core (reverse recall): {out}"
    );
    // The answer carries the USGS "This Dynamic Earth" citation as its proof, at
    // the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("pubs.usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The atmosphere is a gaseous envelope, not one of the four internal layers —
    // honest abstention, never a fabricated state.
    assert!(
        out.contains("\"abstained\":true"),
        "atmosphere abstains: {out}"
    );
}
