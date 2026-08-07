//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/macronutrients.adj`) driven through the built CLI:
//! a native `table` of macronutrient → kilocalories-per-gram resolves binding-
//! query recalls (forward AND backward) with the source's NIH / MedlinePlus
//! citation, and abstains on a word that is not one of these macronutrients
//! (water, which supplies no Calories) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsmn_{tag}_{}", std::process::id()));
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
fn biology_macronutrients_recall_binds_energy_with_citation() {
    let dir = scratch("macronutrients");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/macronutrients.adj");
    std::fs::copy(&src, dir.join("macronutrients.adj")).expect("copy shipped macronutrients.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"macronutrients.adj\"\n\
         ? kcal_per_gram(fat, $Kcal)\n\
         ? kcal_per_gram(carbohydrate, $Kcal)\n\
         ? kcal_per_gram(protein, $Kcal)\n\
         ? kcal_per_gram(alcohol, $Kcal)\n\
         ? kcal_per_gram($Nutrient, 4)\n\
         ? kcal_per_gram(water, $Kcal)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A gram of fat gives 9 Calories; a gram of carbohydrate or protein gives 4;
    // a gram of alcohol gives 7 — the recalled numbers, each a plain integer.
    assert!(out.contains("\"Kcal\":\"9\""), "fat → 9: {out}");
    assert!(out.contains("\"Kcal\":\"4\""), "carbohydrate/protein → 4: {out}");
    assert!(out.contains("\"Kcal\":\"7\""), "alcohol → 7: {out}");
    // The relation runs BACKWARD: bind the value 4, recall the macronutrients
    // that supply it — carbohydrate and protein.
    assert!(
        out.contains("\"Nutrient\":\"carbohydrate\"") && out.contains("\"Nutrient\":\"protein\""),
        "4 → carbohydrate ; protein (reverse recall): {out}"
    );
    // The answer carries the NIH / NLM MedlinePlus citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("medlineplus.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Water supplies no Calories — it is not one of the energy-yielding
    // macronutrients, so an energy recall abstains honestly rather than
    // fabricating a number.
    assert!(out.contains("\"abstained\":true"), "water abstains: {out}");
}
