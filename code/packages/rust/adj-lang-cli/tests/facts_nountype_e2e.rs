//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/noun-type.adj`) driven through the built
//! CLI: a native `table` naming six noun types and what each actually
//! is, quoted verbatim from Grammarly's "Nouns: Definition and Examples"
//! article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_noun_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/noun-type.adj");
    std::fs::copy(&src, dir.join("noun-type.adj")).expect("copy shipped noun-type.adj");
}

#[test]
fn noun_type_recall_binds_the_definition_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type(common_noun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"generic_name_of_an_item_in_a_class_or_group\""),
        "common_noun means generic_name_of_an_item_in_a_class_or_group: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn noun_type_reverse_binds_the_type_for_that_definition() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type($T, cannot_be_perceived_by_the_senses)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"abstract_noun\""),
        "the shipped cannot_be_perceived_by_the_senses example is abstract_noun: {out}"
    );
}

#[test]
fn noun_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type(possessive_noun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "possessive_noun is a real noun type the source mentions but not one of the six tabled here -- honest abstention, never invented: {out}"
    );
}

#[test]
fn noun_type_recall_binds_a_newly_added_row_directly() {
    let dir = scratch("direct_new");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type(concrete_noun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"D\":\"perceived_by_the_senses_physical_or_tangible\""),
        "concrete_noun means perceived_by_the_senses_physical_or_tangible: {out}"
    );
}

#[test]
fn noun_type_reverse_binds_a_newly_added_row() {
    let dir = scratch("reverse_new");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type($T, impossible_to_count)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"uncountable_noun\""),
        "the shipped impossible_to_count example is uncountable_noun: {out}"
    );
}

#[test]
fn noun_type_abstains_honestly_on_a_bundled_fact_candidate() {
    let dir = scratch("abstain_bundled");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type(proper_noun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "proper_noun is a real noun type the same source page defines, but its sentence bundles the naming function with a separate capitalization rule -- honest abstention, never invented: {out}"
    );
}
