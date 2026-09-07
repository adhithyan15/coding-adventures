//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/silent-letter-sound.adj`) driven through the
//! built CLI: a native `table` naming the University of Florida Literacy
//! Institute (UFLI) Foundations Toolbox's sole "Silent Letters Unit" lesson
//! (lesson 98 of the "Diphthongs and Silent Letters Units (Lessons 95-98)"
//! page -- the same page `diphthong-sound.adj` cites for its own, distinct
//! lessons 95-96) and the single speech sound each of its three named
//! silent-letter consonant-cluster spellings actually represents: `kn` ->
//! n_sound, `wr` -> r_sound, `mb` -> m_sound. Abstains honestly on `gh`, a
//! real silent-letter pattern (as in "night") but not one of this UFLI
//! lesson's three named patterns. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "adjcli_silentlettersound_{tag}_{}",
        std::process::id()
    ));
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
    let src = facts_stdlib().join("language/silent-letter-sound.adj");
    std::fs::copy(&src, dir.join("silent-letter-sound.adj"))
        .expect("copy shipped silent-letter-sound.adj");
}

#[test]
fn silent_letter_sound_recall_binds_the_sound_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-letter-sound.adj\"\n\
         ? silent_letter_sound(kn, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle elsewhere in this
    // file matched only part of the sentence, which let the citation be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once. See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"The Silent Letters Unit only consists of one lesson, but this lesson instructs students on three common silent letter patterns (e.g., kn-, wr-, and -mb). This lesson is designed to build students\u{2019} accuracy and automaticity in recognizing silent letter patterns. The lesson also builds students\u{2019} proficiency in reading and spelling words that contain silent letter patterns.\""),
        "the citation is the page's paragraph, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Sound\":\"n_sound\""),
        "kn makes the n_sound: {out}"
    );
    assert!(
        out.contains("ufli.education.ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the UFLI citation at authoritative trust: {out}"
    );
}

#[test]
fn silent_letter_sound_reverse_binds_the_spelling_for_that_sound() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-letter-sound.adj\"\n\
         ? silent_letter_sound($Sp, r_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Sp\":\"wr\""),
        "the silent-letter spelling that makes r_sound is wr: {out}"
    );
}

#[test]
fn silent_letter_sound_mb_recalls_m_sound() {
    let dir = scratch("mb");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-letter-sound.adj\"\n\
         ? silent_letter_sound(mb, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Sound\":\"m_sound\""),
        "mb makes the m_sound: {out}"
    );
}

#[test]
fn silent_letter_sound_abstains_honestly_on_an_untabled_pattern() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-letter-sound.adj\"\n\
         ? silent_letter_sound(gh, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "gh is a real silent-letter pattern (as in \"night\"), but not one of \
         this UFLI lesson's three named patterns -- honest abstention, never \
         invented: {out}"
    );
}

/// Installment 4g (#13934): the `source` is the cited page's definition
/// PARAGRAPH, not that paragraph with the unit's lesson table stitched
/// onto it.
///
/// Until 4g the field read "...contain silent letter patterns. Diphthongs and Silent Letters Units (Lessons 95-98): 98 kn /n/, wr /r/, mb /m/." -- a string that
/// appears nowhere on the page. It invented a colon after the unit
/// heading, invented separators between each lesson number and the cell
/// before it, and flattened the page's U+2019 apostrophe to an ASCII one.
///
/// The POSITIVE needle is the page's U+2019 apostrophe in "students'",
/// which the stitch flattened to ASCII -- this paragraph is otherwise
/// carried whole by both the old value and the new, so the apostrophe is
/// the discriminating byte. The FIRST negative needle SPANS THE SEAM,
/// joining the paragraph's own last words to the heading welded after them,
/// so it matches only if the weld is back; a needle wholly inside either
/// side would not discriminate. The other two are artifacts the stitch
/// invented outright — the colon after the unit heading, and a lesson
/// number welded to the Concept cell that follows it — neither of which
/// occurs anywhere on the page, checked against the raw HTML rather than
/// the extractor, which normalises whitespace.
///
/// This pin does NOT assert that the rows are cited by that span. They
/// are not: the rows read the page's lesson-table Concept cells, whose
/// status as verbatim spans is the question held open on #14111, and the
/// library's Provenance block says so.
#[test]
fn silent_letter_sound_source_is_the_page_paragraph_not_a_stitched_lesson_table() {
    let dir = scratch("span_not_stitch");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-letter-sound.adj\"\n\
         ? silent_letter_sound(kn, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("builds students\u{2019} proficiency in reading and spelling words that contain silent letter patterns."),
        "the citation carries the page paragraph whole: {out}"
    );
    assert!(
        !out.contains("patterns. Diphthongs and Silent Letters"),
        "the seam itself: the paragraph's last word welded to the unit heading: {out}"
    );
    assert!(
        !out.contains("(Lessons 95-98):"),
        "the colon after the heading, which no cell on the page contains: {out}"
    );
    assert!(
        !out.contains("98 kn /n/"),
        "a lesson number stitched to the Concept cell that follows it: {out}"
    );
}
