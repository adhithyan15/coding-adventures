//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/synonyms.adj`) driven through the built
//! CLI: a native `table` naming three common words and a synonym of each,
//! per the English Wiktionary entries for those words -- a sibling library
//! to `opposites.adj` (antonyms). 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_synonyms_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/synonyms.adj");
    std::fs::copy(&src, dir.join("synonyms.adj")).expect("copy shipped synonyms.adj");
}

#[test]
fn synonym_recall_binds_the_synonym_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"synonyms.adj\"\n\
         ? synonym(happy, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Word\":\"cheerful\""),
        "a synonym of happy is cheerful: {out}"
    );
    assert!(
        out.contains("en.wiktionary.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wiktionary citation: {out}"
    );
}

#[test]
fn synonym_reverse_binds_the_word_for_that_synonym() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"synonyms.adj\"\n\
         ? synonym($W, fast)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"W\":\"quick\""),
        "quick's shipped synonym is fast: {out}"
    );
}

#[test]
fn synonym_abstains_honestly_on_an_untabled_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"synonyms.adj\"\n\
         ? synonym(purple, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "purple is a real word but has no shipped synonym in this table -- honest abstention, never invented: {out}"
    );
}

#[test]
fn synonym_extension_recalls_every_synonym_per_word() {
    let dir = scratch("ext");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"synonyms.adj\"\n\
         ? synonym(happy, $Word)\n\
         ? synonym(smart, $Word)\n\
         ? synonym(quick, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // happy now recalls NINE synonyms, not just cheerful -- the header's own
    // already-quoted Wiktionary span always named all nine, only the first
    // had ever been turned into a row.
    assert!(
        out.contains("\"Word\":\"jubilant\""),
        "happy → jubilant (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Word\":\"merry\""),
        "happy → merry (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Word\":\"sophisticated\""),
        "smart → sophisticated (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Word\":\"witty\""),
        "smart → witty (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Word\":\"swift\""),
        "quick → swift (added this cycle): {out}"
    );
}
