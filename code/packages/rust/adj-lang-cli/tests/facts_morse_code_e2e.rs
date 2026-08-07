//! End-to-end test for the LANGUAGE FACTS library
//! (`adj-facts-stdlib/language/morse-code.adj`) driven through the built CLI:
//! a native `table` of the 26 Latin letters → their International Morse code
//! pattern (as ASCII word-atoms like `dot_dot_dot`) resolves forward AND reverse
//! binding-query recalls with the ITU-standard citation, and abstains on a digit
//! that has no shipped row — 0 answer-time model calls.

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
fn language_morse_code_recall_binds_pattern_forward_and_reverse() {
    let dir = scratch("morse_code");
    // Copy the shipped Morse table beside the entry program and import it.
    let src = facts_stdlib().join("language/morse-code.adj");
    std::fs::copy(&src, dir.join("morse-code.adj")).expect("copy shipped morse-code.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"morse-code.adj\"\n\
         ? morse_code(s, $P)\n\
         ? morse_code(o, $P)\n\
         ? morse_code(e, $P)\n\
         ? morse_code($L, dash)\n\
         ? morse_code(5, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward: the S/O/E of the classic SOS distress rhythm, and E = the single dot.
    assert!(out.contains("\"P\":\"dot_dot_dot\""), "s → dot_dot_dot: {out}");
    assert!(out.contains("\"P\":\"dash_dash_dash\""), "o → dash_dash_dash: {out}");
    assert!(out.contains("\"P\":\"dot\""), "e → dot: {out}");
    // Reverse: a single dash is the letter t (binds the other column).
    assert!(out.contains("\"L\":\"t\""), "pattern dash → t: {out}");
    // The answer carries the ITU / Wikipedia citation as its proof, at consensus trust.
    assert!(
        out.contains("en.wikipedia.org/wiki/Morse_code") && out.contains("\"trust\":\"consensus\""),
        "carries the Morse-code citation: {out}"
    );
    // `5` is a digit (Morse numerals are a separate set), not one of the 26 letter
    // rows — honest abstention, never invented.
    assert!(out.contains("\"abstained\":true"), "digit 5 abstains: {out}");
}
