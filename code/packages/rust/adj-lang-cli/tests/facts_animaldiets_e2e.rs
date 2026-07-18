//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/animal-diets.adj`) driven through the built CLI:
//! a native `table` of diet category → food resolves a binding-query recall with
//! the source's citation, runs backward, and abstains on a non-category — with
//! 0 model calls.

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
fn biology_animal_diets_recall_binds_food_with_citation() {
    let dir = scratch("animaldiets");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/animal-diets.adj");
    std::fs::copy(&src, dir.join("animal-diets.adj")).expect("copy shipped animal-diets.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-diets.adj\"\n\
         ? diet_food(herbivore, $Food)\n\
         ? diet_food(carnivore, $Food)\n\
         ? diet_food(omnivore, $Food)\n\
         ? diet_food($Category, plants)\n\
         ? diet_food(detritivore, $Food)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A herbivore eats plants; a carnivore eats animals; an omnivore eats anything.
    assert!(out.contains("\"Food\":\"plants\""), "herbivore → plants: {out}");
    assert!(out.contains("\"Food\":\"animals\""), "carnivore → animals: {out}");
    assert!(out.contains("\"Food\":\"anything\""), "omnivore → anything: {out}");
    // Reverse bind: the food `plants` recalls its diet category, herbivore.
    assert!(
        out.contains("\"Category\":\"herbivore\""),
        "plants → herbivore (reverse): {out}"
    );
    // The answer carries the U.S. National Park Service citation (locator + trust).
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A detritivore is not one of the three diet categories the source defines —
    // honest abstention, never a fabricated food.
    assert!(out.contains("\"abstained\":true"), "detritivore abstains: {out}");
}
