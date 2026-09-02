//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/phoneme-segmentation.adj`) driven through the
//! built CLI: a native FOUR-column `table` naming the one word segmentation
//! ("feet" -> /f/, /ee/, /t/) walked through on Reading Rockets'
//! "Phonological and Phonemic Awareness: In Practice" module -- the
//! ELEVENTH literacy sub-skill in this loop's curriculum sweep, the exact
//! OPPOSITE direction from `phoneme-blending.adj`: this decomposes a word
//! into its sounds rather than composing sounds into a word. 0 answer-time
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_phonemeseg_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/phoneme-segmentation.adj");
    std::fs::copy(&src, dir.join("phoneme-segmentation.adj"))
        .expect("copy shipped phoneme-segmentation.adj");
}

#[test]
fn phoneme_segmentation_recall_binds_the_three_sounds_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-segmentation.adj\"\n\
         ? phoneme_segmentation(feet, $S1, $S2, $S3)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"S1\":\"f\"") && out.contains("\"S2\":\"ee\"") && out.contains("\"S3\":\"t\""),
        "feet segments into /f/, /ee/, /t/: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn phoneme_segmentation_reverse_binds_the_word() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-segmentation.adj\"\n\
         ? phoneme_segmentation($Word, f, ee, t)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Word\":\"feet\""),
        "f/ee/t segments from feet: {out}"
    );
}

#[test]
fn phoneme_segmentation_abstains_honestly_on_an_untabled_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-segmentation.adj\"\n\
         ? phoneme_segmentation(cat, $S1, $S2, $S3)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "cat -> ? has no shipped row -- honest abstention, never invented: {out}"
    );
}

const PHONEME_SEGMENTATION_PIN: &str = r#""bindings":{"S1":"f","S2":"ee","S3":"t"},"citations":[{"source":"Now I will segment sounds in a word, and I will use the caps to show each sound. Watch me. The word is ‘feet’, /f/ [slide a cap], /ee/ [slide a cap], /t/ [slide a cap], feet [sweep finger below the caps blending the sounds].","locator":"https://www.readingrockets.org/reading-101/reading-101-learning-modules/course-modules/phonological-and-phonemic-awareness/practice","trust":"consensus""#;

#[test]
fn phoneme_segmentation_citation_matches_its_page_glyph_for_glyph() {
    let dir = scratch("glyph");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-segmentation.adj\"
? phoneme_segmentation(feet, $S1, $S2, $S3)
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
        out.contains(PHONEME_SEGMENTATION_PIN),
        "the phoneme segmentation citation matches its page: {out}"
    );
}
