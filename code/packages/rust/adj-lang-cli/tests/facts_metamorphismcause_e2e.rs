//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/metamorphism-cause.adj`) driven through
//! the built CLI: a native `table` naming three causes of rock metamorphism
//! and their shared effect, per USGS's "What are metamorphic rocks?" FAQ --
//! a sibling library to `rock-types.adj` but a genuinely different,
//! finer-grained causal axis. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_metamorphismcause_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/metamorphism-cause.adj");
    std::fs::copy(&src, dir.join("metamorphism-cause.adj")).expect("copy shipped metamorphism-cause.adj");
}

#[test]
fn metamorphism_cause_recall_binds_the_effect_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"metamorphism-cause.adj\"\n\
         ? metamorphism_cause(heat, $Effect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Effect\":\"denser_more_compact_rock\""),
        "heat's effect is a denser, more compact rock: {out}"
    );
    assert!(
        out.contains("usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}

#[test]
fn metamorphism_cause_reverse_binds_every_cause_for_that_effect() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"metamorphism-cause.adj\"\n\
         ? metamorphism_cause($Cause, denser_more_compact_rock)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Cause\":\"heat\"")
            && out.contains("\"Cause\":\"pressure\"")
            && out.contains("\"Cause\":\"hot_mineral_rich_fluids\""),
        "all three shipped causes should be enumerated: {out}"
    );
}

#[test]
fn metamorphism_cause_abstains_honestly_on_an_untabled_cause() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"metamorphism-cause.adj\"\n\
         ? metamorphism_cause(sunlight, $Effect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "sunlight is not a shipped cause of metamorphism -- honest abstention, never invented: {out}"
    );
}
