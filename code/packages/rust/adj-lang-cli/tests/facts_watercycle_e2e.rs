//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/water-cycle.adj`) driven through the built
//! CLI: a native `table` of water-cycle stage → step number resolves a
//! binding-query recall with the USGS citation, and abstains on the Sun (which
//! DRIVES the cycle but is not one of its stages) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factswc_{tag}_{}", std::process::id()));
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
fn water_cycle_recall_binds_step_number_with_citation() {
    let dir = scratch("watercycle");
    // Copy the shipped earth-science table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/water-cycle.adj");
    std::fs::copy(&src, dir.join("water-cycle.adj")).expect("copy shipped water-cycle.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"water-cycle.adj\"\n\
         ? water_cycle_stage(evaporation, $N)\n\
         ? water_cycle_stage(precipitation, $N)\n\
         ? water_cycle_stage(sun, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Evaporation is the first step of the cycle; precipitation is the third —
    // the recalled step numbers, straight from the grounded rows.
    assert!(out.contains("\"N\":\"1\""), "evaporation → 1: {out}");
    assert!(out.contains("\"N\":\"3\""), "precipitation → 3: {out}");
    // ONE CONTIGUOUS SPAN, not two loose substrings. `contains("water.usgs.gov")
    // && contains("\"trust\":\"authoritative\"")` was here, and that is the
    // #15139 shape: two halves satisfiable by different parts of the output,
    // which drift apart the moment one of them moves. Pinning source, locator
    // and trust as one run of bytes binds them to each other.
    assert!(
        out.contains(
            "\"source\":\"The water cycle describes how Earth's water is not only always changing forms, between liquid (rain), solid (ice), and gas (vapor), but also moving on, above, and in the Earth.\",\"locator\":\"https://water.usgs.gov/edu/watercycle-kids-beg.html\",\"trust\":\"authoritative\""
        ),
        "carries the USGS citation, whole and contiguous: {out}"
    );
    // The Sun drives the cycle but is not a stage — honest abstention, never a
    // fabricated step number.
    assert!(out.contains("\"abstained\":true"), "sun abstains: {out}");
}

/// Helper: run one query against the shipped table and return stdout.
fn ask(tag: &str, query: &str) -> String {
    let dir = scratch(tag);
    let src = facts_stdlib().join("earth-science/water-cycle.adj");
    std::fs::copy(&src, dir.join("water-cycle.adj")).expect("copy shipped water-cycle.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"water-cycle.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn the_evaporation_sentence_no_longer_warrants_the_other_four_stages() {
    // THIS IS THE CHANGE. The envelope `source` used to be the EVAPORATION
    // sentence, so a query about runoff came back warranted by a sentence
    // about the sun evaporating water -- and four of the five rows were in
    // that position. The envelope is now the page's own framing sentence,
    // which names no stage at all.
    for (tag, stage) in [
        ("neg_cond", "condensation"),
        ("neg_prec", "precipitation"),
        ("neg_run", "runoff"),
        ("neg_ground", "groundwater"),
    ] {
        let out = ask(tag, &format!("water_cycle_stage({stage}, $N)"));
        // THE ABSENCE IS NOT MASKED BY A SIBLING. A query binding several
        // answers would carry other rows' sentences in the same stdout, so
        // `!contains` would measure nothing (#15164). Each of these binds
        // exactly one answer, and that is asserted rather than assumed.
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "exactly one answer for {stage}, so the absence below is about it: {out}"
        );
        assert!(
            !out.contains(
                "The sun causes liquid water to evaporate, or turn from a liquid to a gas (water vapor)."
            ),
            "the evaporation sentence must not reach the {stage} answer: {out}"
        );
    }

    // POSITIVE CONTROL: the evaporation row still carries it, as its own
    // corroboration. Without this, deleting the sentence from the file
    // entirely would leave every assertion above green.
    let out = ask("neg_control", "water_cycle_stage(evaporation, $N)");
    assert!(
        out.contains(
            "The sun causes liquid water to evaporate, or turn from a liquid to a gas (water vapor)."
        ),
        "control: the evaporation row does carry it: {out}"
    );
}

#[test]
fn each_stage_is_corroborated_by_the_sentence_that_names_it() {
    // Pinned PER ROW rather than once, because one pin on one row is exactly
    // what let a single-row truncation survive in the sibling
    // `water-movement-route` suite.
    for (tag, stage, sentence) in [
        (
            "corr_evap",
            "evaporation",
            "The sun causes liquid water to evaporate, or turn from a liquid to a gas (water vapor).",
        ),
        (
            "corr_cond",
            "condensation",
            "This is condensation, the opposite of evaporation.",
        ),
        (
            "corr_prec",
            "precipitation",
            "When they get heavy enough, they fall to Earth as precipitation, such as rain and snow.",
        ),
        (
            "corr_run",
            "runoff",
            "This is called runoff, which provides water to rivers, lakes, and the oceans.",
        ),
        (
            "corr_ground",
            "groundwater",
            "Some precipitation and runoff soaks into the ground to become groundwater.",
        ),
    ] {
        let out = ask(tag, &format!("water_cycle_stage({stage}, $N)"));
        // A CORROBORATION, NOT A SOURCE, and the difference is the point of
        // this table: the sentence says the stage belongs to the cycle, and
        // says nothing about its step NUMBER, which no sentence on the page
        // states. So it is pinned in the `corroborations` array.
        assert!(
            out.contains(&format!(
                "\"corroborations\":[{{\"source\":\"{sentence}\"",
            )),
            "{stage} is corroborated by the sentence that names it: {out}"
        );
    }
}

#[test]
fn the_groundwater_sentence_does_not_leak_to_the_stages_it_mentions() {
    // THE PAIRING TRAP HERE. The groundwater sentence names THREE stages --
    // "Some PRECIPITATION and RUNOFF soaks into the ground to become
    // GROUNDWATER." A "the span mentions the key" rule would hand it to the
    // precipitation or runoff row and pass. It is pinned to groundwater
    // alone.
    for (tag, stage) in [("leak_prec", "precipitation"), ("leak_run", "runoff")] {
        let out = ask(tag, &format!("water_cycle_stage({stage}, $N)"));
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "one answer, so the absence below is about it: {out}"
        );
        assert!(
            !out.contains("Some precipitation and runoff soaks into the ground to become groundwater."),
            "the groundwater sentence names {stage} but must not warrant it: {out}"
        );
    }

    // POSITIVE CONTROL: it does reach the row it belongs to.
    let out = ask("leak_control", "water_cycle_stage(groundwater, $N)");
    assert!(
        out.contains("Some precipitation and runoff soaks into the ground to become groundwater."),
        "control: the groundwater row carries it: {out}"
    );
}

#[test]
fn no_row_carries_a_source_and_that_zero_is_deliberate() {
    // THE ZERO IS THE INSTRUMENT. This table is deliberately NOT converted to
    // per-row `source` (ADJ-TABLES RS-5e, issue #14986): the second column is
    // an INTEGER and no sentence on the cited page assigns a number to a
    // stage, so a per-row `source` would assert that a span warrants a value
    // it does not state. A row `source` appearing here later would be exactly
    // that silent claim, which is why zero is asserted rather than assumed.
    let adj = std::fs::read_to_string(facts_stdlib().join("earth-science/water-cycle.adj"))
        .expect("read shipped water-cycle.adj");
    let body = &adj[adj.find("table water_cycle_stage").expect("table")..];

    assert_eq!(body.matches("\n    row (").count(), 5, "five rows");
    assert_eq!(
        body.matches("\n        cites \"").count(),
        5,
        "each row carries exactly one corroboration"
    );
    assert_eq!(
        body.matches("\n        source \"").count(),
        0,
        "NO row carries a source -- no span on the page states a step number"
    );

    // The envelope, derived totally rather than positionally so a second
    // table-level source cannot hide behind the first.
    let table_sources: Vec<&str> = adj
        .lines()
        .filter(|l| l.starts_with("    source \"") && !l.starts_with("     "))
        .collect();
    assert_eq!(
        table_sources.len(),
        1,
        "exactly one table-level source: {table_sources:?}"
    );
    let envelope = table_sources[0]
        .trim_start()
        .trim_start_matches("source \"")
        .trim_end_matches(0x22 as char);
    assert_eq!(
        envelope,
        "The water cycle describes how Earth's water is not only always changing forms, between liquid (rain), solid (ice), and gas (vapor), but also moving on, above, and in the Earth.",
        "the framing slot carries the page's own sentence, verbatim"
    );

    // THE ENVELOPE NAMES NO STAGE, checked against the keys READ FROM THE
    // TABLE rather than a typed list, and folded on both sides so the guard
    // cannot go vacuous on a capitalised key.
    let folded = envelope.to_lowercase();
    let mut keys = 0;
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("row (") {
            let key = rest
                .split(',')
                .next()
                .expect("row key")
                .replace('_', " ")
                .to_lowercase();
            keys += 1;
            assert!(
                !folded.contains(&key),
                "the framing sentence must name no stage, but names {key:?}"
            );
        }
    }
    assert_eq!(keys, 5, "all five keys were actually checked");
}

#[test]
fn the_step_numbers_are_pinned_to_the_artifact_because_no_citation_can_pin_them() {
    // A MUTANT SURVIVED THIS SUITE UNTIL THIS TEST EXISTED: renumbering
    // `runoff` from 4 to 9 left all five tests green. Every other assertion
    // here is about PROVENANCE -- which sentence warrants which row -- and
    // NO citation can catch a wrong step number, for the reason the `.adj`
    // header gives at length: no sentence on the cited page states a step
    // number at all.
    //
    // So the five pairs are pinned to the SHIPPED ARTIFACT. THIS IS AN
    // ARTIFACT PIN AND NOT A WARRANT, and saying so is the honest part: it
    // means a renumbering has to be done deliberately, in two places, rather
    // than slipping through -- which is the most that can be demanded of a
    // value no source states. It is deliberately NOT dressed up as a
    // citation check.
    let adj = std::fs::read_to_string(facts_stdlib().join("earth-science/water-cycle.adj"))
        .expect("read shipped water-cycle.adj");
    let body = &adj[adj.find("table water_cycle_stage").expect("table")..];

    let mut pairs: Vec<(String, String)> = Vec::new();
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("row (") {
            let inner = rest.split(')').next().expect("row parens");
            let mut it = inner.split(',');
            let stage = it.next().expect("stage").trim().to_string();
            let step = it.next().expect("step").trim().to_string();
            pairs.push((stage, step));
        }
    }
    assert_eq!(
        pairs,
        vec![
            ("evaporation".to_string(), "1".to_string()),
            ("condensation".to_string(), "2".to_string()),
            ("precipitation".to_string(), "3".to_string()),
            ("runoff".to_string(), "4".to_string()),
            ("groundwater".to_string(), "5".to_string()),
        ],
        "the five shipped pairs, in the order the file lists them"
    );

    // AND THE ENGINE AGREES WITH THE FILE. Pinning only the `.adj` would
    // leave a renumbering free to hide in the gap between the artifact and
    // what the CLI actually returns.
    for (stage, step) in &pairs {
        let out = ask(&format!("step_{stage}"), &format!("water_cycle_stage({stage}, $N)"));
        assert!(
            out.contains(&format!("\"N\":\"{step}\"")),
            "the CLI returns {step} for {stage}: {out}"
        );
    }
}
