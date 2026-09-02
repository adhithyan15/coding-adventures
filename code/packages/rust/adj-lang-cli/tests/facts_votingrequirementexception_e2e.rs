//! End-to-end test for the civics FACTS library
//! (`adj-facts-stdlib/civics/voting-requirement-exception.adj`) driven
//! through the built CLI: a native `table` holding the carve-out USA.gov
//! states for each U.S. voting requirement, grounding its "Who can and
//! cannot vote" page.
//!
//! The EIGHTH library in the `civics/` domain, and the first from the
//! voting pages. Its axis is deliberately the exceptions rather than the
//! requirements: the four requirements are a flat bulleted list with no
//! second column the source states, so a `requirement -> description`
//! table would have had to paraphrase. The page attaches exactly one
//! stated carve-out to each bullet, so `requirement -> exception` is a
//! relation the source supplies on BOTH sides, uniformly, with nothing
//! invented.
//!
//! It is also the half that a confident summary drops. Ask a model "do you
//! have to register to vote?" and it will say yes; that North Dakota
//! requires no voter registration at all is exactly the detail that
//! disappears. Recalling that carve-out WITH its citation is the behaviour
//! this stdlib exists to make possible, so the first test below asserts it
//! specifically. 0 answer-time model calls.

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
        "adjcli_factsvotereqexc_{tag}_{}",
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
    let src = facts_stdlib().join("civics/voting-requirement-exception.adj");
    std::fs::copy(&src, dir.join("voting-requirement-exception.adj"))
        .expect("copy shipped voting-requirement-exception.adj");
}

#[test]
fn voting_requirement_exception_recalls_the_carve_out_a_summary_would_drop() {
    let dir = scratch("carveout");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"voting-requirement-exception.adj\"\n\
         ? voting_requirement_exception(voter_registration_by_state_deadline, $E)\n",
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
        out.contains("\"source\":\"North Dakota does not require voter registration.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // "You must register to vote" is what a confident summary produces.
    // The source says North Dakota does not require registration at all.
    assert!(
        out.contains("\"E\":\"north_dakota_does_not_require_registration\""),
        "recalls the registration carve-out: {out}"
    );
    assert!(
        out.contains("North Dakota does not require voter registration."),
        "carries the carve-out sentence verbatim: {out}"
    );
    assert!(
        out.contains("usa.gov/who-can-vote") && out.contains("\"trust\":\"authoritative\""),
        "carries the USA.gov citation: {out}"
    );
}

#[test]
fn voting_requirement_exception_covers_all_four_requirements() {
    let dir = scratch("all");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"voting-requirement-exception.adj\"\n\
         ? voting_requirement_exception($R, $E)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The page attaches a stated carve-out to every one of its four
    // requirement bullets -- the uniformity is what makes this axis
    // honest, so a row going missing should fail loudly.
    for (requirement, exception) in [
        ("us_citizenship", "non_citizens_may_vote_in_some_local_elections_only"),
        ("state_residency", "experiencing_homelessness_still_meets_it"),
        (
            "age_eighteen_by_election_day",
            "some_states_allow_seventeen_year_olds_in_primaries",
        ),
        (
            "voter_registration_by_state_deadline",
            "north_dakota_does_not_require_registration",
        ),
    ] {
        assert!(
            out.contains(&format!(
                "voting_requirement_exception({requirement}, {exception})"
            )),
            "{requirement} carries its stated exception: {out}"
        );
    }
}

#[test]
fn voting_requirement_exception_carries_every_grounding_sentence() {
    let dir = scratch("cites");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"voting-requirement-exception.adj\"\n\
         ? voting_requirement_exception(state_residency, $E)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Four rows from four different sentences: one `source` plus three
    // `cites`. Each row must stay auditable back to its own sentence.
    for sentence in [
        "North Dakota does not require voter registration.",
        "some areas allow non-citizens to vote in local elections only",
        "You can be experiencing homelessness and still meet these requirements.",
        "Some states allow 17-year-olds who will be 18 by Election Day to vote in primaries.",
    ] {
        assert!(
            out.contains(sentence),
            "grounding sentence carried: {sentence}: {out}"
        );
    }
}

#[test]
fn voting_requirement_exception_runs_backward_from_exception_to_requirement() {
    let dir = scratch("reverse");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"voting-requirement-exception.adj\"\n\
         ? voting_requirement_exception($R, experiencing_homelessness_still_meets_it)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains(
            "voting_requirement_exception(state_residency, experiencing_homelessness_still_meets_it)"
        ),
        "homelessness is the residency carve-out: {out}"
    );
}

#[test]
fn voting_requirement_exception_abstains_on_disqualifications() {
    let dir = scratch("abstain");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"voting-requirement-exception.adj\"\n\
         ? voting_requirement_exception(felony_conviction, $E)\n\
         ? voting_requirement_exception($R, cannot_vote_for_president)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The page's separate "Who cannot vote?" section states
    // DISQUALIFICATIONS -- felony conviction, mental disability, residing in
    // a U.S. territory. Those are the OPPOSITE kind of claim from "you
    // still qualify despite X", and folding them in here would blur the
    // two. They belong in their own table.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "abstains on disqualifications rather than blurring them with exceptions: {out}"
    );
}
