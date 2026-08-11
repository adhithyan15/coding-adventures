//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/contraction.adj`) driven through the built
//! CLI: a native `table` naming three contractions and the two-word
//! phrase each stands for, per Grammarly's "What Are Contractions in
//! Writing?" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_contraction_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/contraction.adj");
    std::fs::copy(&src, dir.join("contraction.adj")).expect("copy shipped contraction.adj");
}

#[test]
fn contraction_recall_binds_the_expansion_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"contraction.adj\"\n\
         ? contraction(dont, $Expansion)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Expansion\":\"do_not\""),
        "dont expands to do_not: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn contraction_reverse_binds_the_word_for_that_expansion() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"contraction.adj\"\n\
         ? contraction($Word, will_not)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Word\":\"wont\""),
        "the shipped will_not contraction is wont: {out}"
    );
}

#[test]
fn contraction_abstains_honestly_on_an_untabled_contraction() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"contraction.adj\"\n\
         ? contraction(shouldnt, $Expansion)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "shouldnt is a real contraction (should not) but has no shipped expansion in this table -- honest abstention, never invented: {out}"
    );
}
