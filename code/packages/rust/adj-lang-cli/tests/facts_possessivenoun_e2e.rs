//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/possessive-noun.adj`) driven through the
//! built CLI: a native `table` naming three nouns and which of the three
//! possessive-noun categories each falls into, quoted verbatim from
//! Grammarly's "Possessive Nouns" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_possessivenoun_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/possessive-noun.adj");
    std::fs::copy(&src, dir.join("possessive-noun.adj"))
        .expect("copy shipped possessive-noun.adj");
}

#[test]
fn possessive_noun_recall_binds_the_category_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"possessive-noun.adj\"\n\
         ? possessive_noun(dog, $Category)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Category\":\"singular_possessive\""),
        "dog's possessive is singular_possessive: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn possessive_noun_reverse_binds_the_word_for_that_category() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"possessive-noun.adj\"\n\
         ? possessive_noun($Word, plural_possessive)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Word\":\"bottles\""),
        "the shipped plural_possessive example is bottles: {out}"
    );
}

#[test]
fn possessive_noun_abstains_honestly_on_an_untabled_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"possessive-noun.adj\"\n\
         ? possessive_noun(cat, $Category)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "cat is a real noun but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
