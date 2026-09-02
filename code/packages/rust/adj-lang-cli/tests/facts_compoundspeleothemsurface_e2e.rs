//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/compound-speleothem-surface.adj`)
//! driven through the built CLI: a `rule` that DERIVES which cave surfaces
//! a compound speleothem reaches, by chaining its components with their
//! growth surfaces.
//!
//! THIS IS THE OTHER HALF OF AN ABSTENTION, AND THE POINT OF THE PAIR.
//!
//! `speleothem-growth-surface.adj` refused to give `column` a value, its
//! header stating that recording `cave_ceiling` or `cave_floor` "would be
//! false, and recording both would misrepresent the relation this table
//! holds". That was right: `speleothem_growth_surface` means THE SURFACE A
//! SPELEOTHEM GROWS FROM, and a column does not grow from a surface at all
//! -- it is two speleothems that met.
//!
//! But "does a column touch the ceiling or the floor?" is a good question
//! with a grounded answer. It was never unanswerable; it was merely not a
//! question about growth surfaces. This rule answers it in a relation that
//! MEANS what the answer means. Two answers are correct here, where two
//! rows would have been a misrepresentation there, because the relations
//! are not the same relation.
//!
//! That distinction is the whole argument for abstaining rather than
//! fudging. A table that had quietly bound both surfaces would have been
//! approximately useful and precisely wrong, and nothing downstream could
//! have recovered the difference. Because it declined, the honest answer
//! remained available -- as a composition, in its own name, with its
//! provenance intact.
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
    let dir = std::env::temp_dir().join(format!("adjcli_factscompsurf_{tag}_{}", std::process::id()));
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

/// Place the rule library and both premise libraries it imports.
fn place(dir: &Path) {
    for name in [
        "compound-speleothem-surface.adj",
        "speleothem-component.adj",
        "speleothem-growth-surface.adj",
    ] {
        let src = facts_stdlib().join("earth-science").join(name);
        std::fs::copy(&src, dir.join(name)).unwrap_or_else(|e| panic!("copy shipped {name}: {e}"));
    }
}

fn case(dir: &Path, query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    std::fs::write(
        &path,
        format!("import \"compound-speleothem-surface.adj\"\n? {query}\n"),
    )
    .unwrap();
    path
}

#[test]
fn a_column_reaches_both_the_ceiling_and_the_floor() {
    let dir = scratch("both");
    place(&dir);
    let program = case(&dir, "compound_speleothem_surface(column, $S)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The answer the growth-surface table correctly declined to give,
    // recovered by composition in a relation that means what it means.
    assert!(
        out.contains("\"S\":\"cave_ceiling\"") && out.contains("\"S\":\"cave_floor\""),
        "a column reaches both surfaces through its two components: {out}"
    );
}

#[test]
fn both_premises_citations_survive_the_composition() {
    let dir = scratch("prov");
    place(&dir);
    let program = case(&dir, "compound_speleothem_surface(column, $S)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The conclusion's provenance is the composition of its premises'. Both
    // premise tables must be represented, or a derived answer would be
    // citing less than it actually relied on.
    assert!(
        out.contains("When a stalagmite grows together with its counterpart feeder stalactite"),
        "the component premise is cited: {out}"
    );
    assert!(
        out.contains("Stalactites are the most common and most familiar of all speleothems"),
        "the growth-surface premise is cited: {out}"
    );

    // *** A LIMITATION, PINNED DELIBERATELY SO IT IS NOT MISREAD AS A
    // FEATURE. *** Provenance in this stdlib is attached at TABLE level: a
    // table's `source` envelope and its `cites` corroborations cover every
    // row, not the row that happened to match. So the two answers here are
    // provenanced IDENTICALLY -- the cave_floor answer carries the same
    // citation list as the cave_ceiling one, with the stalactite sentence
    // as its primary citation and the stalagmite sentence only as a
    // corroboration.
    //
    // An earlier draft of this test asserted the opposite: that the ceiling
    // answer traced to the stalactite sentence and the floor answer to the
    // stalagmite one. That claim was FALSE, and worse, the assertions
    // "passed" -- both sentences appear in the output of any single
    // successful use of the growth-surface table, so the test proved
    // nothing while reading as though it proved per-answer attribution.
    //
    // The sharpest way to pin it: ask a question with exactly ONE answer,
    // about the FLOOR. Under row-level provenance that answer would carry
    // only the stalagmite sentence. Under table-level provenance it also
    // carries the stalactite sentence, which is about the ceiling and
    // played no part in this particular answer. Asserting that keeps the
    // limitation visible and will fail loudly if row-level provenance ever
    // lands -- at which point this test and the headers describing
    // table-level provenance should be revisited together.
    let dir = scratch("prov_floor");
    place(&dir);
    let program = case(&dir, "compound_speleothem_surface($C, cave_floor)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"C\":\"column\"") && !out.contains("cave_ceiling"),
        "this query has exactly one answer, about the floor: {out}"
    );
    assert!(
        out.contains("Stalactites are the most common and most familiar of all speleothems"),
        "provenance is table-level: a floor-only answer still carries the ceiling row's \
         sentence. If this ever fails, row-level provenance has landed and the claims in \
         compound-speleothem-surface.adj's header need revisiting: {out}"
    );
}

#[test]
fn the_derived_answer_shows_its_work() {
    let dir = scratch("proof");
    place(&dir);
    let program = case(&dir, "compound_speleothem_surface(column, $S)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"kind\":\"rule\""),
        "the answer is derived, not stored: {out}"
    );
    assert!(
        out.contains("speleothem_component(Compound, Component)")
            && out.contains("speleothem_growth_surface(Component, Surface)"),
        "both premise goals appear in the proof: {out}"
    );
}

#[test]
fn the_rule_runs_backward_from_the_surface() {
    let dir = scratch("reverse");
    place(&dir);
    let program = case(&dir, "compound_speleothem_surface($C, cave_floor)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"C\":\"column\""),
        "the compound speleothem reaching the floor is the column: {out}"
    );
}

#[test]
fn composition_propagates_abstention_from_its_premises() {
    let dir = scratch("helictite");
    place(&dir);
    let program = case(&dir, "compound_speleothem_surface(helictite, $S)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // Absent from BOTH premises. speleothem-growth-surface.adj declined
    // helictite because the source gives three surfaces with a frequency
    // hedge ("cave ceilings, walls, and less often on cave floors") that no
    // two-column row can carry. A derived relation cannot repair a premise
    // it inherits -- composition propagates abstention exactly as it
    // propagates provenance.
    assert!(
        out.contains("\"abstained\":true")
            && out.contains("\"reason\":\"no_grounded_support\""),
        "a derived relation cannot be better grounded than its premises: {out}"
    );

    let dir = scratch("noncompound");
    place(&dir);
    let program = case(&dir, "compound_speleothem_surface(stalactite, $S)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // A stalactite HAS a growth surface, so the second premise would
    // succeed; it is the first that fails, because a stalactite is not a
    // compound. The rule must not degenerate into an alias for
    // speleothem_growth_surface.
    assert!(
        out.contains("\"abstained\":true")
            && out.contains("\"reason\":\"no_grounded_support\""),
        "a non-compound speleothem has nothing to decompose: {out}"
    );
}
