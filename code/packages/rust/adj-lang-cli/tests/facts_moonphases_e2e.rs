//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/moon-phases.adj`) driven through the built CLI:
//! a native `table` of Moon-phase → cycle-position resolves a binding-query
//! recall with NASA's citation, and abstains on a non-phase — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn astronomy_moon_phases_recall_binds_cycle_position_with_citation() {
    let dir = scratch("moonphases");
    // Copy the shipped astronomy table beside the entry program and import it.
    let src = facts_stdlib().join("astronomy/moon-phases.adj");
    std::fs::copy(&src, dir.join("moon-phases.adj")).expect("copy shipped moon-phases.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"moon-phases.adj\"\n\
         ? moon_phase_order(new_moon, $Pos)\n\
         ? moon_phase_order(full_moon, $Pos)\n\
         ? moon_phase_order(eclipse, $Pos)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // New Moon opens the cycle (1); full Moon is the fifth phase (5).
    assert!(out.contains("\"Pos\":\"1\""), "new_moon → 1: {out}");
    assert!(out.contains("\"Pos\":\"5\""), "full_moon → 5: {out}");
    // The answer carries NASA's citation as its proof, at authoritative trust.
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA source citation: {out}"
    );
    // An eclipse is not a lunar phase — honest abstention, never a fabricated order.
    assert!(out.contains("\"abstained\":true"), "eclipse abstains: {out}");
}
