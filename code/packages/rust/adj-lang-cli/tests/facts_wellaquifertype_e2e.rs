//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/well-aquifer-type.adj`) driven through
//! the built CLI: a native `table` recording which kind of aquifer each
//! kind of well is drilled into, grounding the U.S. Geological Survey's
//! Water Science School aquifers page.
//!
//! Third USGS-sourced library. One sentence grounds all three rows, and it
//! is a FIGURE CAPTION rather than body prose -- disclosed in the library
//! header so a reader checking the citation looks under the illustration
//! instead of scanning the article.
//!
//! The reverse query is the one worth having: "which wells reach a confined
//! aquifer?" must return BOTH artesian kinds, because the sentence names
//! "an artesian well AND a flowing artesian well" as distinct things.
//! Collapsing them would assert the source treats them as one well.
//!
//! Every assertion here uses the JOINT binding form
//! (`"bindings":{"W":"...","A":"..."}`) rather than independent substring
//! scans, because two separate `contains` over stdout cannot establish that
//! values arrived in the SAME answer -- the defect class that made four
//! earlier tests in this series pass while proving nothing.
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
    let dir = std::env::temp_dir().join(format!("adjcli_factswellaq_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/well-aquifer-type.adj");
    std::fs::copy(&src, dir.join("well-aquifer-type.adj"))
        .expect("copy shipped well-aquifer-type.adj");
}

fn case(dir: &Path, query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    std::fs::write(
        &path,
        format!("import \"well-aquifer-type.adj\"\n? {query}\n"),
    )
    .unwrap();
    path
}

#[test]
fn an_artesian_well_reaches_a_confined_aquifer() {
    let dir = scratch("artesian");
    place(&dir);
    let program = case(&dir, "well_aquifer_type(artesian_well, $A)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle elsewhere in this
    // file matched only part of the sentence, which let the citation be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once. See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"The illustration shows an artesian well and a flowing artesian well, which are drilled into a confined aquifer, and a water table well, which is drilled into an unconfined aquifer.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"bindings\":{\"A\":\"confined_aquifer\"}"),
        "an artesian well is drilled into a confined aquifer: {out}"
    );
    assert!(
        out.contains(
            "which are drilled into a confined aquifer, and a water table well, \
             which is drilled into an unconfined aquifer."
        ),
        "carries the grounding caption verbatim: {out}"
    );
    assert!(
        out.contains("usgs.gov/special-topics/water-science-school/science/aquifers-and-groundwater")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}

#[test]
fn both_artesian_kinds_reach_the_confined_aquifer() {
    let dir = scratch("reverse");
    place(&dir);
    let program = case(&dir, "well_aquifer_type($W, confined_aquifer)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The sentence names "an artesian well AND a flowing artesian well" as
    // distinct things, so both are rows. Collapsing them into one atom
    // would assert the source treats them as the same well.
    assert!(
        out.contains("\"bindings\":{\"W\":\"artesian_well\"}")
            && out.contains("\"bindings\":{\"W\":\"flowing_artesian_well\"}"),
        "both artesian kinds are returned as distinct wells: {out}"
    );
    // And the unconfined well must not leak in, or the two aquifer kinds
    // have stopped being distinguished.
    assert!(
        !out.contains("\"W\":\"water_table_well\""),
        "the unconfined well must not appear under confined: {out}"
    );
}

#[test]
fn the_water_table_well_is_the_only_unconfined_one() {
    let dir = scratch("unconfined");
    place(&dir);
    let program = case(&dir, "well_aquifer_type($W, unconfined_aquifer)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"bindings\":{\"W\":\"water_table_well\"}"),
        "the water table well reaches the unconfined aquifer: {out}"
    );
    assert!(
        !out.contains("\"W\":\"artesian_well\"") && !out.contains("\"W\":\"flowing_artesian_well\""),
        "artesian wells must not appear under unconfined: {out}"
    );
}

#[test]
fn well_types_the_sentence_does_not_name_abstain() {
    let dir = scratch("unnamed");
    place(&dir);
    // Variable form deliberately: a fully-bound query that matches nothing
    // produces NO recall entry at all rather than an abstention.
    let program = case(&dir, "well_aquifer_type(dug_well, $A)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // Dug, drilled and driven wells are standard types a reader may well
    // ask for. This sentence names three wells and those are not among
    // them, and answering from general hydrology knowledge is exactly what
    // a grounded recall library must not do.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a well type the source does not name has no aquifer here: {out}"
    );

    // POSITIVE CONTROL: a named well still binds, so the abstention above
    // cannot pass against a library that answers nothing at all.
    let dir = scratch("unnamed_control");
    place(&dir);
    let program = case(&dir, "well_aquifer_type(water_table_well, $A)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"bindings\":{\"A\":\"unconfined_aquifer\"}"),
        "control: a named well still binds: {out}"
    );
}

#[test]
fn a_spring_is_not_a_well() {
    let dir = scratch("spring");
    place(&dir);
    let program = case(&dir, "well_aquifer_type(spring, $A)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // A spring is a groundwater feature the page discusses, but it is not a
    // well and this relation is about wells.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a spring is not a value of a well relation: {out}"
    );

    // Positive control, same reason as above.
    let dir = scratch("spring_control");
    place(&dir);
    let program = case(&dir, "well_aquifer_type(artesian_well, $A)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"bindings\":{\"A\":\"confined_aquifer\"}"),
        "control: a named well still binds: {out}"
    );
}
