//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/plant-tropisms.adj`) driven through the built CLI:
//! a native `table` of the classic plant tropisms → the environmental stimulus
//! each responds to resolves binding-query recalls (forward AND backward) with
//! the source's Wikipedia "Tropism" citation, and abstains on a word that is not
//! one of these tropisms (`sound`) — 0 model calls.

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
fn biology_plant_tropisms_recall_binds_stimulus_with_citation() {
    let dir = scratch("planttropisms");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/plant-tropisms.adj");
    std::fs::copy(&src, dir.join("plant-tropisms.adj")).expect("copy shipped plant-tropisms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-tropisms.adj\"\n\
         ? tropism_stimulus(phototropism, $Stimulus)\n\
         ? tropism_stimulus(thigmotropism, $Stimulus)\n\
         ? tropism_stimulus(hydrotropism, $Stimulus)\n\
         ? tropism_stimulus($Tropism, gravity)\n\
         ? tropism_stimulus(sound, $Stimulus)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A phototropism tracks light, a thigmotropism tracks touch, a hydrotropism
    // tracks water — the recalled stimuli (forward binds).
    assert!(
        out.contains("\"Stimulus\":\"light\""),
        "phototropism → light: {out}"
    );
    assert!(
        out.contains("\"Stimulus\":\"touch\""),
        "thigmotropism → touch: {out}"
    );
    assert!(
        out.contains("\"Stimulus\":\"water\""),
        "hydrotropism → water: {out}"
    );
    // The relation runs BACKWARD: bind the stimulus `gravity`, recall its tropism.
    assert!(
        out.contains("\"Tropism\":\"gravitropism\""),
        "gravity → gravitropism (reverse recall): {out}"
    );
    // The answer carries the Wikipedia "Tropism" citation as its proof, at the
    // `consensus` trust tier for a teaching-quality encyclopedia summary.
    assert!(
        out.contains("en.wikipedia.org/wiki/Tropism") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // `sound` is not one of these tropisms — honest abstention, never a
    // fabricated stimulus.
    assert!(out.contains("\"abstained\":true"), "sound abstains: {out}");
}
