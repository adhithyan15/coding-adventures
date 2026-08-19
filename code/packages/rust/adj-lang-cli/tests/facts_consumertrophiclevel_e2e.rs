//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/consumer-trophic-level.adj`) driven through the
//! built CLI: a native `table` naming the three consumer trophic levels an
//! ecosystem's food chain runs on and what each one eats, grounding National
//! Geographic Education's "Consumers" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_consumertrophiclevel_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/consumer-trophic-level.adj");
    std::fs::copy(&src, dir.join("consumer-trophic-level.adj"))
        .expect("copy shipped consumer-trophic-level.adj");
}

#[test]
fn consumer_trophic_level_recall_binds_what_each_level_eats() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"consumer-trophic-level.adj\"\n\
         ? consumer_trophic_level(primary_consumer, $Eats)\n\
         ? consumer_trophic_level(secondary_consumer, $Eats)\n\
         ? consumer_trophic_level(tertiary_consumer, $Eats)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Eats\":\"primary_producers\""),
        "a primary consumer eats primary producers: {out}"
    );
    assert!(
        out.contains("\"Eats\":\"primary_consumers\""),
        "a secondary consumer eats primary consumers: {out}"
    );
    assert!(
        out.contains("\"Eats\":\"other_carnivores\""),
        "a tertiary consumer eats other carnivores: {out}"
    );
    assert!(
        out.contains("nationalgeographic.org") && out.contains("\"trust\":\"consensus\""),
        "carries the National Geographic Education citation at consensus trust: {out}"
    );
}

#[test]
fn consumer_trophic_level_reverse_binds_the_level_from_what_it_eats() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"consumer-trophic-level.adj\"\n\
         ? consumer_trophic_level($L, primary_consumers)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"L\":\"secondary_consumer\""),
        "the level that eats primary consumers is the secondary consumer: {out}"
    );
}

#[test]
fn consumer_trophic_level_abstains_honestly_on_decomposer() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"consumer-trophic-level.adj\"\n\
         ? consumer_trophic_level(decomposer, $Eats)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a decomposer is a real ecosystem role the cited article also discusses, but its own \
         structure keeps decomposers outside the three consumer trophic levels this table \
         grounds -- honest abstention, never invented: {out}"
    );
}
