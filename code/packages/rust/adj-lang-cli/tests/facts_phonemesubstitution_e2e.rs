//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/phoneme-substitution.adj`) driven through the
//! built CLI: a native FOUR-column `table` naming the one phoneme
//! substitution ("make" -> "bake", /m/ -> /b/) walked through on Reading
//! Rockets' "Phonological and Phonemic Awareness: In Practice" module --
//! the FIFTH literacy sub-skill in this loop's curriculum sweep,
//! deliberately different from `word-families.adj` (RF.K.2.a),
//! `syllable-count.adj` (RF.K.2.b), `onset-rime.adj` (RF.K.2.c), and
//! `initial-sound.adj` (RF.K.2.d): this grounds phoneme substitution
//! (RF.K.2.e). 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_phonemesub_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/phoneme-substitution.adj");
    std::fs::copy(&src, dir.join("phoneme-substitution.adj"))
        .expect("copy shipped phoneme-substitution.adj");
}

#[test]
fn phoneme_substitution_recall_binds_the_new_word_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-substitution.adj\"\n\
         ? phoneme_substitution(make, m, b, $New)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"New\":\"bake\""),
        "changing make's /m/ to /b/ gives bake: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn phoneme_substitution_reverse_binds_the_original_word_and_sound() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-substitution.adj\"\n\
         ? phoneme_substitution($Orig, $OrigSound, b, bake)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Orig\":\"make\"") && out.contains("\"OrigSound\":\"m\""),
        "bake came from make by changing /m/: {out}"
    );
}

#[test]
fn phoneme_substitution_abstains_honestly_on_an_untabled_substitution() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-substitution.adj\"\n\
         ? phoneme_substitution(cat, c, b, $New)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "cat -> ? via /c/ -> /b/ has no shipped row -- honest abstention, never invented: {out}"
    );
}

const PHONEME_SUBSTITUTION_PIN: &str = r#""bindings":{"New":"bake"},"citations":[{"source":"I can change one sound in a word to form a new word. Watch me. I will change ‘make’ to ‘bake’.","locator":"https://www.readingrockets.org/reading-101/reading-101-learning-modules/course-modules/phonological-and-phonemic-awareness/practice","trust":"consensus""#;

#[test]
fn phoneme_substitution_citation_matches_its_page_glyph_for_glyph() {
    let dir = scratch("glyph");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-substitution.adj\"
? phoneme_substitution(make, m, b, $New)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The page quotes this word with PAIRED curly quotes -- U+2018 opening and
    // U+2019 closing -- and the shipped citation had flattened both to ASCII.
    // This is NOT the contraction case: curling both ends the same way yields
    // a form that appears on no page. The replacement was confirmed present in
    // a rendered block before being written here.
    assert!(
        out.contains(PHONEME_SUBSTITUTION_PIN),
        "the phoneme substitution citation matches its page: {out}"
    );
}
