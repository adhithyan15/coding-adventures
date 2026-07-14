//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/animal-classes.adj`) driven through the built CLI:
//! a native `table` of animal → vertebrate class resolves a binding-query recall
//! with the source's citation, and abstains on a non-animal — 0 model calls.

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
fn biology_animal_classes_recall_binds_class_with_citation() {
    let dir = scratch("animalclasses");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/animal-classes.adj");
    std::fs::copy(&src, dir.join("animal-classes.adj")).expect("copy shipped animal-classes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-classes.adj\"\n\
         ? animal_class(cat, $Class)\n\
         ? animal_class(snake, $Class)\n\
         ? animal_class(rock, $Class)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A cat is a mammal; a snake is a reptile — the recalled classes.
    assert!(out.contains("\"Class\":\"mammal\""), "cat → mammal: {out}");
    assert!(out.contains("\"Class\":\"reptile\""), "snake → reptile: {out}");
    // The answer carries the Australian Museum citation (locator + trust) as its proof.
    assert!(
        out.contains("australian.museum") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A rock is not an animal — honest abstention, never a fabricated class.
    assert!(out.contains("\"abstained\":true"), "rock abstains: {out}");
}
