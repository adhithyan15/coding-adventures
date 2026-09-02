//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/syllable-blending.adj`) driven through the
//! built CLI: a native THREE-column `table` naming the one syllable blend
//! ("lap", "top" -> "laptop") walked through on Reading Rockets'
//! "Phonological and Phonemic Awareness: In Practice" module -- the
//! THIRTEENTH literacy sub-skill in this loop's curriculum sweep,
//! deliberately the OPPOSITE direction from `syllable-segmentation.adj`:
//! this grounds syllable blending (combining separate syllables into a
//! word) rather than decomposing one. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_syllableblend_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/syllable-blending.adj");
    std::fs::copy(&src, dir.join("syllable-blending.adj"))
        .expect("copy shipped syllable-blending.adj");
}

#[test]
fn syllable_blending_recall_binds_the_word_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-blending.adj\"\n\
         ? syllable_blending(lap, top, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Word\":\"laptop\""),
        "blending lap/top gives laptop: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn syllable_blending_reverse_binds_the_two_syllables() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-blending.adj\"\n\
         ? syllable_blending($S1, $S2, laptop)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S1\":\"lap\"") && out.contains("\"S2\":\"top\""),
        "laptop reverse-binds to lap/top: {out}"
    );
}

#[test]
fn syllable_blending_abstains_honestly_on_an_untabled_pair() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-blending.adj\"\n\
         ? syllable_blending(pea, nut, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "pea/nut -> ? has no shipped row -- honest abstention, never invented: {out}"
    );
}

const SYLLABLE_BLENDING_PIN: &str = r#""bindings":{"Word":"laptop"},"citations":[{"source":"I can say each syllable in a word and then I can blend the syllables to say the word. As I say each syllable, I will lay down a card. I will lay the cards left to right. Watch me. I say each syllable and put down a card: ‘lap’ [place a card] ‘top’ [place a card so it appears left-to-right for students]. Now I sweep my finger below the cards and say the whole word: ‘laptop’ [sweep finger below the cards left-to-right].","locator":"https://www.readingrockets.org/reading-101/reading-101-learning-modules/course-modules/phonological-and-phonemic-awareness/practice","trust":"consensus""#;

#[test]
fn syllable_blending_citation_matches_its_page_glyph_for_glyph() {
    let dir = scratch("glyph");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-blending.adj\"
? syllable_blending(lap, top, $Word)
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
        out.contains(SYLLABLE_BLENDING_PIN),
        "the syllable blending citation matches its page: {out}"
    );
}
