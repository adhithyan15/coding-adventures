//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/speleothem-component.adj`) driven
//! through the built CLI: a native `table` recording which speleothems JOIN
//! to form a compound one, grounding the U.S. National Park Service's
//! "Speleothems" article.
//!
//! This library exists to CLOSE a recorded abstention.
//! `speleothem-growth-surface.adj` refused to give `column` a growth
//! surface, saying why in the source's own words -- "Columns are not
//! stalactites nor are they stalagmites; they are both, together" -- since
//! a column is produced by two speleothems joining rather than by growing
//! from one surface. That abstention was correct and was never meant to be
//! permanent: "what is a column made of?" has a perfectly good grounded
//! answer, it simply is not a question about growth surfaces. It needed a
//! DIFFERENT RELATION, which is this one.
//!
//! Same move `civics/congress-chamber.adj` made for
//! `government-branch-member.adj`'s abstention on the two chambers. When a
//! table declines, the fix is a new relation shaped to the question, never
//! a loosened row in the old one.
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsspelcomp_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/speleothem-component.adj");
    std::fs::copy(&src, dir.join("speleothem-component.adj"))
        .expect("copy shipped speleothem-component.adj");
}

fn case(dir: &Path, query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    std::fs::write(
        &path,
        format!("import \"speleothem-component.adj\"\n? {query}\n"),
    )
    .unwrap();
    path
}

#[test]
fn a_column_is_made_of_both_a_stalactite_and_a_stalagmite() {
    let dir = scratch("both");
    place(&dir);
    let program = case(&dir, "speleothem_component(column, $C)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle in this file
    // matched only part of the sentence, so the citation could be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once.
    //
    // Several tests load this library, because siblings import it as a
    // dependency. The pin belongs in its OWN test: the others are not
    // responsible for its provenance. That is also why the assertion has
    // to be unique -- where a co-loaded sibling carries a byte-identical
    // citation, an assertion either one satisfies pins neither.
    // See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"When a stalagmite grows together with its counterpart feeder stalactite, a new speleothem is formed: a column or pillar.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // BOTH, and neither outranks the other -- the source says a column is
    // "both, together". One answer here would be the misrepresentation the
    // growth-surface table was avoiding.
    assert!(
        out.contains("\"C\":\"stalactite\"") && out.contains("\"C\":\"stalagmite\""),
        "a column is constituted by the pair: {out}"
    );
    assert!(
        out.contains("When a stalagmite grows together with its counterpart feeder stalactite"),
        "carries the formation sentence verbatim: {out}"
    );
    assert!(
        out.contains("nps.gov/subjects/caves/speleothems.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn the_identity_sentence_rides_as_a_corroboration() {
    let dir = scratch("corrob");
    place(&dir);
    let program = case(&dir, "speleothem_component(column, $C)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The second sentence is doing real work, not decoration. On its own the
    // formation sentence could be read as a column being a stalagmite that
    // reached the ceiling -- ONE component. "They are both, together" is what
    // makes two rows correct, so it must actually reach the reader.
    assert!(
        out.contains("Columns are not stalactites nor are they stalagmites"),
        "the identity sentence is carried, not just cited in a comment: {out}"
    );
}

#[test]
fn speleothem_component_runs_backward_from_the_component() {
    let dir = scratch("reverse");
    place(&dir);
    let program = case(&dir, "speleothem_component($Compound, stalagmite)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Compound\":\"column\""),
        "a stalagmite is part of a column: {out}"
    );
}

#[test]
fn speleothem_component_abstains_on_a_synonym_and_on_non_compounds() {
    let dir = scratch("abstain");
    place(&dir);
    let program = case(&dir, "speleothem_component(pillar, $C)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The source offers `pillar` as an alternative NAME for the same
    // speleothem ("a column or pillar"), not as a different thing with
    // components of its own. Tabling it would double-count one speleothem as
    // two. A synonym belongs in an alt-name relation.
    assert!(
        out.contains("\"abstained\":true")
            && out.contains("\"reason\":\"no_grounded_support\""),
        "a synonym is not a second compound speleothem: {out}"
    );

    let dir = scratch("abstain2");
    place(&dir);
    let program = case(&dir, "speleothem_component(stalactite, $C)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // A stalactite is a COMPONENT, not a compound. The relation is not
    // reflexive and does not decompose its own values.
    assert!(
        out.contains("\"abstained\":true")
            && out.contains("\"reason\":\"no_grounded_support\""),
        "a component is not itself a compound: {out}"
    );
}

#[test]
fn speleothem_component_abstains_on_a_developmental_stage() {
    let dir = scratch("stage");
    place(&dir);
    let program = case(&dir, "speleothem_component(soda_straw, $C)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The source says every stalactite begins its growth as a hollow soda
    // straw -- a developmental stage of ONE speleothem, not two joining. It
    // is also a stage whose SUCCESSOR this page never describes: it never
    // states how a soda straw becomes a thicker stalactite. Any stage table
    // would have to invent that step, so none was built and this relation
    // declines rather than stretching to cover it.
    assert!(
        out.contains("\"abstained\":true")
            && out.contains("\"reason\":\"no_grounded_support\""),
        "a developmental stage is not a component relation: {out}"
    );
}
