//! End-to-end test for the MUSIC FACTS library
//! (`adj-facts-stdlib/music/solfege.adj`) driven through the built CLI: a native
//! `table` of the seven movable-do solfège syllables → their 1-based major-scale
//! degree resolves forward AND reverse binding-query recalls with the
//! encyclopedia's citation, and abstains on a chromatic syllable that has no
//! shipped row — 0 answer-time model calls.
//!
//! The cited envelope states two of the seven rows outright and closes with
//! "etc."; the other five are a DISCLOSED COMPOSITION with the syllable order
//! its corroboration states. `solfege_composed_row_carries_a_citation_that_never_states_its_degree`
//! pins one of those five, so the gap is visible here and not only in the
//! library's header.

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
fn music_solfege_recall_binds_degree_forward_and_reverse() {
    let dir = scratch("solfege");
    // Copy the shipped solfège table beside the entry program and import it.
    let src = facts_stdlib().join("music/solfege.adj");
    std::fs::copy(&src, dir.join("solfege.adj")).expect("copy shipped solfege.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"solfege.adj\"\n\
         ? solfege_degree(do, $N)\n\
         ? solfege_degree(sol, $N)\n\
         ? solfege_degree($S, 3)\n\
         ? solfege_degree(di, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward: do is the tonic (1st degree); sol is the dominant (5th).
    assert!(out.contains("\"N\":\"1\""), "do → 1: {out}");
    assert!(out.contains("\"N\":\"5\""), "sol → 5: {out}");
    // Reverse: the third scale degree is mi (binds the other column).
    assert!(out.contains("\"S\":\"mi\""), "degree 3 → mi: {out}");
    // The answer carries the Wikipedia Solfège citation as its proof, at consensus trust.
    assert!(
        out.contains("wikipedia.org/wiki/Solf") && out.contains("\"trust\":\"consensus\""),
        "carries the encyclopedia citation: {out}"
    );
    // `di` is a chromatic (raised-do) syllable, not one of the 7 diatonic rows —
    // honest abstention, never invented.
    assert!(out.contains("\"abstained\":true"), "chromatic syllable di abstains: {out}");
}

const SOLFEGE_PIN: &str = r#""bindings":{"N":"1"},"citations":[{"source":"In the movable do system, each solfège syllable corresponds not to a pitch, but to a scale degree: The first degree of a major scale is always sung as \"do\", the second as \"re\", etc.","locator":"https://en.wikipedia.org/wiki/Solf%C3%A8ge","trust":"consensus","corroborations":[{"source":"The tonic sol-fa method popularised the seven syllables commonly used in English-speaking countries: do (spelt doh in tonic sol-fa),[2] re, mi, fa, so(l), la, and ti (or si) (see below).","locator":"https://en.wikipedia.org/wiki/Solf%C3%A8ge"}"#;

#[test]
fn solfege_citation_keeps_the_pages_footnote_marker() {
    let dir = scratch("reground");
    std::fs::copy(
        facts_stdlib().join("music/solfege.adj"),
        dir.join("solfege.adj"),
    )
    .expect("copy shipped solfege.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"solfege.adj\"\n? solfege_degree(do, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // TWO REPAIRS, AND THE SECOND ONE ONLY HAPPENED BECAUSE REVIEW ASKED.
    //
    // (1) The syllable sentence had DROPPED the Wikipedia footnote marker "[2]"
    // from the middle of the list and TRUNCATED before " (see below).". The
    // marker is RESTORED, not elided: it sits between "tonic sol-fa)," and
    // "re, mi, fa", and it is in the page's text AND displayed to a reader as a
    // superscript link, so including it is right under both readings of the
    // open document-versus-rendered question. Dropping it was a silent tidy.
    //
    // (2) BUT THAT SENTENCE NEVER MENTIONS A SCALE DEGREE. It lists seven
    // syllables in order, and this row is (do, 1) -- so "1" was an ordinal
    // INFERENCE from list position, not a claim the citation made. This
    // library's own header already conceded it, naming a "movable-do degree
    // table" that was not in the envelope at all. Fixing the sentence's two
    // real defects without asking whether it was the RIGHT sentence is the
    // first-workable-span fault, for the third time in this effort.
    //
    // The `source` is now the page's movable-do sentence, which states the
    // mapping outright. On the page that reads: The first degree of a major
    // scale is always sung as "do", the second as "re". (Unwrapped and
    // unescaped -- the backslashes in the pin above belong to the .adj string
    // literal, not to the page, and writing them here as if they were page
    // text is the same defect the library header declines to commit.)
    // The syllable sentence is a `cites` grounding the SET and ORDER of the
    // syllables -- what it actually says.
    //
    // The pin spans bindings -> envelope -> corroboration so that mutating
    // EITHER repair reddens it.
    assert!(
        out.contains(SOLFEGE_PIN),
        "the solfege citation matches its page: {out}"
    );
}


// A COMPOSED row, cut from the CLI's own output rather than hand-written. The
// envelope states do = 1 and re = 2 and then "etc."; `sol` is the fifth by
// continuing that through the syllable ORDER the corroboration states.
const SOLFEGE_COMPOSED_ROW_PIN: &str = r#""bindings":{"N":"5"},"citations":[{"source":"In the movable do system, each solfège syllable corresponds not to a pitch, but to a scale degree: The first degree of a major scale is always sung as \"do\", the second as \"re\", etc.","locator":"https://en.wikipedia.org/wiki/Solf%C3%A8ge","trust":"consensus","corroborations":[{"source":"The tonic sol-fa method popularised the seven syllables commonly used in English-speaking countries: do (spelt doh in tonic sol-fa),[2] re, mi, fa, so(l), la, and ti (or si) (see below).","locator":"https://en.wikipedia.org/wiki/Solf%C3%A8ge"}]"#;

#[test]
fn solfege_composed_row_carries_a_citation_that_never_states_its_degree() {
    let dir = scratch("composed");
    std::fs::copy(
        facts_stdlib().join("music/solfege.adj"),
        dir.join("solfege.adj"),
    )
    .expect("copy shipped solfege.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"solfege.adj\"\n? solfege_degree(sol, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // WHY THIS TEST EXISTS. Round 1 moved the envelope to the page's movable-do
    // sentence because the old one never mentioned a scale degree. The new one
    // does -- but only for do and re, closing 2 of the 7 rows and leaving 5 as
    // an ordinal inference. The other pin queries `do`, the one row the
    // envelope grounds outright, so nothing in CI could see that gap. This one
    // queries a COMPOSED row and shows exactly what a consumer receives.
    assert!(
        out.contains(SOLFEGE_COMPOSED_ROW_PIN),
        "sol -> 5 carries the movable-do envelope and its corroboration: {out}"
    );
    // POSITIVE CONTROL on the disclosure itself: the cited spans must NOT state
    // this row's DEGREE. Note what the gap is and is not -- the corroboration
    // names `so(l)` outright, and the composition depends on it doing so; what
    // no span does is call that syllable the fifth. So the needle is not a
    // phrase but the digit: strip the binding this answer is disclosing, and
    // nothing resembling `5` may remain anywhere in the citations. A phrase
    // needle ("the fifth") would pass against a span stating the degree
    // numerically, which is the shape a repoint would most likely take.
    let citations_only = SOLFEGE_COMPOSED_ROW_PIN
        .strip_prefix(r#""bindings":{"N":"5"},"#)
        .expect("the pin opens with the binding it discloses");
    assert!(
        !citations_only.contains('5') && !citations_only.contains("fifth"),
        "the cited spans never assign sol a degree -- that is the disclosed gap"
    );
    // The library must SAY so, not merely be true. Prose drifting away from the
    // artifact it describes is this effort's most repeated failure.
    let lib = std::fs::read_to_string(facts_stdlib().join("music/solfege.adj"))
        .expect("read shipped solfege.adj");
    assert!(
        lib.contains("COMPOSED, not stated"),
        "the shipped library discloses which rows are composed"
    );
}
