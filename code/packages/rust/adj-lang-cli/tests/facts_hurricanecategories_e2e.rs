//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/hurricane-categories.adj`) driven through the
//! built CLI: a native `table` of the five Saffir-Simpson Hurricane Wind Scale
//! categories → their NHC damage descriptor resolves binding-query recalls
//! (forward AND backward) with the NOAA / National Hurricane Center citation,
//! and abstains on a key that is not one of the five categories (category_6) —
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
fn meteorology_hurricane_categories_recall_binds_descriptor_with_citation() {
    let dir = scratch("hurricanecategories");
    // Copy the shipped meteorology table beside the entry program and import it.
    let src = facts_stdlib().join("meteorology/hurricane-categories.adj");
    std::fs::copy(&src, dir.join("hurricane-categories.adj"))
        .expect("copy shipped hurricane-categories.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"hurricane-categories.adj\"\n\
         ? damage_level(category_1, $Descriptor)\n\
         ? damage_level(category_3, $Descriptor)\n\
         ? damage_level(category_4, $Descriptor)\n\
         ? damage_level(category_5, $Descriptor)\n\
         ? damage_level($Category, catastrophic_damage)\n\
         ? damage_level(category_6, $Descriptor)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");

    // (a) A category-1 hurricane produces "some damage" — the recalled
    // descriptor (forward bind) — and the answer carries the NHC citation at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("\"Descriptor\":\"some_damage\""),
        "category_1 → some_damage: {out}"
    );
    assert!(
        out.contains("nhc.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "category_1 answer carries the NHC source citation at authoritative trust: {out}"
    );

    // A category 3 brings "devastating damage"; a category 4 and a category 5
    // both bring "catastrophic damage" (the honest duplicate — the NHC source
    // opens both paragraphs with the same "Catastrophic damage will occur").
    assert!(
        out.contains("\"Descriptor\":\"devastating_damage\""),
        "category_3 → devastating_damage: {out}"
    );
    assert!(
        out.contains("\"Descriptor\":\"catastrophic_damage\""),
        "category_4 / category_5 → catastrophic_damage: {out}"
    );

    // The relation runs BACKWARD: bind the descriptor `catastrophic_damage`, and
    // recall the categories that carry it — HONESTLY both category_4 AND
    // category_5, because the source states the same descriptor for each.
    assert!(
        out.contains("\"Category\":\"category_4\""),
        "catastrophic_damage → category_4 (reverse recall): {out}"
    );
    assert!(
        out.contains("\"Category\":\"category_5\""),
        "catastrophic_damage → category_5 (reverse recall): {out}"
    );

    // (b) There is no category 6 on the Saffir-Simpson scale — honest
    // abstention, never a fabricated damage word.
    assert!(
        out.contains("\"abstained\":true"),
        "category_6 abstains: {out}"
    );
}
