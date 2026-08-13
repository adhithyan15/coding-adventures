//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/phoneme-addition.adj`) driven through the
//! built CLI: a native THREE-column `table` naming the one phoneme addition
//! (/i/, /s/ -> "ice") walked through on Reading Rockets' "Phonological and
//! Phonemic Awareness: In Practice" module -- the TENTH literacy sub-skill in
//! this loop's curriculum sweep, the narrowest sibling of
//! `phoneme-blending.adj`: the SAME direction (sounds combining into a word)
//! but for exactly TWO sounds rather than three, a distinct arity the cited
//! page treats as its own named skill. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_phonemeadd_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/phoneme-addition.adj");
    std::fs::copy(&src, dir.join("phoneme-addition.adj"))
        .expect("copy shipped phoneme-addition.adj");
}

#[test]
fn phoneme_addition_recall_binds_the_word_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-addition.adj\"\n\
         ? phoneme_addition(i, s, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Word\":\"ice\""),
        "combining /i/ and /s/ gives ice: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn phoneme_addition_reverse_binds_the_two_sounds() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-addition.adj\"\n\
         ? phoneme_addition($S1, $S2, ice)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S1\":\"i\"") && out.contains("\"S2\":\"s\""),
        "ice combines from /i/, /s/: {out}"
    );
}

#[test]
fn phoneme_addition_abstains_honestly_on_an_untabled_addition() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-addition.adj\"\n\
         ? phoneme_addition(m, e, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "m/e -> ? has no shipped row -- honest abstention, never invented: {out}"
    );
}
