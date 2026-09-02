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
    // reader could not tell who "They" is. The envelope therefore quotes
    // the two CONTIGUOUS sentences as one string -- still verbatim, and
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
