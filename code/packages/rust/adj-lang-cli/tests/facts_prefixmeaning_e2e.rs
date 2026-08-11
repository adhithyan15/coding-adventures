//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/prefix-meaning.adj`) driven through the
//! built CLI: a native `table` naming three common prefixes and what
//! each actually means, quoted verbatim from Grammarly's "Prefixes:
//! Definition and Examples" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_prefix_meaning_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/prefix-meaning.adj");
    std::fs::copy(&src, dir.join("prefix-meaning.adj"))
        .expect("copy shipped prefix-meaning.adj");
}

#[test]
fn prefix_meaning_recall_binds_the_meaning_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"prefix-meaning.adj\"\n\
         ? prefix_meaning(un_, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"M\":\"negation_or_absence\""),
        "un- means negation_or_absence: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn prefix_meaning_reverse_binds_the_prefix_for_that_meaning() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"prefix-meaning.adj\"\n\
         ? prefix_meaning($P, doing_again)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"P\":\"re_\""),
        "the shipped doing_again example is re_: {out}"
    );
}

#[test]
fn prefix_meaning_abstains_honestly_on_an_untabled_prefix() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"prefix-meaning.adj\"\n\
         ? prefix_meaning(over_, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "over_ is a real prefix but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
