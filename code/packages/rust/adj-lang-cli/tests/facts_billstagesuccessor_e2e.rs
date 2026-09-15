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
//!
//! EACH ROW CARRIES ITS OWN HOP (RS-5e, #14986). The envelope used to be the
//! first transition sentence and every answer carried all seven sentences, so
//! the answer to "what follows introduction?" cited "The president then
//! considers the bill." -- and a test below asserted exactly that. Each hop now
//! carries only the sentence(s) naming its two stages; where a sentence points
//! back ("then", "that chamber", "If it passes"), the row carries the sentence
//! it points to, as a second sentence of its span or as a corroboration.

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
fn the_introduced_hop_carries_its_own_sentence_and_no_other_hops() {
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
    // This test used to assert the OPPOSITE: that the answer to "what follows
    // introduction?" carried all seven transition sentences, which was the
    // #14986 defect (every answer cited every hop). The introduced hop now
    // carries exactly its own sentence, whole, and no other hop's.
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(out.contains(&only_citation(S1, None)), "the introduced hop carries only its own sentence: {out}");
    for other in [S2, S3, S4, S5, S6, S7] {
        assert!(!out.contains(other), "another hop's sentence must not reach the introduced answer: {other}: {out}");
    }
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

const LOCATOR: &str = "https://www.usa.gov/how-laws-are-made";
const ENVELOPE: &str = "Congress is the lawmaking branch of the federal government.";
const S1: &str = "Once a bill is introduced, it is assigned to a committee whose members will research, discuss, and make changes to the bill.";
const S2: &str = "The bill is then put before that chamber to be voted on.";
const S3: &str = "If the bill passes one body of Congress, it goes to the other body to go through a similar process of research, discussion, changes, and voting.";
const S4: &str = "Once both bodies vote to accept a bill, they must work out any differences between the two versions.";
const S5: &str = "Then both chambers vote on the same version of the bill.";
const S6: &str = "If it passes, they present it to the president.";
const S7: &str = "The president then considers the bill.";

/// (stage, next stage, that row's source span, the span it cites if any)
fn hops() -> Vec<(&'static str, &'static str, String, Option<&'static str>)> {
    vec![
        ("introduced", "committee_review", S1.to_string(), None),
        ("committee_review", "first_chamber_vote", S2.to_string(), Some(S1)),
        ("first_chamber_vote", "second_chamber_process", S3.to_string(), None),
        ("second_chamber_process", "reconcile_differences", S4.to_string(), None),
        ("reconcile_differences", "vote_on_same_version", format!("{S4} {S5}"), None),
        ("vote_on_same_version", "presented_to_president", format!("{S5} {S6}"), None),
        ("presented_to_president", "president_considers", S7.to_string(), Some(S6)),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(span: &str, cited: Option<&str>) -> String {
    let corr = match cited {
        Some(c) => format!("{{\"source\":\"{c}\",\"locator\":\"{LOCATOR}\"}}"),
        None => String::new(),
    };
    format!("\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[{corr}]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("civics/bill-stage-successor.adj"))
        .expect("read shipped bill-stage-successor.adj");
    adj[adj.find("table bill_stage_successor").expect("table")..].to_string()
}

#[test]
fn every_hop_answer_carries_only_the_sentences_that_name_that_hop() {
    for (stage, next, span, cited) in hops() {
        let dir = scratch(&format!("hop_{stage}"));
        place(&dir, &["bill-stage-successor.adj"]);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"bill-stage-successor.adj\"\n? bill_stage_successor({stage}, $N)\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {stage}: {out}");
        assert!(out.contains(&format!("\"N\":\"{next}\"")), "{stage} -> {next}: {out}");
        assert!(out.contains(&only_citation(&span, cited)), "{stage}: its own span, whole, and the only citation: {out}");
        // No sentence outside this hop's span and corroboration reaches it.
        for other in [S1, S2, S3, S4, S5, S6, S7] {
            if !span.contains(other) && cited != Some(other) {
                assert!(!out.contains(other), "a sentence from another hop must not reach {stage}: {other}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {stage}: {out}");
    }
}

#[test]
fn the_backward_pointing_hops_carry_the_sentence_they_point_back_to() {
    // "The bill is then put before that chamber" names no committee, and "The
    // president then considers the bill" names no presentation; each row cites
    // the sentence that ends the previous list item. "Then both chambers vote"
    // and "If it passes" need the sentence just before them in the same item,
    // so those rows carry the two sentences together.
    let body = shipped_table();
    assert!(body.contains(&format!(
        "    row (committee_review, first_chamber_vote) {{\n        source \"{S2}\"\n        cites \"{S1}\" locator \"{LOCATOR}\"\n    }}"
    )));
    assert!(body.contains(&format!(
        "    row (presented_to_president, president_considers) {{\n        source \"{S7}\"\n        cites \"{S6}\" locator \"{LOCATOR}\"\n    }}"
    )));
    assert!(body.contains(&format!("        source \"{S4} {S5}\"\n")));
    assert!(body.contains(&format!("        source \"{S5} {S6}\"\n")));
    assert!(!body.contains(&format!("        source \"{S5}\"\n")), "no row is warranted by 'Then both chambers vote' alone");
    assert!(!body.contains(&format!("        source \"{S6}\"\n")), "no row is warranted by 'If it passes' alone");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    assert_eq!(body.matches("\n        source \"").count(), 7, "seven row sources");
    assert_eq!(body.matches("\n        cites \"").count(), 2, "two row corroborations");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{S1}\"\n    locator")), "not the first transition sentence as the envelope again");
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    for word in ["bill", "committee", "introduc", "chamber", "vote", "president", "version"] {
        assert!(!shipped_envelope.contains(word), "the shipped envelope must name no stage, but contains {word:?}");
    }
}
