//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/element-groups.adj`) driven through the built
//! CLI: a native `table` of common element → periodic-table group family
//! resolves binding-query recalls (forward and backward) with the source's
//! Wikipedia citation, and abstains on an element not in the table (gold) —
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
    let dir = std::env::temp_dir().join(format!("adjcli_factseg_{tag}_{}", std::process::id()));
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
fn chemistry_element_group_family_recall_binds_family_with_citation() {
    let dir = scratch("elementgroups");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/element-groups.adj");
    std::fs::copy(&src, dir.join("element-groups.adj")).expect("copy shipped element-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"element-groups.adj\"\n\
         ? element_group_family(sodium, $Family)\n\
         ? element_group_family(chlorine, $Family)\n\
         ? element_group_family(iron, $Family)\n\
         ? element_group_family($E, noble_gas)\n\
         ? element_group_family(gold, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Sodium is an alkali metal, chlorine a halogen, iron a transition metal —
    // the recalled families (forward binds).
    assert!(
        out.contains("\"Family\":\"alkali_metal\""),
        "sodium → alkali_metal: {out}"
    );
    assert!(
        out.contains("\"Family\":\"halogen\""),
        "chlorine → halogen: {out}"
    );
    assert!(
        out.contains("\"Family\":\"transition_metal\""),
        "iron → transition_metal: {out}"
    );
    // The relation runs BACKWARD: bind the family noble_gas, recall an element in
    // it — helium, the first noble gas in the table.
    assert!(
        out.contains("\"E\":\"helium\""),
        "noble_gas → helium (reverse recall into the noble gases): {out}"
    );
    // The answer carries the Wikipedia citation as its proof, at consensus trust
    // (a secondary encyclopedia reference, honestly tiered).
    assert!(
        out.contains("en.wikipedia.org/wiki/Alkali_metal")
            && out.contains("\"trust\":\"consensus\""),
        "carries the source citation at consensus trust: {out}"
    );
    // "gold" is not in the table — honest abstention, never a fabricated family.
    assert!(out.contains("\"abstained\":true"), "gold abstains: {out}");
}

#[test]
fn chemistry_element_group_family_extension_recalls_newly_added_elements() {
    let dir = scratch("elementgroups_ext");
    let src = facts_stdlib().join("chemistry/element-groups.adj");
    std::fs::copy(&src, dir.join("element-groups.adj")).expect("copy shipped element-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"element-groups.adj\"\n\
         ? element_group_family($E, noble_gas)\n\
         ? element_group_family(caesium, $Family)\n\
         ? element_group_family(cobalt, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Each family's own cited Wikipedia sentence always named more members than
    // had ever been turned into rows — this cycle added the rest as pure
    // additions sharing the existing family key. The noble_gas reverse recall
    // now returns all six shipped noble gases (oganesson deliberately excluded
    // as a considered exclusion — its source sentence hedges it "in some
    // cases").
    for gas in ["helium", "neon", "argon", "krypton", "xenon", "radon"] {
        assert!(
            out.contains(&format!("element_group_family({gas}, noble_gas)")),
            "noble_gas recalls {gas} (krypton/xenon/radon added this cycle): {out}"
        );
    }
    // oganesson is deliberately NOT a row. Assert that STRUCTURALLY -- no
    // binding and no governing term -- rather than as a bare substring
    // absence over the whole blob.
    //
    // The previous form was `!out.contains("oganesson")`, which also forbade
    // the word appearing inside quoted EVIDENCE. Encoding the Wikipedia
    // noble-gas sentence puts "oganesson (Og)" into the citation text, so
    // that assertion failed while the property it cares about still held.
    // Note the sentence it tripped on is the same one this test's comment
    // cites as its justification ("hedges it in some cases") -- the check
    // forbade the output carrying the evidence for its own reasoning.
    assert!(
        !out.contains("element_group_family(oganesson, noble_gas)"),
        "oganesson is never a governing term: {out}"
    );
    assert!(
        !out.contains("\"E\":\"oganesson\""),
        "oganesson is never bound as an answer: {out}"
    );
    assert!(
        out.contains("\"Family\":\"alkali_metal\""),
        "caesium → alkali_metal (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Family\":\"transition_metal\""),
        "cobalt → transition_metal (added this cycle): {out}"
    );
}

const EG_PREFIX_PIN: &str = r#""bindings":{"Family":"halogen"},"citations":[{"source":"The alkali metals consist of the chemical elements lithium (Li), sodium (Na), potassium (K), rubidium (Rb), caesium (Cs), and francium (Fr).","locator":"https://en.wikipedia.org/wiki/Alkali_metal","trust":"consensus","corroborations":[{"source":"They are beryllium (Be), magnesium (Mg), calcium (Ca), strontium (Sr), barium (Ba), and radium (Ra).","locator":"https://en.wikipedia.org/wiki/Alkaline_earth_metal""#;

const EG_ALL_PIN: &str = r#""bindings":{"E":"helium"},"citations":[{"source":"The alkali metals consist of the chemical elements lithium (Li), sodium (Na), potassium (K), rubidium (Rb), caesium (Cs), and francium (Fr).","locator":"https://en.wikipedia.org/wiki/Alkali_metal","trust":"consensus","corroborations":[{"source":"They are beryllium (Be), magnesium (Mg), calcium (Ca), strontium (Sr), barium (Ba), and radium (Ra).","locator":"https://en.wikipedia.org/wiki/Alkaline_earth_metal"},{"source":"The halogens are a group in the periodic table consisting of six chemically related elements, fluorine (F), chlorine (Cl), bromine (Br), iodine (I), and the radioactive elements astatine (At) and tennessine (Ts), though some authors[1] would exclude tennessine as its chemistry is unknown and is theoretically expected to be more like that of gallium.","locator":"https://en.wikipedia.org/wiki/Halogen"},{"source":"The noble gases (historically the inert gases, sometimes referred to as aerogens[1]) are the members of group 18 of the periodic table: helium (He), neon (Ne), argon (Ar), krypton (Kr), xenon (Xe), radon (Rn) and, in some cases, oganesson (Og).","locator":"https://en.wikipedia.org/wiki/Noble_gas""#;

#[test]
fn element_group_halogen_answer_carries_its_wikipedia_corroboration_intact() {
    let dir = scratch("cite_halogen");
    std::fs::copy(
        facts_stdlib().join("chemistry/element-groups.adj"),
        dir.join("element-groups.adj"),
    )
    .expect("copy shipped element-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"element-groups.adj\"\n? element_group_family(chlorine, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains(EG_PREFIX_PIN),
        "halogen's answer carries the alkaline-earth corroboration intact: {out}"
    );
}

#[test]
fn element_group_noble_gas_answer_keeps_the_halogen_tennessine_caveat() {
    let dir = scratch("cite_noble");
    std::fs::copy(
        facts_stdlib().join("chemistry/element-groups.adj"),
        dir.join("element-groups.adj"),
    )
    .expect("copy shipped element-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"element-groups.adj\"\n? element_group_family($E, noble_gas)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // This whole-list pin exists mainly to hold ONE clause in place.
    //
    // The halogen sentence ends "...and tennessine (Ts), though some authors[1]
    // would exclude tennessine as its chemistry is unknown...". The library's
    // header previously truncated immediately before that clause -- while the
    // table ships `row (tennessine, halogen)`. Truncating right before the
    // qualification that bears on your own row is the same defect found in
    // brain-parts' hippocampus quote. Pinning the full sentence is what stops
    // the caveat being trimmed back off.
    //
    // `transition_metal` deliberately has NO corroboration: its header
    // sentence is not on the live page under any extractor fix, and the
    // nearest candidate names only iron, indirectly, for a row set of
    // iron/cobalt/nickel. Category, not value.
    assert!(
        out.contains(EG_ALL_PIN),
        "noble_gas's answer carries all three corroborations, caveat included: {out}"
    );
}
