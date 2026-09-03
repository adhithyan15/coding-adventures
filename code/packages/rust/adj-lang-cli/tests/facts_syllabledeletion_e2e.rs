//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/syllable-deletion.adj`) driven through the
//! built CLI: a native THREE-column `table` naming the one syllable
//! deletion ("pencil" -> "pen", removing "cil") walked through on Reading
//! Rockets' "Phonological and Phonemic Awareness: In Practice" module --
//! the EIGHTH literacy sub-skill in this loop's curriculum sweep, the
//! syllable-level analogue of `phoneme-deletion.adj` the same way
//! `syllable-substitution.adj` is the syllable-level analogue of
//! `phoneme-substitution.adj`. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_syllabledel_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/syllable-deletion.adj");
    std::fs::copy(&src, dir.join("syllable-deletion.adj")).expect("copy shipped syllable-deletion.adj");
}

#[test]
fn syllable_deletion_recall_binds_the_new_word_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-deletion.adj\"\n\
         ? syllable_deletion(pencil, cil, $New)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"New\":\"pen\""),
        "removing pencil's second syllable 'cil' gives pen: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn syllable_deletion_reverse_binds_the_original_word_and_syllable() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-deletion.adj\"\n\
         ? syllable_deletion($Orig, $Syllable, pen)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Orig\":\"pencil\"") && out.contains("\"Syllable\":\"cil\""),
        "pen came from pencil by removing 'cil': {out}"
    );
}

#[test]
fn syllable_deletion_abstains_honestly_on_an_untabled_deletion() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-deletion.adj\"\n\
         ? syllable_deletion(basket, bas, $New)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "basket -> ? via removing 'bas' has no shipped row -- honest abstention, never invented: {out}"
    );
}


const DELETION_PENCIL_PIN: &str = r#""bindings":{"New":"pen"},"citations":[{"source":"I take away ‘cil’ [remove the card]. ‘Pen’ is left [touch remaining card].","locator":"https://www.readingrockets.org/reading-101/reading-101-learning-modules/course-modules/phonological-and-phonemic-awareness/practice","trust":"consensus""#;

#[test]
fn syllable_deletion_citation_is_one_span_not_three_stitched_fragments() {
    let dir = scratch("reground_4e");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-deletion.adj\"\n? syllable_deletion(pencil, cil, $New)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THE OLD VALUE WAS ASSEMBLED, NOT QUOTED. It read
    //
    //   The word is: 'pencil'. I take away 'cil'. 'Pen' is left.
    //
    // but those three fragments are not adjacent. The page runs them across
    // intervening sentences with bracketed stage directions between:
    //
    //   The word is: 'pencil'. 'Pen' 'cil' [place a card ...]. 'Pencil'
    //   [sweep finger ...]. I take away 'cil' [remove the card]. 'Pen' is
    //   left [touch remaining card].
    //
    // Three fragments stitched together, every stage direction deleted, and
    // the result presented as a span. Its 25% triage overlap said so.
    //
    // The replacement is the page's own contiguous sentence pair, which
    // states this table's deletion in one breath and is BETTER evidence than
    // the stitched version, not merely more honest. The dropped lead-in named
    // the word, which the row's own first column already carries.
    assert!(
        out.contains(DELETION_PENCIL_PIN),
        "the deletion citation is a real span of its page: {out}"
    );
}
