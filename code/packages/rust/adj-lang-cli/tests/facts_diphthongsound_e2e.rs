//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/diphthong-sound.adj`) driven through the
//! built CLI: a native `table` naming the two diphthong lessons of the
//! University of Florida Literacy Institute (UFLI) Foundations Toolbox's
//! "Diphthongs and Silent Letters Units (Lessons 95-98)" page and the
//! single glided vowel sound each spelling represents. `oi`/`oy` share the
//! same sound (lesson 95) and `ou`/`ow` share the same sound (lesson 96) --
//! a genuine many-keys-to-one-sound shape, the mirror image of
//! `digraph-sound.adj`'s one-key-to-many-sounds `th` case. Abstains
//! honestly on `au`, a spelling UFLI's own broader scope and sequence
//! tables under a DIFFERENT unit ("Other Vowel Teams", lesson 93), not this
//! cited Diphthongs page. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_diphthongsound_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/diphthong-sound.adj");
    std::fs::copy(&src, dir.join("diphthong-sound.adj")).expect("copy shipped diphthong-sound.adj");
}

#[test]
fn diphthong_sound_recall_binds_the_sound_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound(oi, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle in this file
    // matched only part of the sentence, so the citation could be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once.
    //
    // Several tests load this library, because siblings import it as a
    // dependency. The pin belongs in its OWN test: the others are not
    // responsible for its provenance. That is also why the assertion has
    // to be unique -- where a co-loaded sibling carries a byte-identical
    // citation, an assertion either one satisfies pins neither.
    // See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"A Diphthong is sound produced by combining two vowels, gliding the tongue from one position to another during articulation (e.g., /ow/, /oy/). These lessons are designed to build students\u{2019} accuracy and automaticity in recognizing diphthongs. The lessons also build students\u{2019} proficiency in reading and spelling words that contain diphthongs.\""),
        "the citation is the page's paragraph, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Sound\":\"oi_sound\""),
        "oi makes the oi_sound: {out}"
    );
    assert!(
        out.contains("ufli.education.ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the UFLI citation at authoritative trust: {out}"
    );
}

#[test]
fn diphthong_sound_reverse_binds_both_spellings_of_oi_sound() {
    let dir = scratch("reverse_oi");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound($D, oi_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Many-keys-to-one-sound: the cited page's own lesson 95 pairs BOTH "oi"
    // and "oy" with the same /oi/ sound, so a backward recall on the
    // sound must yield both spellings, not just one.
    assert!(out.contains("\"D\":\"oi\""), "oi carries oi_sound: {out}");
    assert!(out.contains("\"D\":\"oy\""), "oy carries oi_sound too: {out}");
}

#[test]
fn diphthong_sound_reverse_binds_both_spellings_of_ow_sound() {
    let dir = scratch("reverse_ow");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound($D, ow_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Lesson 96 pairs BOTH "ou" and "ow" with the same /ow/ sound.
    assert!(out.contains("\"D\":\"ou\""), "ou carries ow_sound: {out}");
    assert!(out.contains("\"D\":\"ow\""), "ow carries ow_sound too: {out}");
}

#[test]
fn diphthong_sound_forward_ow_recalls_ow_sound() {
    let dir = scratch("forward_ow");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound(ow, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Sound\":\"ow_sound\""),
        "ow makes the ow_sound: {out}"
    );
}

#[test]
fn diphthong_sound_abstains_honestly_on_a_different_ufli_unit() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound(au, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "au is tabled by UFLI under a DIFFERENT unit (Other Vowel Teams, \
         lesson 93), not this cited Diphthongs page -- honest abstention, \
         never invented: {out}"
    );
}

/// Installment 4g (#13934): the `source` is the cited page's definition
/// PARAGRAPH, not that paragraph with the unit's lesson table stitched
/// onto it.
///
/// Until 4g the field read "...contain diphthongs. Diphthongs and Silent Letters Units (Lessons 95-98): 95 oi /oi/, oy /oi/, ..." -- a string that
/// appears nowhere on the page. It invented a colon after the unit
/// heading, invented separators between each lesson number and the cell
/// before it, and flattened the page's U+2019 apostrophe to an ASCII one,
/// twice. It carried all three of the paragraph's sentences and dropped
/// none.
///
/// The POSITIVE needle is the page's U+2019 apostrophe in "students'",
/// which the stitch flattened to ASCII -- this paragraph is otherwise
/// carried whole by both the old value and the new, so the apostrophe is
/// the discriminating byte. The FIRST negative needle SPANS THE SEAM,
/// joining the paragraph's own last words to the heading welded after them,
/// so it matches only if the weld is back; a needle wholly inside either
/// side would not discriminate. The other two are artifacts the stitch
/// invented outright, and they are not the same kind of artifact. The colon
/// after the unit heading occurs nowhere on the page under any
/// normalisation. The lesson number welded to the Concept cell after it is
/// subtler, and worth being exact about: it is NOT absent from the page.
/// Normalise the page's whitespace and "95 oi /oi/" appears, because the
/// lesson cell and the Concept cell are adjacent. What the old value
/// invented was that string as a CONTIGUOUS span with the cell boundary
/// erased, in a field whose whole contract is that it holds one. (Checking
/// an ABSENCE against tag-bearing raw HTML would be the WEAKER test, not
/// the stronger one: embedded markup guarantees a non-match. Raw HTML is
/// the right tool for a PRESENCE claim, which is the opposite direction.)
///
/// This pin does NOT assert that the rows are cited by that span. They
/// are not: the rows read the page's lesson-table Concept cells, whose
/// status as verbatim spans is the question held open on #14111, and the
/// library's Provenance block says so.
#[test]
fn diphthong_sound_source_is_the_page_paragraph_not_a_stitched_lesson_table() {
    let dir = scratch("span_not_stitch");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound(oi, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("build students\u{2019} accuracy and automaticity in recognizing diphthongs."),
        "the citation carries the page paragraph whole: {out}"
    );
    assert!(
        !out.contains("diphthongs. Diphthongs and Silent Letters"),
        "the seam itself: the paragraph's last word welded to the unit heading: {out}"
    );
    assert!(
        !out.contains("(Lessons 95-98):"),
        "the colon after the heading, which no cell on the page contains: {out}"
    );
    assert!(
        !out.contains("95 oi /oi/"),
        "a lesson number stitched to the Concept cell that follows it: {out}"
    );
}
