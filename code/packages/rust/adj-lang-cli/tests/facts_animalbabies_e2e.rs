//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/animal-babies.adj`) driven through the built CLI:
//! a native `table` of animal → the name of its baby resolves a binding-query
//! recall with the source's citation, and abstains on a non-animal — 0 model
//! calls.

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
fn biology_animal_babies_recall_binds_baby_name_with_citation() {
    let dir = scratch("animalbabies");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/animal-babies.adj");
    std::fs::copy(&src, dir.join("animal-babies.adj")).expect("copy shipped animal-babies.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-babies.adj\"\n\
         ? animal_baby(dog, $Baby)\n\
         ? animal_baby(kangaroo, $Baby)\n\
         ? animal_baby(rock, $Baby)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A baby dog is a puppy; a baby kangaroo is a joey — the recalled names.
    assert!(out.contains("\"Baby\":\"puppy\""), "dog → puppy: {out}");
    assert!(out.contains("\"Baby\":\"joey\""), "kangaroo → joey: {out}");
    // The answer carries the Wikipedia citation (locator + trust) as its proof.
    // Wikipedia is an encyclopedia — a secondary reference — so `consensus`.
    assert!(
        out.contains("List_of_animal_names") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // A rock is not an animal — honest abstention, never a fabricated name.
    assert!(out.contains("\"abstained\":true"), "rock abstains: {out}");
}
