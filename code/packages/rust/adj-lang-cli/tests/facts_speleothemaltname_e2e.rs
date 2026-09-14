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

const NPS_LOCATOR: &str = "https://www.nps.gov/subjects/caves/speleothems.htm";
const COLUMN_SPAN: &str = "When a stalagmite grows together with its counterpart feeder stalactite, a new speleothem is formed: a column or pillar.";
const CORALLOID_SPAN: &str = "Coralloid (or corallite or cave popcorn) is a catchall term describing knobby, nodular, botryoidal, or corallike speleothems.";
const WATERFALL_SPAN: &str = "The most common of these is the petrified or frozen waterfall, also referred to as cascades, rivers, glaciers, or organ pipes.";
const BACON_SPAN: &str = "Cave Bacon forms on slanted surfaces, and is called \\\"bacon\\\" instead of drapery, when the characteristic layers are present.";

/// Assert one row's warrant, binding the ALT NAME so exactly one row answers.
///
/// Binding the speleothem would return five rows for `frozen_waterfall`, and a
/// whole-stdout `contains` is then satisfied by any sibling's intact copy —
/// the masking defect mutation found in `joint-types` (#15164). Every alt name
/// in this table is unique, so the alt-name direction is single-answer.
fn assert_alt(tag: &str, alt: &str, speleothem: &str, span: &str) -> String {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("earth-science/speleothem-alt-name.adj"),
        dir.join("speleothem-alt-name.adj"),
    )
    .expect("copy shipped speleothem-alt-name.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"speleothem-alt-name.adj\"\n? speleothem_alt_name($S, {alt})\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row has the alt name {alt}: {out}"
    );
    assert!(
        out.contains(&format!("\"S\":\"{speleothem}\"")),
        "{alt} binds {speleothem}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{NPS_LOCATOR}\",\"trust\":\"authoritative\""
        )),
        "{alt} is warranted by the sentence that states IT: {out}"
    );
    out
}

/// #14986. This table held all four justifying sentences already — the COLUMN
/// one as the envelope `source`, the other three as table-level `cites` — so
/// every row carried all four and the seven rows that are not about columns
/// were warranted, in the field that carries the tier, by a sentence about a
/// stalagmite meeting a stalactite.
///
/// The file's own header called that a limitation the reader must work around
/// (#13893). It was not a reading problem: the JSON said the same thing.
#[test]
fn each_row_is_warranted_by_the_sentence_that_states_its_name() {
    let out = assert_alt("altpillar", "pillar", "column", COLUMN_SPAN);
    // The column row keeps its own sentence, and no longer drags the other
    // three along. Twice, not once — `citations` and again under `steps`.
    assert_eq!(
        out.matches("\"corroborations\":[]").count(),
        2,
        "the row's own sentence is its warrant, so nothing corroborates: {out}"
    );
    assert!(
        !out.contains("Coralloid (or corallite"),
        "a column answer no longer carries the coralloid sentence: {out}"
    );

    let out = assert_alt("altcorallite", "corallite", "coralloid", CORALLOID_SPAN);
    assert!(
        !out.contains("When a stalagmite grows together"),
        "a coralloid answer is no longer proved by the column sentence: {out}"
    );

    assert_alt(
        "altbacon",
        "cave_bacon_when_characteristic_layers_present",
        "drapery",
        BACON_SPAN,
    );
}

/// FIVE rows share one sentence — it lists five alternative names for the
/// frozen waterfall. Each carries its own copy, and each is pinned separately,
/// because one broken copy hiding behind four intact ones is exactly the
/// failure `skeleton-bones` (#15171) shipped and `speleothem-substrate`
/// (#15175) designed out.
#[test]
fn all_five_frozen_waterfall_names_carry_their_own_copy() {
    for (tag, alt) in [
        ("wf1", "petrified_waterfall"),
        ("wf2", "cascades"),
        ("wf3", "rivers"),
        ("wf4", "glaciers"),
        ("wf5", "organ_pipes"),
    ] {
        assert_alt(tag, alt, "frozen_waterfall", WATERFALL_SPAN);
    }
}

/// The envelope is the framing sentence, and its WORDING is pinned against the
/// shipped file rather than disclosed as unreachable — the gap #15176 closed.
#[test]
fn the_framing_envelope_never_reaches_an_answer_and_is_pinned() {
    let dir = scratch("altenvelope");
    std::fs::copy(
        facts_stdlib().join("earth-science/speleothem-alt-name.adj"),
        dir.join("speleothem-alt-name.adj"),
    )
    .expect("copy shipped speleothem-alt-name.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"speleothem-alt-name.adj\"\n? speleothem_alt_name($S, $A)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        8,
        "all eight rows answer: {out}"
    );
    assert!(
        !out.contains("Cave Minerals of the World"),
        "the framing span warrants no row: {out}"
    );
    let adj = std::fs::read_to_string(
        facts_stdlib().join("earth-science/speleothem-alt-name.adj"),
    )
    .expect("read shipped speleothem-alt-name.adj");
    assert!(
        adj.contains(
            "    source \"Cave Minerals of the World (Hill, 1997) refers to 38 different types of speleothems and numerous subtypes and varieties.\"\n    locator"
        ),
        "the envelope carries the page's framing sentence, verbatim"
    );
}
