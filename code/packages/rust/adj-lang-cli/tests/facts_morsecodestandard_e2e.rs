//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/morse-code-standard.adj`) driven through the
//! built CLI: a native `table` naming the international standard that
//! specifies International Morse code, decoded from a span already sitting
//! unused inside the SAME Wikipedia "Morse code" quote `morse-code.adj`
//! already cites -- a sibling to that table. Resolves binding-query recall
//! (both directions) with the source's citation, and abstains on a code
//! system (american_morse_code) the cited span does not name -- 0 model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_morsecodestandard_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/morse-code-standard.adj");
    std::fs::copy(&src, dir.join("morse-code-standard.adj"))
        .expect("copy shipped morse-code-standard.adj");
}

#[test]
fn morse_code_standard_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"morse-code-standard.adj\"\n\
         ? morse_code_standard(international_morse_code, $StandardId)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"morse_code_standard(international_morse_code, itu_r_m_1677_1)\""),
        "international Morse code is specified by ITU-R M.1677-1: {out}"
    );
    assert!(
        out.contains("wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wikipedia citation: {out}"
    );
}

#[test]
fn morse_code_standard_recalls_backward_from_a_bound_standard_id() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"morse-code-standard.adj\"\n\
         ? morse_code_standard($CodeSystem, itu_r_m_1677_1)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"morse_code_standard(international_morse_code, itu_r_m_1677_1)\""),
        "the standard names international Morse code: {out}"
    );
}

#[test]
fn morse_code_standard_abstains_honestly_on_american_morse_code() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"morse-code-standard.adj\"\n\
         ? morse_code_standard(american_morse_code, $StandardId)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the cited span names no standard for american_morse_code -- honest abstention: {out}"
    );
}


const MORSE_STANDARD_SENTENCE_PIN: &str = r#""bindings":{"StandardId":"itu_r_m_1677_1"},"citations":[{"source":"The Morse code, as specified in the current international standard, International Morse Code Recommendation, ITU-R M.1677-1,[2] was derived from a much-improved proposal by Friedrich Gerke in 1848","locator":"https://en.wikipedia.org/wiki/Morse_code","trust":"consensus""#;

#[test]
fn morse_code_standard_citation_keeps_the_pages_footnote_marker_and_no_full_stop() {
    let dir = scratch("reground_4f");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"morse-code-standard.adj\"\n? morse_code_standard(international_morse_code, $StandardId)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // TWO DEFECTS IN ONE SENTENCE, shipped byte-identically by three
    // libraries. Wikipedia reads:
    //
    //   ... ITU-R M.1677-1,[2] was derived from a much-improved proposal by
    //   Friedrich Gerke in 1848 that became known as the "Hamburg alphabet" ...
    //
    // (1) The footnote marker [2] was dropped. It sits mid-sentence, so no
    //     contiguous span covers the clause without it.
    // (2) A full stop was FABRICATED at "1848." to close a sentence the page
    //     continues. One character on no page, in a field whose contract is
    //     that its bytes are on that page.
    //
    // PINNED IN ALL THREE LIBRARIES. The value is identical in each, so a
    // single pin would leave two free to revert with CI still green.
    assert!(
        out.contains(MORSE_STANDARD_SENTENCE_PIN),
        "the citation matches its page: {out}"
    );
}
