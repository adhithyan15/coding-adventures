//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/element-categories.adj`) driven through the
//! built CLI: a native `table` of the three broad categories of chemical
//! element → the defining electrical property resolves binding-query recalls
//! (forward AND backward) with the source's Chemistry LibreTexts citation, and
//! abstains on a word that is not one of the three broad categories (a
//! compound) — 0 model calls.

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
fn chemistry_element_categories_recall_binds_property_with_citation() {
    let dir = scratch("elementcategories");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/element-categories.adj");
    std::fs::copy(&src, dir.join("element-categories.adj"))
        .expect("copy shipped element-categories.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"element-categories.adj\"\n\
         ? element_category_property(metal, $Property)\n\
         ? element_category_property(nonmetal, $Property)\n\
         ? element_category_property(metalloid, $Property)\n\
         ? element_category_property($Category, poor_conductor)\n\
         ? element_category_property(compound, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Metals are good conductors, nonmetals are poor conductors, metalloids are
    // semiconductors — the recalled properties (forward binds).
    assert!(
        out.contains("\"Property\":\"good_conductor\""),
        "metal → good_conductor: {out}"
    );
    assert!(
        out.contains("\"Property\":\"poor_conductor\""),
        "nonmetal → poor_conductor: {out}"
    );
    assert!(
        out.contains("\"Property\":\"semiconductor\""),
        "metalloid → semiconductor: {out}"
    );
    // The relation runs BACKWARD: bind the property `poor_conductor`, recall its
    // category.
    assert!(
        out.contains("\"Category\":\"nonmetal\""),
        "poor_conductor → nonmetal (reverse recall): {out}"
    );
    // The answer carries the Chemistry LibreTexts citation as its proof, at the
    // `consensus` trust tier for an open teaching resource.
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // A compound is not one of the three broad element categories — honest
    // abstention, never a fabricated property.
    assert!(out.contains("\"abstained\":true"), "compound abstains: {out}");
}
