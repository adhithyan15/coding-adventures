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
    assert!(
        out.contains("\"source\":\"Water moves between the atmosphere and the surface through evaporation, evapotranspiration, and precipitation.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"R\":\"across_the_surface\""),
        "runoff moves water across the surface: {out}"
    );
    assert!(
        out.contains("Water moves across the surface through snowmelt, runoff, and streamflow."),
        "the envelope carries this sentence verbatim. NOTE: provenance is TABLE-level, so this          proves the sentence reached stdout, NOT that runoff is attributed to it specifically --          every row carries all three sentences (issue #13898): {out}"
    );
    assert!(
        out.contains("usgs.gov/special-topics/water-science-school/science/water-cycle")
            && out.contains("\"trust\":\"authoritative\""),
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
