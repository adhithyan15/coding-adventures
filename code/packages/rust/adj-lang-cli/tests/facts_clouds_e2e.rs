//! End-to-end test for the earth-science CLOUD-TYPES facts library
//! (`adj-facts-stdlib/earth-science/cloud-types.adj`) driven through the built
//! CLI: a native `table` of cloud → altitude level resolves a binding-query
//! recall carrying the NOAA / National Weather Service citation, and abstains on
//! a word the page never assigns a level — 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN DECK SENTENCE (RS-5e, #14986). The envelope used to
//! be the HIGH-deck sentence and the mid/low deck sentences were table-level
//! `cites`, so every answer carried all three and four of the seven clouds --
//! altostratus, altocumulus, stratus, cumulus -- had a PRIMARY source naming a
//! different cloud. The tests below assert the opposite of what they once did:
//! each answer carries the one deck sentence that names its own cloud, alone.

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

const LOCATOR: &str = "https://www.weather.gov/lmk/cloud_classification";
const ENVELOPE: &str =
    "Clouds are classified according to their height above and appearance (texture) from the ground.";
const HIGH: &str = "The three main types of high clouds are cirrus, cirrostratus, and cirrocumulus.";
// The page's own singular slip -- "The two main type of mid-level clouds" -- is
// reproduced, not corrected; correcting it would be the defect.
const MID: &str = "The two main type of mid-level clouds are altostratus and altocumulus.";
const LOW: &str = "The two main types of low clouds include stratus, which develop horizontally, and cumulus, which develop vertically.";

/// (cloud, its level, the one deck sentence that names that cloud)
fn decks() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("cirrus", "high", HIGH),
        ("cirrostratus", "high", HIGH),
        ("cirrocumulus", "high", HIGH),
        ("altostratus", "middle", MID),
        ("altocumulus", "middle", MID),
        ("stratus", "low", LOW),
        ("cumulus", "low", LOW),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run.
/// It CLOSES on the corroborations `]`, so a fabricated `cites` cannot be
/// appended without reddening the assertion (#14735).
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("earth-science/cloud-types.adj"))
        .expect("read shipped cloud-types.adj");
    adj[adj.find("table cloud_altitude").expect("table")..].to_string()
}

fn place(dir: &Path) {
    let src = facts_stdlib().join("earth-science/cloud-types.adj");
    std::fs::copy(&src, dir.join("cloud-types.adj")).expect("copy shipped cloud-types.adj");
}

#[test]
fn earth_science_cloud_altitude_recall_binds_level_with_citation() {
    let dir = scratch("clouds");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-types.adj\"\n\
         ? cloud_altitude(cirrus, $Level)\n\
         ? cloud_altitude(cumulus, $Level)\n\
         ? cloud_altitude(fog, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Cirrus is a high cloud; cumulus is a low cloud — the recalled levels.
    assert!(out.contains("\"Level\":\"high\""), "cirrus → high: {out}");
    assert!(out.contains("\"Level\":\"low\""), "cumulus → low: {out}");
    // Each answer's citation is pinned as TEXT, not by locator and trust alone.
    //
    // This assertion was once `contains(locator) && contains(trust)`, which says
    // nothing about what the `source` field CONTAINS -- it was satisfied by a
    // welded three-sentence string that occurs nowhere on the page. It was then
    // strengthened to the whole citation object, which caught that but still
    // pinned ONE object shared by all seven rows, two thirds of whose text named
    // clouds other than the one asked about. Now each answer is pinned to its
    // OWN deck sentence, and the needle closes on the corroborations `]`, so a
    // fabricated `cites` cannot be appended without reddening this (#14735).
    assert_eq!(out.matches("\"citations\":[").count(), 2, "two answers bind: {out}");
    assert!(
        out.contains(&only_citation(HIGH)),
        "cirrus carries the high-deck sentence, whole, and nothing else: {out}"
    );
    assert!(
        out.contains(&only_citation(LOW)),
        "cumulus carries the low-deck sentence, whole, and nothing else: {out}"
    );
    // Neither answer is a mid-level cloud, so the mid-level sentence -- which
    // used to ride along on every answer as a corroboration -- must not appear.
    assert!(!out.contains(MID), "the mid-level sentence reaches neither answer: {out}");
    assert!(!out.contains(ENVELOPE), "the envelope is primary for no answer: {out}");
    // Fog is never assigned a level on the page — honest abstention, never a
    // fabricated level.
    assert!(out.contains("\"abstained\":true"), "fog abstains: {out}");
}

#[test]
fn earth_science_cloud_altitude_recall_binds_newly_added_rows() {
    let dir = scratch("clouds_new");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-types.adj\"\n\
         ? cloud_altitude(cirrocumulus, $Level)\n\
         ? cloud_altitude(altocumulus, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The two rows THIS TEST BINDS -- both among the three added in #11015 --
    // are named by DIFFERENT SENTENCES, and now carry different citations.
    //
    // This comment has been wrong twice before. It first said both rows came
    // from the same already-quoted source sentence, true only while one `source`
    // field held three welded sentences. The correction said they were grounded
    // by different citations, which was not what the output showed either: the
    // table emitted ONE citation object, in which `cirrocumulus` was named by
    // the `source` and `altocumulus` only by a corroboration -- so a mid-level
    // answer's primary source was a sentence that did not name it. Per-row
    // provenance is what finally makes "different citations" true, so the claim
    // is asserted here rather than described.
    assert!(out.contains("\"Level\":\"high\""), "cirrocumulus → high: {out}");
    assert!(out.contains("\"Level\":\"middle\""), "altocumulus → middle: {out}");
    assert_eq!(out.matches("\"citations\":[").count(), 2, "two answers bind: {out}");
    assert!(out.contains(&only_citation(HIGH)), "cirrocumulus carries the high-deck sentence: {out}");
    assert!(out.contains(&only_citation(MID)), "altocumulus carries the mid-level sentence: {out}");
    assert!(!out.contains(LOW), "the low-deck sentence reaches neither answer: {out}");
}

#[test]
fn every_cloud_answer_carries_only_the_deck_sentence_that_names_it() {
    // Before RS-5e every answer carried all three deck sentences, and for four
    // of the seven clouds the PRIMARY source named a different cloud. Each row
    // now carries the one deck sentence that names it, and nothing else.
    for (cloud, level, span) in decks() {
        let dir = scratch(&format!("deck_{cloud}"));
        place(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"cloud-types.adj\"\n? cloud_altitude({cloud}, $Level)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {cloud}: {out}");
        assert!(out.contains(&format!("\"Level\":\"{level}\"")), "{cloud} -> {level}: {out}");
        assert!(
            out.contains(&only_citation(span)),
            "{cloud}: its own deck sentence, whole, and the only citation: {out}"
        );
        assert!(
            span.contains(cloud),
            "the span carried by {cloud} names {cloud} -- the defect was that it did not"
        );
        for other in [HIGH, MID, LOW] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another deck's sentence must not reach {cloud}: {out}"
                );
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {cloud}: {out}");
    }
}

#[test]
fn the_mid_and_low_decks_are_no_longer_corroborations_of_the_high_deck() {
    let body = shipped_table();
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert_eq!(
        body.matches(&format!("        source \"{HIGH}\"")).count(),
        3,
        "three high rows"
    );
    assert_eq!(
        body.matches(&format!("        source \"{MID}\"")).count(),
        2,
        "two middle rows"
    );
    assert_eq!(
        body.matches(&format!("        source \"{LOW}\"")).count(),
        2,
        "two low rows"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    assert_eq!(body.matches("\n        source \"").count(), 7, "seven row sources");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n"
        )),
        "the envelope is the page's classification sentence"
    );
    assert!(
        !body.contains(&format!("\n    source \"{HIGH}\"\n    locator")),
        "not the high-deck sentence as the envelope again"
    );
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    for word in [
        "cirrus",
        "cirrostratus",
        "cirrocumulus",
        "altostratus",
        "altocumulus",
        "stratus",
        "cumulus",
        "high",
        "middle",
        "low",
    ] {
        assert!(
            !shipped_envelope.contains(word),
            "the shipped envelope must name no cloud and no level, but contains {word:?}"
        );
    }
}
