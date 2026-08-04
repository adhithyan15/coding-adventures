//! End-to-end test for the LANGUAGE FACTS library
//! (`adj-facts-stdlib/language/vowels.adj`) driven through the built CLI:
//! a native `table` of the five English vowel letters resolves a binding-query
//! recall with the encyclopedia's citation, and abstains on a consonant letter
//! that has no shipped row — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn language_vowels_recall_binds_membership_with_citation() {
    let dir = scratch("vowels");
    // Copy the shipped vowels table beside the entry program and import it.
    let src = facts_stdlib().join("language/vowels.adj");
    std::fs::copy(&src, dir.join("vowels.adj")).expect("copy shipped vowels.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"vowels.adj\"\n\
         ? vowel_letter(a, $V)\n\
         ? vowel_letter(u, $V)\n\
         ? vowel_letter(b, $V)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // a and u are both vowel letters — each binds `yes` from the table.
    assert!(out.contains("\"V\":\"yes\""), "vowel letters bind yes: {out}");
    // The answer carries the Simple-Wikipedia citation as its proof, at consensus trust.
    assert!(
        out.contains("simple.wikipedia.org/wiki/Vowel") && out.contains("\"trust\":\"consensus\""),
        "carries the encyclopedia citation: {out}"
    );
    // `b` is a consonant with no shipped row — honest abstention, never invented.
    assert!(out.contains("\"abstained\":true"), "consonant b abstains: {out}");
}
