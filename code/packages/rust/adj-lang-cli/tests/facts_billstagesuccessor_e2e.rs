//! End-to-end test for the civics FACTS library
//! (`adj-facts-stdlib/civics/bill-stage-successor.adj`) driven through the
//! built CLI: a native `table` recording which stage a bill moves to next
//! on its way through Congress, grounding USA.gov's "How laws are made"
//! page.
//!
//! The SEVENTH library in the `civics/` domain, and the first ordered
//! sequence in this stdlib expressed as a SUCCESSOR relation rather than
//! an ordinal position. That choice is the point of the slice: the four
//! existing ordered tables (`moon_phase_order`, `planet_order`,
//! `mitosis_phase_order`, `sedimentary_rock_formation_step`) all decode
//! sources that state POSITIONS, while this source states TRANSITIONS in
//! continuous prose -- "Once a bill is introduced, it is assigned to a
//! committee...", "The bill is THEN put before that chamber...". Assigning
//! absolute step numbers would have invented an answer to a question the
//! page never addresses (where does the count start?), and no test would
//! have caught it.
//!
//! So the tests below walk the chain hop by hop rather than checking
//! indices, and assert the chain STOPS where the source stops being
//! linear: `president_considers` has no successor, because from there the
//! prose branches into sign / veto / pocket veto and a successor relation
//! cannot honestly pick one. 0 answer-time model calls.

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
        "adjcli_factsbillstagesucc_{tag}_{}",
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

fn place(dir: &Path, names: &[&str]) {
    for name in names {
        let src = facts_stdlib().join("civics").join(name);
        std::fs::copy(&src, dir.join(name))
            .unwrap_or_else(|e| panic!("copy shipped {name}: {e}"));
    }
}

#[test]
fn bill_stage_successor_walks_the_whole_linear_chain() {
    let dir = scratch("chain");
    place(&dir, &["bill-stage-successor.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"bill-stage-successor.adj\"\n\
         ? bill_stage_successor(introduced, $N)\n\
         ? bill_stage_successor(committee_review, $N)\n\
         ? bill_stage_successor(first_chamber_vote, $N)\n\
         ? bill_stage_successor(second_chamber_process, $N)\n\
         ? bill_stage_successor(reconcile_differences, $N)\n\
         ? bill_stage_successor(vote_on_same_version, $N)\n\
         ? bill_stage_successor(presented_to_president, $N)\n",
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
        out.contains("\"source\":\"Once a bill is introduced, it is assigned to a committee whose members will research, discuss, and make changes to the bill.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Each hop is one transition sentence in the source's own prose. Walking
    // them in order is how a learner actually answers "how does a bill
    // become a law?" -- no index arithmetic anywhere.
    for (from, to) in [
        ("introduced", "committee_review"),
        ("committee_review", "first_chamber_vote"),
        ("first_chamber_vote", "second_chamber_process"),
        ("second_chamber_process", "reconcile_differences"),
        ("reconcile_differences", "vote_on_same_version"),
        ("vote_on_same_version", "presented_to_president"),
        ("presented_to_president", "president_considers"),
    ] {
        assert!(
            out.contains(&format!("bill_stage_successor({from}, {to})")),
            "the chain steps {from} -> {to}: {out}"
        );
    }
}

#[test]
fn bill_stage_successor_carries_every_transition_sentence() {
    let dir = scratch("cites");
    place(&dir, &["bill-stage-successor.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"bill-stage-successor.adj\"\n\
         ? bill_stage_successor(introduced, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Seven rows come from seven different sentences: one `source` plus six
    // `cites`. Each row must stay auditable back to the sentence whose
    // connective states that particular hop.
    for sentence in [
        "Once a bill is introduced, it is assigned to a committee",
        "The bill is then put before that chamber to be voted on.",
        "If the bill passes one body of Congress, it goes to the other body",
        "Once both bodies vote to accept a bill, they must work out any differences",
        "Then both chambers vote on the same version of the bill.",
        "If it passes, they present it to the president.",
        "The president then considers the bill.",
    ] {
        assert!(
            out.contains(sentence),
            "transition sentence carried: {sentence}: {out}"
        );
    }
    assert!(
        out.contains("usa.gov/how-laws-are-made") && out.contains("\"trust\":\"authoritative\""),
        "carries the USA.gov citation: {out}"
    );
}

#[test]
fn bill_stage_successor_runs_backward_to_the_prerequisite_stage() {
    let dir = scratch("reverse");
    place(&dir, &["bill-stage-successor.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"bill-stage-successor.adj\"\n\
         ? bill_stage_successor($P, presented_to_president)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // "What has to happen before it reaches the president?" -- a successor
    // relation answers this directly; an ordinal table would need the reader
    // to decrement an index and look it back up.
    assert!(
        out.contains("bill_stage_successor(vote_on_same_version, presented_to_president)"),
        "both chambers vote on the same version first: {out}"
    );
}

#[test]
fn bill_stage_successor_stops_where_the_source_stops_being_linear() {
    let dir = scratch("branch");
    place(&dir, &["bill-stage-successor.adj"]);
    std::fs::write(
        dir.join("case.adj"),
        "import \"bill-stage-successor.adj\"\n\
         ? bill_stage_successor(president_considers, $N)\n\
         ? bill_stage_successor(citizen_petition, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // `president_considers` has NO successor, deliberately. From there the
    // prose BRANCHES -- sign into law, veto, or pocket veto -- and a
    // successor relation cannot honestly represent a branch without picking
    // one arbitrarily or returning three "next stages" as if all happened.
    // What occurs on each branch is held by checks-and-balances.adj and
    // veto-override.adj instead. `citizen_petition` is an idea ORIGIN the
    // same page lists, not a stage in the journey.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "abstains at the branch point and on the idea origin: {out}"
    );
    // Guard the specific failure this design avoids: no successor may be
    // asserted out of the branch point, under any of the three outcomes.
    for outcome in ["signed_into_law", "vetoed", "pocket_vetoed"] {
        assert!(
            !out.contains(&format!("bill_stage_successor(president_considers, {outcome})")),
            "must not pick one branch outcome as THE successor ({outcome}): {out}"
        );
    }
}
