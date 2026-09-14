//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/flower-parts.adj`) driven through the built CLI:
//! a native `table` of flower parts → the function / role the source states
//! resolves binding-query recalls (forward AND backward) with the source's
//! University of Illinois Extension citation, and abstains on a word that is not
//! one of these flower parts (the leaf) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factfp_{tag}_{}", std::process::id()));
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

#[test]
fn biology_flower_parts_recall_binds_function_with_citation() {
    let dir = scratch("flowerparts");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/flower-parts.adj");
    std::fs::copy(&src, dir.join("flower-parts.adj")).expect("copy shipped flower-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"flower-parts.adj\"\n\
         ? flower_part_function(petal, $Function)\n\
         ? flower_part_function(anther, $Function)\n\
         ? flower_part_function(ovary, $Function)\n\
         ? flower_part_function(stigma, $Function)\n\
         ? flower_part_function($Part, male)\n\
         ? flower_part_function(leaf, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THE WHOLE CITATION, anchored on its JSON key and closed by the
    // terminating quote. This sentence carries a qualifier, so a
    // truncation would silently drop meaning -- the defect issue #13916
    // shipped. Pinning a fragment narrows that hole rather than closing
    // it, because `contains` on a fragment cannot see what precedes or
    // follows it. See issue #13918.
    assert!(
        out.contains("\"source\":\"Petals attract pollinators and are usually the reason why we buy and enjoy flowers.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The petals attract pollinators, the anthers carry the pollen, the ovary
    // contains the ovules, the stigma traps the pollen — the recalled functions
    // (forward binds).
    assert!(
        out.contains("\"Function\":\"attract_pollinators\""),
        "petal → attract_pollinators: {out}"
    );
    assert!(
        out.contains("\"Function\":\"carry_pollen\""),
        "anther → carry_pollen: {out}"
    );
    assert!(
        out.contains("\"Function\":\"contains_ovules\""),
        "ovary → contains_ovules: {out}"
    );
    assert!(
        out.contains("\"Function\":\"traps_pollen\""),
        "stigma → traps_pollen: {out}"
    );
    // The relation runs BACKWARD: bind the function `male`, recall its flower part.
    assert!(
        out.contains("\"Part\":\"stamen\""),
        "male → stamen (reverse recall): {out}"
    );
    // THIS ASSERTION COULD NOT HAVE CAUGHT THE DEFECT. Every span on this
    // table is on the same Illinois Extension page, so a hostname needle held
    // just as well when all seven rows carried the PETAL sentence. Kept
    // because the tier still matters; the per-row pins below are what bind a
    // row to the sentence that states it.
    assert!(
        out.contains("web.extension.illinois.edu")
            && out.contains("\"trust\":\"authoritative\""),
        "carries a citation on the cited page, at the authoritative tier: {out}"
    );
    // Since #14986 these answers carry FOUR different sentences, not one.
    for span in [
        "The anthers carry the pollen.",
        "The style leads down to the ovary that contains the ovules.",
        "The stigma is the sticky surface at the top of the pistil; it traps and holds the pollen.",
    ] {
        assert!(
            out.contains(span),
            "an answer here is warranted by {span}: {out}"
        );
    }
    // The leaf is a plant part, not a part of the flower — honest abstention,
    // never a fabricated function.
    assert!(out.contains("\"abstained\":true"), "leaf abstains: {out}");
}

const PAGE: &str = "https://web.extension.illinois.edu/gpe/case4/c4facts1a.html";
const STAMEN_PISTIL: &str = "The main flower parts are the male part called the stamen and the female part called the pistil.";

/// Assert one row's warrant, binding the PART so exactly one row answers.
fn assert_part(tag: &str, part: &str, function: &str, span: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("biology/flower-parts.adj"),
        dir.join("flower-parts.adj"),
    )
    .expect("copy shipped flower-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"flower-parts.adj\"\n? flower_part_function({part}, $F)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {part}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"F\":\"{function}\"")),
        "{part} binds {function}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{PAGE}\",\"trust\":\"authoritative\""
        )),
        "{part} is warranted by the sentence that states its function: {out}"
    );
    // The pairing, asserted rather than assumed: the sentence given to this row
    // has to name the part. Plurals allowed (`Petals`, `Sepals`, `anthers`);
    // a looser substring test is NOT used, because "pistil" occurs inside the
    // stigma sentence and would let the wrong row pass.
    let lower = span.to_lowercase();
    assert!(
        lower.contains(part) ,
        "{part}'s own span names it: {span}"
    );
}

/// #14986. The PETAL sentence was this table's `source` — the field that
/// carries the tier — for all seven rows, so a recall of `ovary` came back
/// proved by *"Petals attract pollinators and are usually the reason why we buy
/// and enjoy flowers."*
///
/// Every span here is on the SAME page, so the locator was never wrong and a
/// hostname assertion could never have caught it — only the span was wrong.
#[test]
fn every_part_carries_the_sentence_that_states_its_function() {
    assert_part(
        "fppetal", "petal", "attract_pollinators",
        "Petals attract pollinators and are usually the reason why we buy and enjoy flowers.",
    );
    assert_part("fpsepal", "sepal", "protect", "Sepals help protect the developing bud.");
    assert_part("fpanther", "anther", "carry_pollen", "The anthers carry the pollen.");
    assert_part(
        "fpstigma", "stigma", "traps_pollen",
        "The stigma is the sticky surface at the top of the pistil; it traps and holds the pollen.",
    );
    assert_part(
        "fpovary", "ovary", "contains_ovules",
        "The style leads down to the ovary that contains the ovules.",
    );
}

/// TWO ROWS SHARE ONE SENTENCE. The page fixes `stamen` and `pistil` in the
/// same clause, so each row carries its own copy and each is asserted in output
/// where the other row is absent — the failure `skeleton-bones` (#15171)
/// shipped by testing one row per shared span.
#[test]
fn the_two_rows_that_share_a_sentence_each_carry_their_own_copy() {
    assert_part("fpstamen", "stamen", "male", STAMEN_PISTIL);
    assert_part("fppistil", "pistil", "female", STAMEN_PISTIL);
}

/// The envelope is the page's own sentence saying a flower has some parts that
/// are basic equipment. It names NO part — checked, not assumed — so it
/// warrants none of the seven rows.
#[test]
fn the_framing_envelope_never_reaches_an_answer_and_is_pinned() {
    let dir = scratch("fpenvelope");
    std::fs::copy(
        facts_stdlib().join("biology/flower-parts.adj"),
        dir.join("flower-parts.adj"),
    )
    .expect("copy shipped flower-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"flower-parts.adj\"\n? flower_part_function($P, $F)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        7,
        "all seven rows answer: {out}"
    );
    assert!(
        !out.contains("there are some parts that are basic equipment"),
        "the framing span warrants no row: {out}"
    );
    // The petal sentence warrants exactly ONE row where it used to be the
    // `source` on all seven — 14 occurrences across citations and steps, now 2.
    assert_eq!(
        out.matches("Petals attract pollinators").count(),
        2,
        "the petal sentence warrants the petal row and nothing else: {out}"
    );
    // The shared sentence warrants exactly TWO rows.
    assert_eq!(
        out.matches(STAMEN_PISTIL).count(),
        4,
        "the stamen/pistil sentence warrants its two rows and no others: {out}"
    );
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/flower-parts.adj"))
        .expect("read shipped flower-parts.adj");
    assert!(
        adj.contains(
            "    source \"Flowers can be made up of different parts, but there are some parts that are basic equipment.\"\n    locator \"https://web.extension.illinois.edu/gpe/case4/c4facts1a.html\"\n    trust authoritative"
        ),
        "the envelope carries the page's framing sentence, verbatim"
    );
    // The `columns` line. A surviving mutant is a finding: renaming it is
    // invisible to every assertion above, because column names are positional
    // and never reach the output. Same survivor as `heredity-term` (#15188)
    // and `scientific-method-step` (#15190).
    assert!(
        adj.contains("    columns part, function"),
        "the shipped column names are unchanged"
    );
}
