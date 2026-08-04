//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/light-behaviors.adj`) driven through the built
//! CLI: a native `table` of the ways a light wave interacts with matter → the
//! effect the source states resolves binding-query recalls (forward AND
//! backward) with the source's NASA Science citation, and abstains on a word
//! that is not one of these light behaviors (gravity) — 0 model calls.

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
fn physics_light_behaviors_recall_binds_effect_with_citation() {
    let dir = scratch("lightbehaviors");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/light-behaviors.adj");
    std::fs::copy(&src, dir.join("light-behaviors.adj")).expect("copy shipped light-behaviors.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"light-behaviors.adj\"\n\
         ? light_behavior(reflection, $Effect)\n\
         ? light_behavior(refraction, $Effect)\n\
         ? light_behavior(diffraction, $Effect)\n\
         ? light_behavior($Behavior, bounces_off)\n\
         ? light_behavior(gravity, $Effect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Reflection bounces off, refraction changes direction, diffraction bends
    // and spreads — the recalled effects (forward binds).
    assert!(
        out.contains("\"Effect\":\"bounces_off\""),
        "reflection → bounces_off: {out}"
    );
    assert!(
        out.contains("\"Effect\":\"changes_direction\""),
        "refraction → changes_direction: {out}"
    );
    assert!(
        out.contains("\"Effect\":\"bends_and_spreads\""),
        "diffraction → bends_and_spreads: {out}"
    );
    // The relation runs BACKWARD: bind the effect `bounces_off`, recall its
    // behavior.
    assert!(
        out.contains("\"Behavior\":\"reflection\""),
        "bounces_off → reflection (reverse recall): {out}"
    );
    // The answer carries the NASA Science citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Gravity is a force, not a way light interacts with matter — honest
    // abstention, never a fabricated effect.
    assert!(out.contains("\"abstained\":true"), "gravity abstains: {out}");
}
