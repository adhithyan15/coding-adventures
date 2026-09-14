//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/water-movement-route.adj`) driven
//! through the built CLI: a native `table` recording which way through the
//! Earth system a named water cycle process moves water, grounding the
//! U.S. Geological Survey's Water Science School.
//!
//! NOT a duplicate of `water-cycle.adj`. That table ORDERS five processes
//! (evaporation 1, condensation 2, precipitation 3, runoff 4, groundwater
//! 5) and answers "what comes next?". This one answers "WHICH WAY does
//! water move?", covers EIGHT processes, and is not an ordering. They
//! overlap on three atoms -- evaporation, precipitation and runoff -- and
//! disagree about nothing, because they answer different questions about
//! them: runoff is stage 4 there AND moves water across the surface here.
//!
//! The assertion that matters most is the condensation abstention. It is
//! unmistakably a water cycle process and is stage 2 of the sibling table,
//! so a system reasoning from general knowledge would confidently give it a
//! route. None of the three source sentences lists it, so this relation has
//! no value for it. BEING A FAMOUS PROCESS IS NOT EVIDENCE ABOUT THIS
//! RELATION.
//!
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
    let dir = std::env::temp_dir().join(format!("adjcli_factswaterroute_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/water-movement-route.adj");
    std::fs::copy(&src, dir.join("water-movement-route.adj"))
        .expect("copy shipped water-movement-route.adj");
}

fn case(dir: &Path, query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    std::fs::write(
        &path,
        format!("import \"water-movement-route.adj\"\n? {query}\n"),
    )
    .unwrap();
    path
}

#[test]
fn runoff_moves_water_across_the_surface() {
    let dir = scratch("runoff");
    place(&dir);
    let program = case(&dir, "water_movement_route(runoff, $R)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle elsewhere in this
    // file matched only part of the sentence, which let the citation be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once. See issues #13916 and #13918.
    //
    // THE SENTENCE THIS PINS CHANGED, and that change is the point of
    // issue #14986. Provenance used to be table-level: the ATMOSPHERE
    // sentence was the envelope `source` and therefore runoff's primary
    // source, though it does not mention runoff, with the sentence that
    // does mention it demoted to a `cites` corroboration. Each row now
    // carries the sentence that lists it.
    assert!(
        out.contains("\"source\":\"Water moves across the surface through snowmelt, runoff, and streamflow.\""),
        "runoff's own source is the sentence that lists runoff, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"R\":\"across_the_surface\""),
        "runoff moves water across the surface: {out}"
    );
    // THE NEGATIVE ARM, which is the half that was impossible before: the
    // atmosphere sentence must not reach this answer AT ALL, not even as a
    // corroboration. Under the old table-level shape it was this answer's
    // primary source.
    assert!(
        !out.contains("Water moves between the atmosphere and the surface through evaporation, evapotranspiration, and precipitation."),
        "the atmosphere sentence must not reach the runoff answer: {out}"
    );
    // ONE CONTIGUOUS SPAN, not three loose needles. A bare
    // `contains("\"corroborations\":[]")` was here and did not measure what
    // its message claimed: the CLI's UNRESOLVED_PROV constant ends with
    // exactly that text, so ANY unprovenanced block anywhere in stdout
    // satisfied it. Pinning source, locator, trust and the empty
    // corroborations array as one run of bytes binds them to each other.
    assert!(
        out.contains(
            "\"source\":\"Water moves across the surface through snowmelt, runoff, and streamflow.\",\"locator\":\"https://www.usgs.gov/water-science-school/water-cycle\",\"trust\":\"authoritative\",\"corroborations\":[]"
        ),
        "runoff's whole warrant, contiguous and with no corroborations: {out}"
    );
    // THE NEGATIVE ARM IS NOT MASKED BY A SIBLING. A query binding several
    // answers would carry the other rows' sentences in the same stdout, so
    // `!contains` would be measuring nothing (#15164). This query binds
    // exactly one answer, and that is asserted rather than assumed.
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one answer, so the absence above is about THIS answer: {out}"
    );
    // THE LOCATOR BOUND TO THE TIER, not two loose substrings. These two
    // halves drifted apart once already: with `cites` corroborations
    // carrying the same url, reverting ONLY the envelope locator left
    // this assertion green (#15139). A corroboration serializes with
    // `locator` as its last key, so pairing it with `trust` can only
    // match the row's own warrant.
    assert!(
        out.contains("\"locator\":\"https://www.usgs.gov/water-science-school/water-cycle\",\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}

#[test]
fn the_reverse_lookup_answers_which_processes_reach_the_ground() {
    let dir = scratch("reverse");
    place(&dir);
    let program = case(&dir, "water_movement_route($P, into_the_ground)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The question worth having, and one nothing in this stdlib could
    // answer before.
    assert!(
        out.contains("\"P\":\"infiltration\"") && out.contains("\"P\":\"groundwater_recharge\""),
        "both ground-bound processes are returned: {out}"
    );
    // Without this, a mutation mapping EVERY process to into_the_ground
    // would keep the assertion above green. The route has to be selective.
    assert!(
        !out.contains("\"P\":\"runoff\"") && !out.contains("\"P\":\"evaporation\""),
        "processes on other routes must not be returned: {out}"
    );

    let dir = scratch("reverse2");
    place(&dir);
    let program = case(&dir, "water_movement_route($P, between_atmosphere_and_surface)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"P\":\"evaporation\"")
            && out.contains("\"P\":\"evapotranspiration\"")
            && out.contains("\"P\":\"precipitation\""),
        "all three atmosphere-surface processes are returned: {out}"
    );
}

#[test]
fn the_citation_is_byte_faithful_around_its_linked_terms() {
    let dir = scratch("bytes");
    place(&dir);
    let program = case(&dir, "water_movement_route(evaporation, $R)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // EXTRACTION HAZARD, PINNED. On the source page each process name is
    // wrapped in a link -- <a>evaporation</a>, <a>evapotranspiration</a> --
    // so naive tag-stripping yields "evaporation , evapotranspiration",
    // with spaces the page does not contain. The envelope was checked
    // against the raw HTML instead. This asserts the real punctuation
    // survived, and that the artifact form is absent.
    assert!(
        out.contains(
            "through evaporation, evapotranspiration, and precipitation."
        ),
        "the citation carries the page's real punctuation: {out}"
    );
    assert!(
        !out.contains("evaporation , evapotranspiration"),
        "the tag-stripping artifact must not have reached the envelope: {out}"
    );
}

#[test]
fn condensation_abstains_though_it_is_a_famous_water_cycle_process() {
    let dir = scratch("condensation");
    place(&dir);
    // Variable form deliberately: a fully-bound query that matches nothing
    // produces NO recall entry at all rather than an abstention.
    let program = case(&dir, "water_movement_route(condensation, $R)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // THE ASSERTION THAT MATTERS MOST. Condensation is stage 2 in the
    // sibling `water-cycle.adj` and is discussed on this very page, so a
    // model answering from general knowledge would give it a route without
    // hesitating. None of the three route sentences lists it, so this
    // relation has no value for it. Being famous is not evidence.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a process no route sentence lists has no route here: {out}"
    );
    // A `!out.contains("\"R\":\"")` needle was deliberately REMOVED here.
    // `abstained` is emitted as `dag.proofs.is_empty()` and the answer list
    // is built from those same proofs, so that needle is strictly implied by
    // the assertion above -- it can never fail independently. Worse, it
    // would go SILENTLY VACUOUS if the query variable were ever renamed
    // from `$R`, while still reading as a guard. That is the same
    // silent-degradation shape as the two vacuous tests already caught in
    // this series, so it is better absent than decorative.
    //
    // POSITIVE CONTROL, which is what actually makes the abstention mean
    // something: a query that MUST bind, proving the table is loaded and
    // answering rather than uniformly empty. Without this, the abstention
    // above stays green even against a completely gutted table.
    let dir = scratch("condensation_control");
    place(&dir);
    let program = case(&dir, "water_movement_route(precipitation, $R)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"R\":\"between_atmosphere_and_surface\""),
        "control: a listed process still binds, so the abstention above is not vacuous: {out}"
    );
}

#[test]
fn the_compound_evapotranspiration_is_not_split() {
    let dir = scratch("compound");
    place(&dir);
    let program = case(&dir, "water_movement_route(transpiration, $R)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The sentence lists `evapotranspiration`, and that compound is the
    // atom. Splitting it into transpiration and evaporation would assert a
    // decomposition this sentence does not make -- a real temptation, since
    // the compound obviously contains both words.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "the compound is not silently decomposed: {out}"
    );

    let dir = scratch("compound2");
    place(&dir);
    let program = case(&dir, "water_movement_route(evapotranspiration, $R)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"R\":\"between_atmosphere_and_surface\""),
        "the compound itself is a value: {out}"
    );
}

#[test]
fn sublimation_abstains_because_no_route_sentence_places_it() {
    let dir = scratch("sublimation");
    place(&dir);
    let program = case(&dir, "water_movement_route(sublimation, $R)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The page names sublimation once and never places it in a route
    // sentence. Mentioned-on-the-page is not the same as stated-by-the-
    // sentences this relation draws from.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a mentioned-but-unplaced process abstains: {out}"
    );

    // Positive control, for the same reason as above: renaming every atom
    // in the table would leave the assertion above green while the library
    // answered nothing at all.
    let dir = scratch("sublimation_control");
    place(&dir);
    let program = case(&dir, "water_movement_route(streamflow, $R)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"R\":\"across_the_surface\""),
        "control: a listed process still binds: {out}"
    );
}

#[test]
fn the_envelope_framing_sentence_reaches_no_answer() {
    let dir = scratch("framing");
    place(&dir);
    // Every one of the eight rows overrides `source`, so the envelope's
    // sentence is emitted ZERO times. THE ZERO IS THE INSTRUMENT: it is the
    // strongest available form of "the envelope mis-warrants no row", and
    // it is not decorative, because it reddens the moment any row loses its
    // own block and falls back to the envelope -- the exact silent
    // regression this conversion has to stay fixed against.
    //
    // `source` is a REQUIRED envelope field (an envelope carrying only
    // `locator` and `trust` is rejected with `TableMissingProvenance`), so
    // the slot cannot simply be left empty. It is filled with the page's
    // own framing sentence, which names none of the eight processes.
    for query in [
        "water_movement_route($P, $R)",
        "water_movement_route(runoff, $R)",
        "water_movement_route($P, into_the_ground)",
    ] {
        let program = case(&dir, query);
        let (ok, out) = run(&program);
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(
            out.matches("The water cycle describes where water is on Earth")
                .count(),
            0,
            "the framing sentence warrants no answer, for {query}: {out}"
        );
    }

    // POSITIVE CONTROL for the count above, because a zero proves nothing
    // about a program that printed nothing. The whole-table query must
    // return all eight rows, and provenance is emitted twice per answer
    // (once under `citations`, once under `steps`), so the three route
    // sentences together occur exactly sixteen times.
    let program = case(&dir, "water_movement_route($P, $R)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // WHOLE SENTENCES, NOT PREFIXES. A prefix needle was here first, and a
    // mutant that truncated the surface sentence after "snowmelt." SURVIVED
    // the whole suite: the shortened span still starts with the prefix, so
    // the count never moved, and only the `runoff` row had a full anchored
    // pin. Counting the whole sentence makes a truncation anywhere in the
    // table drop the count. Same defect as #13916/#13918, one row over.
    let atmosphere = out
        .matches(
            "Water moves between the atmosphere and the surface through \
             evaporation, evapotranspiration, and precipitation.",
        )
        .count();
    let surface = out
        .matches(
            "Water moves across the surface through snowmelt, runoff, and \
             streamflow.",
        )
        .count();
    let ground = out
        .matches(
            "Water moves into the ground through infiltration and \
             groundwater recharge.",
        )
        .count();
    assert_eq!(
        (atmosphere, surface, ground),
        (6, 6, 4),
        "control: 3 + 3 + 2 rows, each emitting its span twice: {out}"
    );
}

#[test]
fn every_row_carries_its_own_span_and_none_restates_the_locator() {
    // STRUCTURAL, AND IT READS THE SHIPPED FILE. One side of each
    // comparison below is the `.adj` on disk rather than another literal in
    // this test, so the two cannot drift into agreeing with each other.
    let adj = std::fs::read_to_string(
        facts_stdlib().join("earth-science/water-movement-route.adj"),
    )
    .expect("read shipped water-movement-route.adj");
    let body = &adj[adj.find("table water_movement_route").expect("table")..];

    let rows = body.matches("\n    row (").count();
    let row_sources = body.matches("\n        source \"").count();
    assert_eq!(rows, 8, "eight rows");
    assert_eq!(
        row_sources, rows,
        "every row carries its own source block, not the envelope's"
    );

    // EVERY ROW SPAN IS PINNED, not just the one a query happens to bind.
    // The distinct row `source` values must be exactly the three sentences,
    // carried by exactly 3 / 3 / 2 rows. A mutant truncating the SNOWMELT
    // row's span survived the whole suite before this existed, because the
    // only full-sentence pin in the file was on `runoff`.
    // EIGHT SPACES EXACTLY, which is what makes this a ROW span and not the
    // envelope's. A trimmed prefix read nine spans, not eight: it swept up
    // the four-space envelope source as if it were a ninth row.
    let mut spans: Vec<&str> = body
        .lines()
        .filter(|l| !l.starts_with("         "))
        .filter_map(|l| l.strip_prefix("        source \""))
        .map(|r| r.trim_end_matches(0x22 as char))
        .collect();
    assert_eq!(spans.len(), rows, "one span read per row");
    spans.sort_unstable();
    let counted = |needle: &str| spans.iter().filter(|s| **s == needle).count();
    let atmosphere = "Water moves between the atmosphere and the surface \
                      through evaporation, evapotranspiration, and \
                      precipitation.";
    let surface =
        "Water moves across the surface through snowmelt, runoff, and \
         streamflow.";
    let ground = "Water moves into the ground through infiltration and \
                  groundwater recharge.";
    assert_eq!(
        (counted(atmosphere), counted(surface), counted(ground)),
        (3, 3, 2),
        "the three sentences, verbatim, carried by 3 / 3 / 2 rows: {spans:?}"
    );
    spans.dedup();
    assert_eq!(
        spans.len(),
        3,
        "and NOTHING ELSE is a row span -- so an added or reworded sentence \
         cannot slip in beside them: {spans:?}"
    );

    // THE ENVELOPE LOCATOR IS DERIVED TOTALLY, not positionally. Requiring
    // exactly one four-space `locator` line, and every `locator` line to be
    // indented four or eight, means a row locator misindented to four
    // spaces -- which the parser accepts and ships as that answer's locator
    // -- makes the count two and reddens, instead of being adopted here as
    // "the envelope".
    let table_locators: Vec<&str> = adj
        .lines()
        .filter(|l| l.starts_with("    locator \"") && !l.starts_with("     "))
        .collect();
    assert_eq!(
        table_locators.len(),
        1,
        "exactly one table-level locator: {table_locators:?}"
    );
    for l in adj.lines() {
        if l.trim_start().starts_with("locator \"") {
            let indent = l.len() - l.trim_start().len();
            assert!(
                indent == 4 || indent == 8,
                "every locator line is indented 4 (table) or 8 (row): {l:?}"
            );
        }
    }
    assert_eq!(
        body.matches("\n        locator \"").count(),
        0,
        "no row restates the locator -- every sentence is on the one page \
         the envelope names, and `geography/reference-lines.adj` restates a \
         row locator only when its page DIFFERS"
    );

    // THE ENVELOPE NAMES NO ROW KEY. That is the property that makes a
    // framing sentence safe in a required slot, and it is checked against
    // the keys READ FROM THE TABLE rather than a typed list, so adding a
    // ninth row re-checks the envelope automatically.
    let envelope_raw = adj
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("envelope source")
        .trim_start()
        .trim_start_matches("source \"")
        .trim_end_matches(0x22 as char);
    // THE ENVELOPE SENTENCE IS PINNED VERBATIM. Review found it was the one
    // field in this table nothing asserted the VALUE of: the only other
    // needle touching it was a prefix inside an `assert_eq!(..., 0)`, which
    // stays 0 for any string whatsoever, and the row-key scan below only
    // checks what it does NOT say. A fabricated framing sentence naming no
    // process left all eight tests green -- the same defect class this
    // conversion exists to fix, one field over.
    assert_eq!(
        envelope_raw,
        "The water cycle describes where water is on Earth and how it moves.",
        "the framing slot carries the page's own sentence, verbatim"
    );
    let envelope = envelope_raw.to_lowercase();
    let mut keys = 0;
    for line in body.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("row (") {
            let key = rest.split(',').next().expect("row key").replace('_', " ");
            keys += 1;
            assert!(
                !envelope.contains(&key),
                "the framing sentence must name no process, but names {key:?}"
            );
        }
    }
    assert_eq!(keys, 8, "all eight keys were actually checked");
}
