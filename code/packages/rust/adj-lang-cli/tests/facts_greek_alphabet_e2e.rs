//! End-to-end test for the LANGUAGE FACTS library
//! (`adj-facts-stdlib/language/greek-alphabet.adj`) driven through the built CLI:
//! a native `table` of the 24 Greek letters → their 1-based position in the
//! alphabet resolves forward AND reverse binding-query recalls with the
//! encyclopedia's citation, and abstains on a Latin letter that has no shipped
//! row — 0 answer-time model calls.

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
fn language_greek_alphabet_recall_binds_position_forward_and_reverse() {
    let dir = scratch("greek_alphabet");
    // Copy the shipped Greek-alphabet table beside the entry program and import it.
    let src = facts_stdlib().join("language/greek-alphabet.adj");
    std::fs::copy(&src, dir.join("greek-alphabet.adj")).expect("copy shipped greek-alphabet.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"greek-alphabet.adj\"\n\
         ? greek_letter_position(alpha, $N)\n\
         ? greek_letter_position(omega, $N)\n\
         ? greek_letter_position($L, 3)\n\
         ? greek_letter_position(b, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward: alpha is the first letter, omega is the twenty-fourth (and last).
    assert!(out.contains("\"N\":\"1\""), "alpha → 1: {out}");
    assert!(out.contains("\"N\":\"24\""), "omega → 24: {out}");
    // Reverse: the third letter of the Greek alphabet is gamma (binds the other column).
    assert!(out.contains("\"L\":\"gamma\""), "position 3 → gamma: {out}");
    // The answer carries the Wikipedia citation as its proof, at consensus trust.
    assert!(
        out.contains("en.wikipedia.org/wiki/Greek_alphabet")
            && out.contains("\"trust\":\"consensus\""),
        "carries the encyclopedia citation: {out}"
    );
    // `b` is a Latin letter, not one of the 24 Greek rows — honest abstention, never invented.
    assert!(out.contains("\"abstained\":true"), "Latin letter b abstains: {out}");
}
