//! End-to-end test for the civics FACTS library
//! (`adj-facts-stdlib/civics/electoral-college-count.adj`) driven through
//! the built CLI: a native `table` holding the numbers USA.gov states
//! about the U.S. Electoral College, grounding its "Electoral College"
//! page.
//!
//! The NINTH library in the `civics/` domain and the first about elections
//! themselves. Exact numbers are precisely the sort of fact that should
//! come from a citation rather than a model's recollection, so the tests
//! below check the bindings AND that the sentence stating each number
//! rides along.
//!
//! One assertion here is a deliberate encoding regression guard: the "270
//! electors" sentence contains U+2014 EM DASHes, and this test asserts the
//! character survives the round trip through the CLI's JSON output. (While
//! authoring, a check that piped stdout through a Python reader appeared to
//! show the dash mangled; that turned out to be the reader decoding UTF-8
//! with the Windows default codepage, not a defect in the CLI, which emits
//! correct UTF-8. Asserting it from Rust -- where `String::from_utf8`
//! handles the bytes correctly -- pins the real behaviour so a future
//! reader is not sent chasing the same phantom.)
//! 0 answer-time model calls.

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
        "adjcli_factselectoralcount_{tag}_{}",
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

fn place(dir: &Path) {
    let src = facts_stdlib().join("civics/electoral-college-count.adj");
    std::fs::copy(&src, dir.join("electoral-college-count.adj"))
        .expect("copy shipped electoral-college-count.adj");
}

#[test]
fn electoral_college_count_binds_the_winning_threshold_with_its_sentence() {
    let dir = scratch("threshold");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"electoral-college-count.adj\"\n\
         ? electoral_college_count(electors_needed_to_win, $N)\n",
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
        out.contains("\"source\":\"Including Washington, D.C.'s three electors, there are currently 538 electors in all.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"N\":\"270\""), "270 electors to win: {out}");
    // An exact number must arrive with the sentence that states it.
    assert!(
        out.contains("A candidate needs the vote of at least 270 electors"),
        "carries the threshold sentence: {out}"
    );
    assert!(
        out.contains("usa.gov/electoral-college") && out.contains("\"trust\":\"authoritative\""),
        "carries the USA.gov citation: {out}"
    );
}

#[test]
fn electoral_college_count_preserves_the_em_dashes_in_its_citation() {
    let dir = scratch("emdash");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"electoral-college-count.adj\"\n\
         ? electoral_college_count(electors_needed_to_win, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // U+2014 EM DASH, not a hyphen. The whole value of a `cites` string is
    // that it is byte-faithful to the source, so the exact punctuation is
    // part of the claim rather than incidental formatting.
    assert!(
        out.contains(
            "A candidate needs the vote of at least 270 electors\u{2014}more than half of all electors\u{2014}to win the presidential election."
        ),
        "the em-dashed sentence survives the round trip byte-for-byte: {out}"
    );
    assert!(
        !out.contains("270 electors-more"),
        "the em dash must not have degraded to a hyphen: {out}"
    );
}

#[test]
fn electoral_college_count_covers_every_stated_number() {
    let dir = scratch("all");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"electoral-college-count.adj\"\n\
         ? electoral_college_count($Q, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for (quantity, count) in [
        ("total_electors", "538"),
        ("district_of_columbia_electors", "3"),
        ("electors_needed_to_win", "270"),
        ("winner_take_all_states", "48"),
    ] {
        assert!(
            out.contains(&format!("electoral_college_count({quantity}, {count})")),
            "{quantity} is {count}: {out}"
        );
    }
}

#[test]
fn electoral_college_count_carries_the_sources_own_currently_hedge() {
    let dir = scratch("hedge");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"electoral-college-count.adj\"\n\
         ? electoral_college_count(total_electors, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"N\":\"538\""), "538 electors in all: {out}");
    // The source says there are CURRENTLY 538 electors. The total tracks
    // congressional apportionment, so it is stable but not fixed. The
    // qualifier travels with the answer rather than the reader inheriting a
    // bare number presented as timeless.
    assert!(
        out.contains("there are currently 538 electors in all."),
        "the source's own 'currently' rides along with the count: {out}"
    );
}

#[test]
fn electoral_college_count_abstains_on_methods_and_on_historical_counts() {
    let dir = scratch("abstain");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"electoral-college-count.adj\"\n\
         ? electoral_college_count(proportional_states, $N)\n\
         ? electoral_college_count($Q, 2)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Maine and Nebraska assign electors by a proportional METHOD; the page
    // states no count for them, and it is why those two are excluded from
    // the 48. And "this has happened twice" counts HISTORICAL OCCURRENCES
    // of the House deciding a presidential election -- a different subject
    // from the size of the College, so 2 must not bind here.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "abstains on the method and on the historical count: {out}"
    );
}
