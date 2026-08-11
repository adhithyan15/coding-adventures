//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/idiom-meaning.adj`) driven through the built
//! CLI: a native `table` naming three common English idioms and what each
//! one actually means, per Oxford International English's "30 Useful
//! English Idiomatic Expressions & Their Meanings" article. The
//! THIRTEENTH literacy sub-skill library in this loop's sweep. 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_idiommeaning_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/idiom-meaning.adj");
    std::fs::copy(&src, dir.join("idiom-meaning.adj")).expect("copy shipped idiom-meaning.adj");
}

#[test]
fn idiom_meaning_recall_binds_the_meaning_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"idiom-meaning.adj\"\n\
         ? idiom_meaning(piece_of_cake, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"M\":\"very_easy_to_do\""),
        "piece of cake means very easy to do: {out}"
    );
    assert!(
        out.contains("oxfordinternationalenglish.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Oxford International English citation: {out}"
    );
}

#[test]
fn idiom_meaning_reverse_binds_the_idiom_for_that_meaning() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"idiom-meaning.adj\"\n\
         ? idiom_meaning($I, feeling_slightly_ill)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"I\":\"under_the_weather\""),
        "under the weather means feeling slightly ill: {out}"
    );
}

#[test]
fn idiom_meaning_abstains_honestly_on_an_untabled_idiom() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"idiom-meaning.adj\"\n\
         ? idiom_meaning(raining_cats_and_dogs, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "raining cats and dogs is a real idiom but not one of the three tabled examples -- honest abstention, never invented: {out}"
    );
}
