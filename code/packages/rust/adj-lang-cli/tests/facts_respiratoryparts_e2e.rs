//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/respiratory-parts.adj`) driven through the built
//! CLI: a native `table` of each respiratory-system part → the function / role
//! its source states resolves binding-query recalls (forward AND backward) with
//! the source's NCI SEER Training Modules citation, and abstains on a word that
//! is not one of these respiratory parts (the stomach) — 0 model calls.

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
fn anatomy_respiratory_parts_recall_binds_function_with_citation() {
    let dir = scratch("respiratoryparts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/respiratory-parts.adj");
    std::fs::copy(&src, dir.join("respiratory-parts.adj"))
        .expect("copy shipped respiratory-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"respiratory-parts.adj\"\n\
         ? part_function(trachea, $Function)\n\
         ? part_function(larynx, $Function)\n\
         ? part_function(diaphragm, $Function)\n\
         ? part_function($Part, gas_exchange)\n\
         ? part_function(stomach, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The trachea is the main airway, the larynx does speech, the diaphragm
    // contracts on inspiration — the recalled functions (forward binds).
    assert!(
        out.contains("\"Function\":\"main_airway\""),
        "trachea → main_airway: {out}"
    );
    assert!(
        out.contains("\"Function\":\"human_speech\""),
        "larynx → human_speech: {out}"
    );
    assert!(
        out.contains("\"Function\":\"contracts_inspiration\""),
        "diaphragm → contracts_inspiration: {out}"
    );
    // The relation runs BACKWARD: bind the function `gas_exchange`, recall the
    // part that performs it.
    assert!(
        out.contains("\"Part\":\"alveoli\""),
        "gas_exchange → alveoli (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training Modules citation as its proof, at
    // the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The stomach is a digestive organ, not a respiratory part — honest
    // abstention, never a fabricated function.
    assert!(out.contains("\"abstained\":true"), "stomach abstains: {out}");
}
