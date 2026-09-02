//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/onset-rime.adj`) driven through the built
//! CLI: a native THREE-column `table` naming each of two words' onset/rime
//! split, quoted verbatim from Reading Rockets' "Tuning In to the Sounds in
//! Words" article -- the FOURTH literacy sub-skill in this loop's curriculum
//! sweep, deliberately different in shape from `word-families.adj`'s rhyme
//! derivation (RF.K.2.a), `syllable-count.adj`'s syllable recall
//! (RF.K.2.b), and `initial-sound.adj`'s beginning-sound recall (RF.K.2.d):
//! this grounds onset/rime blending and segmenting (RF.K.2.c). 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_onsetrime_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/onset-rime.adj");
    std::fs::copy(&src, dir.join("onset-rime.adj")).expect("copy shipped onset-rime.adj");
}

#[test]
fn onset_rime_recall_segments_the_word_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"onset-rime.adj\"\n\
         ? onset_rime(sleep, $Onset, $Rime)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Onset\":\"sl\"") && out.contains("\"Rime\":\"eep\""),
        "sleep splits into sl + eep: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn onset_rime_reverse_blends_the_parts_into_the_word() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"onset-rime.adj\"\n\
         ? onset_rime($Word, bl, ast)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Word\":\"blast\""),
        "bl + ast blends into blast: {out}"
    );
}

#[test]
fn onset_rime_abstains_honestly_on_an_unshipped_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"onset-rime.adj\"\n\
         ? onset_rime(cat, $Onset, $Rime)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"cat\" has no shipped row -- honest abstention, never invented: {out}"
    );
}

#[test]
fn onset_rime_extension_recalls_the_newly_added_map_and_tape_splits() {
    let dir = scratch("ext");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"onset-rime.adj\"\n\
         ? onset_rime($Word, m, ap)\n\
         ? onset_rime(tape, $Onset, $Rime)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Discovered via a fresh WebFetch of Reading Rockets' "In Practice"
    // module's "Blending Onset and Rime" and "Onset-rime Completion"
    // sections -- both new rows share the table's existing schema, a pure
    // addition alongside the original sleep/blast rows.
    assert!(
        out.contains("\"Word\":\"map\""),
        "m + ap blends into map (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Onset\":\"t\"") && out.contains("\"Rime\":\"ape\""),
        "tape splits into t + ape (added this cycle): {out}"
    );
}

const MAP_PIN: &str = r#""bindings":{"Word":"map"},"citations":[{"source":"For example, sleep could be broken into /sl/ and /eep/.","locator":"https://www.readingrockets.org/topics/phonological-and-phonemic-awareness/articles/tuning-sounds-words","trust":"consensus","corroborations":[{"source":"So in the word “map,” /m/ is the onset and /ap/ is the rime.","locator":"https://www.readingrockets.org/reading-101/reading-101-learning-modules/course-modules/phonological-and-phonemic-awareness/practice""#;

const TAPE_PIN: &str = r#""bindings":{"Onset":"t","Rime":"ape"},"citations":[{"source":"For example, sleep could be broken into /sl/ and /eep/.","locator":"https://www.readingrockets.org/topics/phonological-and-phonemic-awareness/articles/tuning-sounds-words","trust":"consensus","corroborations":[{"source":"So in the word “map,” /m/ is the onset and /ap/ is the rime.","locator":"https://www.readingrockets.org/reading-101/reading-101-learning-modules/course-modules/phonological-and-phonemic-awareness/practice"},{"source":"The word is ‘tape’. The first part is /t/ [we put right fist on the table]. What’s the rest of the word? /Ape/ [we put left fist on table].","locator":"https://www.readingrockets.org/reading-101/reading-101-learning-modules/course-modules/phonological-and-phonemic-awareness/practice""#;

#[test]
fn onset_rime_map_answer_carries_its_reading_rockets_corroboration_intact() {
    let dir = scratch("cite_map");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"onset-rime.adj\"\n? onset_rime($Word, m, ap)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // ANCHORED and JOINT: bindings + the full envelope + the added
    // corroboration as ONE contiguous span, ending on a closing quote. Two
    // separate `contains` scans over this blob could not tell which answer a
    // citation belonged to, and a truncated quote would still pass.
    //
    // NOTE the curly quotes: the page renders U+201C/U+201D around "map," and
    // the library header previously flattened them to ASCII. Pinning the real
    // glyphs is what stops that regressing.
    //
    // This does NOT assert row-scoped provenance -- `cites` is table-scoped,
    // so every answer carries the same corroboration list. It asserts that
    // THIS answer carries THIS evidence uncorrupted.
    assert!(
        out.contains(MAP_PIN),
        "map's answer carries the In Practice corroboration verbatim: {out}"
    );
}

#[test]
fn onset_rime_tape_answer_carries_its_bracketed_stage_directions_verbatim() {
    let dir = scratch("cite_tape");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"onset-rime.adj\"\n? onset_rime(tape, $Onset, $Rime)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The page states tape's onset and rime across one block that also
    // contains bracketed stage directions ("[we put right fist on the
    // table]"). The library header used to elide those with "...", which is a
    // CONSTRUCTED span -- text the page never displays as one run. The quote
    // is carried whole instead, brackets included, and pinned that way here.
    //
    // `blast` gets no such pin ON PURPOSE: its lead-in is a <p> and its split
    // is an <li>, so no single rendered span names the word AND gives its
    // split. Leaving it uncited with the reason recorded is the result.
    assert!(
        out.contains(TAPE_PIN),
        "tape's answer keeps the page's bracketed asides: {out}"
    );
}
