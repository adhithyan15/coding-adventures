//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/water-share.adj`) driven through the
//! built CLI: a THREE-column `table` recording what share of a stated body
//! of water sits in each place, AND what that share is a share of,
//! grounding the U.S. Geological Survey's "Where is Earth's Water?".
//!
//! THE DENOMINATOR IS PART OF THE FACT. The page states seven shares
//! against three different bases: all Earth's water (saline 96%, freshwater
//! 2.5%), all freshwater (ice 68%, ground 30%, surface 1.2%), and surface
//! freshwater (lakes 20.9%, rivers 0.49%). A two-column table would put
//! "over 96 percent" and "over 68 percent" in one column as though they
//! were comparable. They are not: 96% is of all water on Earth, 68% is of
//! the 2.5% of it that is fresh. Reading them as commensurable is the
//! commonest error made with these statistics, and a table that invited it
//! would be worse than no table, because it would carry a citation.
//!
//! The assertion that matters most is `the_cross_base_conversion_abstains`:
//! ground water is stated as 30% OF FRESHWATER, so asking for its share of
//! all Earth's water must find nothing. That conversion is arithmetic the
//! source never performs, and performing it here would be reasoning
//! presented as recall.
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
    let dir = std::env::temp_dir().join(format!("adjcli_factswatershare_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/water-share.adj");
    std::fs::copy(&src, dir.join("water-share.adj")).expect("copy shipped water-share.adj");
}

fn case(dir: &Path, query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    std::fs::write(&path, format!("import \"water-share.adj\"\n? {query}\n")).unwrap();
    path
}

#[test]
fn a_share_always_arrives_with_its_denominator() {
    let dir = scratch("withbase");
    place(&dir);
    let program = case(&dir, "water_share(ice_and_glaciers, $S, $B)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle elsewhere in this
    // file matched only part of the sentence, which let the citation be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once. See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"Notice how of the world's total water supply of about 332.5 million cubic miles of water, over 96 percent is saline. And, of the total freshwater, over 68 percent is locked up in ice and glaciers. Another 30 percent of freshwater is in the ground.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // ONE JOINT NEEDLE, NOT TWO INDEPENDENT SCANS. Asserting
    // out.contains("S":"...") and out.contains("B":"...") separately does
    // NOT establish co-occurrence: both are scans of the whole stdout, so
    // they pass even when the share and the base come back in DIFFERENT
    // answers. Mutation-verified -- splitting the ice row into
    // (ice, over_68, surface_freshwater) and (ice, 2_5, all_freshwater)
    // satisfies both separate needles while the data is wrong. The binding
    // object is emitted contiguously in first-appearance variable order,
    // so matching it whole is what actually proves "together".
    assert!(
        out.contains("\"bindings\":{\"S\":\"share_over_68_percent\",\"B\":\"all_freshwater\"}"),
        "the share and its base are returned in ONE binding set: {out}"
    );
    assert!(
        out.contains("of the total freshwater, over 68 percent is locked up in ice and glaciers"),
        "carries the grounding sentence verbatim: {out}"
    );
    assert!(
        out.contains("usgs.gov/special-topics/water-science-school/science/where-earths-water")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}

#[test]
fn the_reverse_lookup_is_scoped_by_base() {
    let dir = scratch("bybase");
    place(&dir);
    let program = case(&dir, "water_share($P, $S, surface_freshwater)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // Joint needles again: these pin place-to-share, not merely that both
    // place names appear somewhere. Without them, swapping the lakes and
    // rivers figures would leave this test green.
    assert!(
        out.contains("\"bindings\":{\"P\":\"lakes\",\"S\":\"share_20_9_percent\"}")
            && out.contains("\"bindings\":{\"P\":\"rivers\",\"S\":\"share_0_49_percent\"}"),
        "both surface-freshwater places return their own share: {out}"
    );
    // The scoping is the point: places measured against a DIFFERENT base
    // must not appear, or the denominators have leaked back together.
    assert!(
        !out.contains("\"P\":\"saline_water\"")
            && !out.contains("\"P\":\"ice_and_glaciers\"")
            && !out.contains("\"P\":\"ground\""),
        "places measured against other bases must not appear: {out}"
    );
}

#[test]
fn the_cross_base_conversion_abstains() {
    let dir = scratch("crossbase");
    place(&dir);
    // Variable form deliberately: a fully-bound query that matches nothing
    // produces NO recall entry at all rather than an abstention.
    let program = case(&dir, "water_share(ground, $S, all_earths_water)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // THE ASSERTION THAT MATTERS MOST. Ground water is stated as 30% OF
    // FRESHWATER. Its share of all Earth's water is computable -- roughly
    // 30% of 2.5% -- but this source never performs that arithmetic, and a
    // recall library that did would be presenting reasoning as citation.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a share against a base the source never used is not a fact here: {out}"
    );

    // POSITIVE CONTROL: the same place, against the base the source DID
    // use, still answers. Without this the abstention above would stay
    // green against a table that answered nothing at all.
    let dir = scratch("crossbase_control");
    place(&dir);
    let program = case(&dir, "water_share(ground, $S, all_freshwater)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"share_30_percent\""),
        "control: ground water's stated share against its stated base still binds: {out}"
    );
}

#[test]
fn a_bare_unhedged_figure_is_not_a_value() {
    let dir = scratch("unhedged");
    place(&dir);
    let program = case(&dir, "water_share($P, share_96_percent, $B)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The source says "over 96 percent", so the hedge rides in the atom and
    // the bare exact figure is not a value -- the same placement rule
    // veto-override.adj and karst-process-zone.adj apply.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "the unhedged figure is not stated by this source: {out}"
    );

    // POSITIVE CONTROL: the hedged form does bind.
    let dir = scratch("unhedged_control");
    place(&dir);
    let program = case(&dir, "water_share($P, share_over_96_percent, $B)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"bindings\":{\"P\":\"saline_water\",\"B\":\"all_earths_water\"}"),
        "control: the hedged form binds with its base, so the abstention above is not vacuous: {out}"
    );
}

#[test]
fn the_pronoun_base_travels_with_its_antecedent() {
    let dir = scratch("pronoun");
    place(&dir);
    let program = case(&dir, "water_share(lakes, $S, $B)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The lakes figure appears as "Most of this water is locked up in ice,
    // and another 20.9% is found in lakes." -- whose base is a PRONOUN.
    // "This water" is surface freshwater only because of the sentence
    // before it. The citation therefore quotes the contiguous run, so the
    // denominator is recoverable from the citation itself.
    //
    // ONE NEEDLE SPANNING THE SENTENCE BOUNDARY, not two separate ones:
    // asserting the halves independently would pass just as happily if the
    // sentences were split into different citations, which is the
    // arrangement this test exists to rule out.
    assert!(
        out.contains(
            "The right bar shows the breakdown of surface freshwater. \
             Most of this water is locked up in ice, and another 20.9% is found in lakes."
        ),
        "the pronoun's antecedent travels with it in a SINGLE citation string: {out}"
    );
    assert!(
        out.contains("\"bindings\":{\"S\":\"share_20_9_percent\",\"B\":\"surface_freshwater\"}"),
        "and the base is bound to THIS share explicitly rather than left to the reader: {out}"
    );
}

#[test]
fn figures_only_in_page_tables_are_not_stated() {
    let dir = scratch("tables");
    place(&dir);
    let program = case(&dir, "water_share(atmosphere, $S, $B)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The page gives atmospheric water, soil moisture and swamps in HTML
    // TABLES rather than in the prose passages this library quotes. Table
    // cells do not survive tag-stripping reliably, so a figure that cannot
    // be cited byte-faithfully is a figure this library does not state.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a figure that cannot be quoted byte-faithfully is not asserted: {out}"
    );

    // POSITIVE CONTROL. This was the one abstention test in the file
    // without one, and review showed it stayed green when the relation was
    // renamed so the library asserted NOTHING AT ALL -- the other
    // abstention tests caught that, this one did not.
    let dir = scratch("tables_control");
    place(&dir);
    let program = case(&dir, "water_share(surface_water, $S, $B)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains(
            "\"bindings\":{\"S\":\"share_a_little_more_than_1_2_percent\",\"B\":\"all_freshwater\"}"
        ),
        "control: a quoted figure still binds, so the abstention above is not vacuous: {out}"
    );
}
