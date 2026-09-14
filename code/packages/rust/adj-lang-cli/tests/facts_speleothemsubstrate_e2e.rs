//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/speleothem-substrate.adj`) driven
//! through the built CLI: a native `table` recording what a speleothem
//! grows ON, grounding the U.S. National Park Service's "Speleothems"
//! article.
//!
//! EIGHTH cave/karst library. It exists because
//! `speleothem-growth-surface.adj` ABSTAINED on `helictite`, its header
//! recording that the source places helictites on three surfaces with a
//! frequency hedge on the third, and that "one surface would drop the
//! others while three-as-equals would flatten the source's own frequency
//! hedge."
//!
//! That reasoning was right about THAT relation -- single-valued, "grows
//! FROM". It was never an argument that the fact is untableable. This
//! relation is multi-valued, means "grows ON" (the source's own verb), and
//! carries each hedge inside the atom of the value it modifies, the same
//! placement rule `veto-override.adj` and `karst-process-zone.adj` apply.
//!
//! The assertion that matters most is the hedge one: bare `cave_floor` must
//! NOT be a value for either speleothem. If it ever binds, a "less often"
//! has been silently promoted to an unqualified fact.
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
    let dir =
        std::env::temp_dir().join(format!("adjcli_factssubstrate_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/speleothem-substrate.adj");
    std::fs::copy(&src, dir.join("speleothem-substrate.adj"))
        .expect("copy shipped speleothem-substrate.adj");
}

fn case(dir: &Path, query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    std::fs::write(
        &path,
        format!("import \"speleothem-substrate.adj\"\n? {query}\n"),
    )
    .unwrap();
    path
}

#[test]
fn helictites_grow_on_six_named_substrates() {
    let dir = scratch("helictite");
    place(&dir);
    let program = case(&dir, "speleothem_substrate(helictite, $S)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle elsewhere in this
    // file matched only part of the sentence, which let the citation be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once. See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"Helictites grow on cave ceilings, walls, and less often on cave floors. They typically grow on other speleothems, such as carbonate coatings, crusts, and sometimes on soda straws.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    for s in [
        "cave_ceiling",
        "cave_wall",
        "cave_floor_less_often",
        "carbonate_coating",
        "crust",
        "soda_straw_sometimes",
    ] {
        assert!(
            out.contains(&format!("\"S\":\"{s}\"")),
            "helictites grow on {s}: {out}"
        );
    }
    assert!(
        out.contains("nps.gov/subjects/caves/speleothems.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn the_citation_carries_its_own_pronoun_antecedent() {
    let dir = scratch("pronoun");
    place(&dir);
    let program = case(&dir, "speleothem_substrate(helictite, $S)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The substrate sentence begins "They typically grow on other
    // speleothems...". Cited alone its subject is a bare pronoun and a
    // reader could not tell who "They" is. The HELICTITE ROWS' `source`
    // therefore quotes the two CONTIGUOUS sentences as one string -- it was
    // the table envelope until #14986 -- still verbatim, and
    // self-contained. A citation that cannot be read without the page open
    // is not doing its job.
    //
    // ONE NEEDLE SPANNING THE SENTENCE BOUNDARY, NOT TWO NEEDLES. Asserting
    // the two sentences separately would pass just as happily if they had
    // been split into two citations -- `source` holding the first and a
    // `cites` holding the second -- which is exactly the arrangement this
    // test exists to rule out (mutation-verified: the split makes the
    // spanning needle disappear while both halves remain present). The
    // property is that the antecedent travels with the pronoun IN ONE
    // STRING, so the assertion has to straddle the join.
    assert!(
        out.contains(
            "Helictites grow on cave ceilings, walls, and less often on cave floors. \
             They typically grow on other speleothems"
        ),
        "the antecedent travels with the pronoun in a SINGLE citation string: {out}"
    );
}

#[test]
fn the_reverse_lookup_finds_both_speleothems() {
    let dir = scratch("reverse");
    place(&dir);
    let program = case(&dir, "speleothem_substrate($P, cave_wall)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // "What grows on cave walls?" -- a question nothing in this stdlib
    // could answer before. Both members answer it, from two different
    // sentences, which is what makes this a real axis rather than a
    // wrapper around one sentence.
    assert!(
        out.contains("\"P\":\"helictite\"") && out.contains("\"P\":\"frostwork\""),
        "both speleothems grow on cave walls: {out}"
    );
}

#[test]
fn each_speleothem_keeps_its_own_hedge_wording() {
    let dir = scratch("wording");
    place(&dir);
    let program = case(&dir, "speleothem_substrate(frostwork, $S)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // Frostwork's sentence says "less occasionally"; helictite's says "less
    // often". They plainly mean the same thing and are deliberately NOT
    // normalised to one spelling -- each atom carries the word its own
    // sentence used. Smoothing them together would be editing a citation to
    // make a table tidier.
    assert!(
        out.contains("\"S\":\"cave_floor_less_occasionally\""),
        "frostwork keeps its own wording: {out}"
    );
    assert!(
        !out.contains("\"S\":\"cave_floor_less_often\""),
        "frostwork must not borrow helictite's wording: {out}"
    );
    assert!(
        out.contains("less occasionally on floors"),
        "carries the frostwork sentence verbatim: {out}"
    );
}

#[test]
fn the_unhedged_cave_floor_abstains_for_both() {
    let dir = scratch("floor");
    place(&dir);
    // Variable form deliberately: a fully-bound query that matches nothing
    // produces NO recall entry at all rather than an abstention.
    let program = case(&dir, "speleothem_substrate($P, cave_floor)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // THE POINT OF THE HEDGE PLACEMENT. Both sentences qualify the floor,
    // so asking for the unqualified floor is asking where these speleothems
    // grow just as readily as anywhere else -- which neither sentence says.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "the unhedged floor is not a value of this relation: {out}"
    );
    assert!(
        !out.contains("\"P\":\"helictite\"") && !out.contains("\"P\":\"frostwork\""),
        "neither speleothem may be asserted as growing on floors unqualified: {out}"
    );
}

#[test]
fn speleothem_substrate_abstains_where_the_source_names_no_substrate() {
    let dir = scratch("unnamed");
    place(&dir);
    let program = case(&dir, "speleothem_substrate(stalagmite, $S)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The page says plenty about stalagmites, but never what they grow ON.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "no substrate is inferred from general karst knowledge: {out}"
    );

    let dir = scratch("shape");
    place(&dir);
    let program = case(&dir, "speleothem_substrate($P, slanted_surface)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // Cave bacon "forms on slanted surfaces", but that describes the SHAPE
    // of a surface rather than an identifiable thing in a cave, and it
    // would not join with any other value in this column.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a surface shape is not a substrate value: {out}"
    );
}

const SPEL_LOCATOR: &str = "https://www.nps.gov/subjects/caves/speleothems.htm";
const HELICTITE_SPAN: &str = "Helictites grow on cave ceilings, walls, and less often on cave floors. They typically grow on other speleothems, such as carbonate coatings, crusts, and sometimes on soda straws.";
const FROSTWORK_SPAN: &str = "Frostwork can also be found on stalactites, walls, ceilings, ledges, and less occasionally on floors (Hill, 1997).";

/// Assert a row's warrant with a needle that SPANS from the binding into the
/// citation, so the span provably belongs to the row that bound it.
///
/// Two dead ends preceded this shape, and both are worth recording:
///
/// 1. A fully-ground query (`speleothem_substrate(frostwork, ledge)`) emits no
///    `citations` at all — the engine ranks it as a hypothesis rather than
///    recalling it. An assertion counting citation blocks failed with zero.
/// 2. Binding only the speleothem returns six rows for `helictite`, and a
///    whole-stdout `contains` is then satisfied by any sibling's intact copy —
///    the masking defect mutation found in `anatomy/joint-types.adj` (#15164).
///
/// Binding the SUBSTRATE and spanning binding→citation avoids both: the needle
/// cannot match an answer that bound a different speleothem, even when both
/// answers are in the same output.
fn assert_warrant(tag: &str, substrate: &str, speleothem: &str, span: &str) -> String {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("earth-science/speleothem-substrate.adj"),
        dir.join("speleothem-substrate.adj"),
    )
    .expect("copy shipped speleothem-substrate.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"speleothem-substrate.adj\"\n? speleothem_substrate($S, {substrate})\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains(&format!(
            "\"bindings\":{{\"S\":\"{speleothem}\"}},\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{SPEL_LOCATOR}\",\"trust\":\"authoritative\"",
        )),
        "{speleothem}/{substrate}: the span belongs to the row that bound it: {out}"
    );
    out
}

/// #14986. This table held BOTH justifying sentences already — the helictite
/// one as the envelope `source`, the frostwork one as a table-level `cites` —
/// so every row got both, and the five FROSTWORK rows were warranted by a
/// sentence about helictites, with their real evidence demoted to an untiered
/// corroboration.
///
/// That is what the atmosphere review (#15137) rejected in the other
/// direction: the field carrying the TIER must be the sentence that supports
/// the row. No new source and no widening were needed — both sentences already
/// name their own speleothem.
#[test]
fn each_row_is_warranted_by_its_own_speleothems_sentence() {
    // `ledge` is frostwork's alone, so this output has exactly one answer.
    let out = assert_warrant("spelledge", "ledge", "frostwork", FROSTWORK_SPAN);
    assert!(
        !out.contains("Helictites grow on cave ceilings"),
        "a frostwork row is no longer proved by a sentence about helictites: {out}"
    );
    // No corroboration: the warrant IS the supporting sentence now. Twice —
    // provenance is emitted under `citations` and again under `steps`.
    assert_eq!(
        out.matches("\"corroborations\":[]").count(),
        2,
        "the row's own sentence is its warrant, so nothing corroborates: {out}"
    );

    let out = assert_warrant("spelstraw", "soda_straw_sometimes", "helictite", HELICTITE_SPAN);
    assert!(
        !out.contains("Frostwork can also be found"),
        "and the frostwork sentence no longer reaches a helictite answer: {out}"
    );
}

/// All eleven rows, each pinned by a binding→citation needle. `cave_wall` and
/// `cave_ceiling` belong to BOTH speleothems and return two answers; the
/// spanning needle still distinguishes them, which is the property that makes
/// this shape worth using over a count.
#[test]
fn all_eleven_rows_carry_their_own_span() {
    for (tag, sub) in [
        ("h1", "cave_ceiling"),
        ("h2", "cave_wall"),
        ("h3", "cave_floor_less_often"),
        ("h4", "carbonate_coating"),
        ("h5", "crust"),
        ("h6", "soda_straw_sometimes"),
    ] {
        assert_warrant(tag, sub, "helictite", HELICTITE_SPAN);
    }
    for (tag, sub) in [
        ("f1", "stalactite"),
        ("f2", "cave_wall"),
        ("f3", "cave_ceiling"),
        ("f4", "ledge"),
        ("f5", "cave_floor_less_occasionally"),
    ] {
        assert_warrant(tag, sub, "frostwork", FROSTWORK_SPAN);
    }
}

/// The envelope now carries the page's own DEFINITION of a speleothem. It
/// warrants no row; its wording is unreachable from any answer once every row
/// overrides `source`, which is disclosed rather than implied.
///
/// A first version used "In general, however, one thing caves do have in
/// common is where speleothems form." Review read it in context: on the page
/// that sentence introduces the water-table zone, not the substrate, and its
/// leading connective has no antecedent inside the quote.
#[test]
fn the_framing_envelope_never_reaches_an_answer() {
    let dir = scratch("spelenvelope");
    std::fs::copy(
        facts_stdlib().join("earth-science/speleothem-substrate.adj"),
        dir.join("speleothem-substrate.adj"),
    )
    .expect("copy shipped speleothem-substrate.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"speleothem-substrate.adj\"\n? speleothem_substrate($S, $B)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        11,
        "all eleven rows answer: {out}"
    );
    assert!(
        !out.contains("The term speleothem refers to the mode of occurrence"),
        "the framing span warrants no row: {out}"
    );
}
