//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/syllable-segmentation.adj`) driven through
//! the built CLI: a native THREE-column `table` naming the two syllable
//! parts of each of four words (peanut/pencil/sunset/laptop) walked through
//! on Reading Rockets' "Phonological and Phonemic Awareness: In Practice"
//! module -- a sibling to the already-shipped `syllable-count.adj` (which
//! recalls only the COUNT, not the actual syllable text). 0 answer-time
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
    let dir = std::env::temp_dir().join(format!("adjcli_syllableseg_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/syllable-segmentation.adj");
    std::fs::copy(&src, dir.join("syllable-segmentation.adj"))
        .expect("copy shipped syllable-segmentation.adj");
}

#[test]
fn syllable_segmentation_recall_binds_the_two_syllables_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-segmentation.adj\"\n\
         ? syllable_segmentation(peanut, $S1, $S2)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"S1\":\"pea\"") && out.contains("\"S2\":\"nut\""),
        "peanut splits into pea/nut: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn syllable_segmentation_reverse_binds_the_word() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-segmentation.adj\"\n\
         ? syllable_segmentation($Word, sun, set)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Word\":\"sunset\""),
        "sun/set segments from sunset: {out}"
    );
}

#[test]
fn syllable_segmentation_covers_all_four_shipped_words() {
    let dir = scratch("allfour");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-segmentation.adj\"\n\
         ? syllable_segmentation(pencil, $S1, $S2)\n\
         ? syllable_segmentation(laptop, $S1, $S2)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"syllable_segmentation(pencil, pen, cil)\""),
        "pencil splits into pen/cil: {out}"
    );
    assert!(
        out.contains("\"term\":\"syllable_segmentation(laptop, lap, top)\""),
        "laptop splits into lap/top: {out}"
    );
}

#[test]
fn syllable_segmentation_abstains_honestly_on_an_untabled_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-segmentation.adj\"\n\
         ? syllable_segmentation(pretzel, $S1, $S2)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "pretzel -> ? has no shipped row -- honest abstention, never invented: {out}"
    );
}
