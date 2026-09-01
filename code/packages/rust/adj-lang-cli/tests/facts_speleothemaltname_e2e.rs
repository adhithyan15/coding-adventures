//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/speleothem-alt-name.adj`) driven
//! through the built CLI: a native `table` recording the other names a cave
//! formation goes by, grounding the U.S. National Park Service's
//! "Speleothems" article.
//!
//! SEVENTH cave/karst library, and the first on the naming axis. Same shape
//! as `astronomy/space-rock-alt-name.adj`, which reads its pairs from the
//! same kind of apposition.
//!
//! The BACKWARD direction is the one that matters here: a reader who meets
//! "organ pipes" on a cave tour needs to get back to `frozen_waterfall`,
//! not the other way round.
//!
//! Two abstentions carry as much content as the rows. `cave_popcorn` is
//! refused because THE SOURCE CONTRADICTS ITSELF -- it is offered as a
//! synonym for coralloid in one sentence and as one of several KINDS of
//! coralloid in another, and picking whichever reading suited the table
//! would be choosing an answer and then finding a citation for it. Bare
//! `cave_bacon` is refused because the source makes that name conditional,
//! so the condition rides inside the atom.
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsaltname_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/speleothem-alt-name.adj");
    std::fs::copy(&src, dir.join("speleothem-alt-name.adj"))
        .expect("copy shipped speleothem-alt-name.adj");
}

fn case(dir: &Path, query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    std::fs::write(
        &path,
        format!("import \"speleothem-alt-name.adj\"\n? {query}\n"),
    )
    .unwrap();
    path
}

#[test]
fn a_frozen_waterfall_answers_to_five_other_names() {
    let dir = scratch("five");
    place(&dir);
    let program = case(&dir, "speleothem_alt_name(frozen_waterfall, $N)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    for name in [
        "petrified_waterfall",
        "cascades",
        "rivers",
        "glaciers",
        "organ_pipes",
    ] {
        assert!(
            out.contains(&format!("\"N\":\"{name}\"")),
            "one sentence licenses all five names, including {name}: {out}"
        );
    }
    assert!(
        out.contains("also referred to as cascades, rivers, glaciers, or organ pipes"),
        "carries the grounding sentence verbatim: {out}"
    );
    assert!(
        out.contains("nps.gov/subjects/caves/speleothems.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn the_reverse_lookup_is_the_useful_direction() {
    let dir = scratch("reverse");
    place(&dir);
    let program = case(&dir, "speleothem_alt_name($S, organ_pipes)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The question a reader actually has: they met the odd name first.
    assert!(
        out.contains("\"S\":\"frozen_waterfall\""),
        "\"organ pipes\" resolves back to the frozen waterfall: {out}"
    );

    let dir = scratch("reverse2");
    place(&dir);
    let program = case(&dir, "speleothem_alt_name($S, pillar)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"column\""),
        "\"pillar\" resolves back to the column: {out}"
    );
}

#[test]
fn the_bacon_condition_rides_inside_the_atom() {
    let dir = scratch("bacon");
    place(&dir);
    let program = case(&dir, "speleothem_alt_name(drapery, $N)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The source makes the name conditional -- bacon "instead of drapery,
    // when the characteristic layers are present" -- so the condition lives
    // in the atom, the same placement rule veto-override.adj and
    // karst-process-zone.adj apply.
    assert!(
        out.contains("\"N\":\"cave_bacon_when_characteristic_layers_present\""),
        "the condition is carried, not dropped: {out}"
    );
    assert!(
        !out.contains("\"N\":\"cave_bacon\""),
        "must not state the name unconditionally: {out}"
    );
}

#[test]
fn the_unconditional_bacon_name_abstains() {
    let dir = scratch("bacon2");
    place(&dir);
    // Variable form deliberately: a fully-bound query that matches nothing
    // produces NO recall entry at all rather than an abstention, so the
    // ground form would silently prove nothing.
    let program = case(&dir, "speleothem_alt_name($S, cave_bacon)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // Asking for the unconditional name is asking what a drapery is ALWAYS
    // called, which this sentence declines to say.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "the unconditional name is not a value of this relation: {out}"
    );
}

#[test]
fn cave_popcorn_abstains_because_the_source_contradicts_itself() {
    let dir = scratch("popcorn");
    place(&dir);
    let program = case(&dir, "speleothem_alt_name($S, cave_popcorn)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // THE MOST IMPORTANT ABSTENTION IN THIS LIBRARY. The page offers cave
    // popcorn as a synonym -- "Coralloid (or corallite or cave popcorn) is
    // a catchall term" -- and then makes it a MEMBER instead: "Coralloids
    // include cave popcorn, grapes, knobstone, coral, cauliflower,
    // globularites, and grapefruit." A thing cannot be both another name
    // for coralloids and one of several kinds of coralloid.
    //
    // `corallite` ships because it appears only in the parenthetical and
    // carries no such conflict. Picking whichever reading suited the table
    // would be choosing an answer and then finding a citation for it, which
    // is the exact failure this stdlib exists to prevent.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a self-contradicting source grounds nothing: {out}"
    );
    assert!(
        !out.contains("\"S\":\"coralloid\""),
        "must never resolve popcorn to coralloid on one of two conflicting readings: {out}"
    );
}

#[test]
fn speleothem_alt_name_abstains_on_single_named_formations_and_on_members() {
    let dir = scratch("single");
    place(&dir);
    let program = case(&dir, "speleothem_alt_name(stalactite, $N)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // This page gives stalactites exactly one name. Inventing synonyms from
    // general karst vocabulary is what a grounded recall library must not do.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "no synonym is invented for a singly-named formation: {out}"
    );

    let dir = scratch("member");
    place(&dir);
    let program = case(&dir, "speleothem_alt_name($S, grapes)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // "grapes" appears on the page only in the list of things coralloids
    // INCLUDE -- a member, not another name. Membership is a different
    // relation and this table does not pretend to hold it.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a member of a category is not an alternative name for it: {out}"
    );
}
