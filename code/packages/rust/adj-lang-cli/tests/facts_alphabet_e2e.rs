//! End-to-end test for the LANGUAGE FACTS library
//! (`adj-facts-stdlib/language/alphabet.adj`) driven through the built CLI:
//! a native `table` of the 26 English letters → their 1-based position in the
//! alphabet resolves forward AND reverse binding-query recalls with the
//! encyclopedia's citation, and abstains on a digit that has no shipped row —
//! 0 answer-time model calls.

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
fn language_alphabet_recall_binds_position_forward_and_reverse() {
    let dir = scratch("alphabet");
    // Copy the shipped alphabet table beside the entry program and import it.
    let src = facts_stdlib().join("language/alphabet.adj");
    std::fs::copy(&src, dir.join("alphabet.adj")).expect("copy shipped alphabet.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"alphabet.adj\"\n\
         ? alphabet_position(a, $N)\n\
         ? alphabet_position(z, $N)\n\
         ? alphabet_position($L, 3)\n\
         ? alphabet_position(3, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward: a is the first letter, z is the twenty-sixth (and last).
    assert!(out.contains("\"N\":\"1\""), "a → 1: {out}");
    assert!(out.contains("\"N\":\"26\""), "z → 26: {out}");
    // Reverse: the third letter of the alphabet is c (binds the other column).
    assert!(out.contains("\"L\":\"c\""), "position 3 → c: {out}");
    // The answer carries the Simple-Wikipedia citation as its proof, at consensus trust.
    assert!(
        out.contains("simple.wikipedia.org/wiki/English_alphabet")
            && out.contains("\"trust\":\"consensus\""),
        "carries the encyclopedia citation: {out}"
    );
    // `3` is a digit, not one of the 26 letter rows — honest abstention, never invented.
    assert!(out.contains("\"abstained\":true"), "digit 3 abstains: {out}");
}
