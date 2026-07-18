//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/reaction-types.adj`) driven through the built
//! CLI: a native `table` of the five basic chemical-reaction types → the
//! defining token each is described by resolves binding-query recalls (forward
//! AND backward) with the source's LibreTexts citation, and abstains on a word
//! that is not one of the five basic reaction types (neutralization) — 0 model
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
fn chemistry_reaction_types_recall_binds_defining_with_citation() {
    let dir = scratch("reactiontypes");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/reaction-types.adj");
    std::fs::copy(&src, dir.join("reaction-types.adj")).expect("copy shipped reaction-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"reaction-types.adj\"\n\
         ? reaction_defining(combination, $Defining)\n\
         ? reaction_defining(combustion, $Defining)\n\
         ? reaction_defining(double_replacement, $Defining)\n\
         ? reaction_defining($Type, breaks_down)\n\
         ? reaction_defining(neutralization, $Defining)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A combination reaction combines two or more substances, combustion burns
    // in oxygen, double-replacement swaps ions — the recalled defining tokens
    // (forward binds).
    assert!(
        out.contains("\"Defining\":\"two_or_more_combine\""),
        "combination → two_or_more_combine: {out}"
    );
    assert!(
        out.contains("\"Defining\":\"reacts_with_oxygen\""),
        "combustion → reacts_with_oxygen: {out}"
    );
    assert!(
        out.contains("\"Defining\":\"ions_exchange_places\""),
        "double_replacement → ions_exchange_places: {out}"
    );
    // The relation runs BACKWARD: bind the defining token `breaks_down`, recall
    // its reaction type.
    assert!(
        out.contains("\"Type\":\"decomposition\""),
        "breaks_down → decomposition (reverse recall): {out}"
    );
    // The answer carries the LibreTexts citation as its proof, at the
    // `consensus` trust tier for an openly-licensed college teaching resource.
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // Neutralization is a reaction but not one of the five basic classification
    // types in this table — honest abstention, never a fabricated definition.
    assert!(
        out.contains("\"abstained\":true"),
        "neutralization abstains: {out}"
    );
}
