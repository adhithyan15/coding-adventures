//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/noun-type.adj`) driven through the built
//! CLI: a native `table` naming six noun types and what each actually
//! is, quoted from Grammarly's "What Is a Noun? Definition, Types, and
//! Examples" article. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the common-noun row's own sentence, the primary source of all six answers;
//! it is now a framing sentence from the article, which every row overrides.
//! Each row's citation is pinned whole, and the countable and uncountable
//! rows carry their whole sentences (the header quotes stop short of both).

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_noun_type_{tag}_{}", std::process::id()));
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

fn place_lib(dir: &Path) {
    let src = facts_stdlib().join("language/noun-type.adj");
    std::fs::copy(&src, dir.join("noun-type.adj")).expect("copy shipped noun-type.adj");
}

const LOCATOR: &str = "https://www.grammarly.com/blog/parts-of-speech/nouns/";
const ENVELOPE: &str = "Nouns are everywhere in our writing.";
const COMMON: &str = "A common noun is the generic name of an item in a class or group.";
const COLLECTIVE: &str = "A collective noun denotes a group or collection of people or things.";
const ABSTRACT: &str = "An abstract noun is something that cannot be perceived by the senses.";
const CONCRETE: &str = "A concrete noun is something that is perceived by the senses, something physical or tangible.";
const COUNTABLE: &str = "Countable nouns can be counted, even if the resulting number would be extraordinarily high (like the number of humans in the world).";
const UNCOUNTABLE: &str = "Uncountable nouns, or mass nouns, are nouns that are impossible to count, whether because they name intangible concepts (information, animal husbandry, wealth), collections of things that are considered as wholes (jewelry, equipment, the working class), or homogeneous physical substances (milk, sand, air).";

/// (type, definition, that row's own sentence)
const ROWS: [(&str, &str, &str); 6] = [
    ("common_noun", "generic_name_of_an_item_in_a_class_or_group", COMMON),
    ("collective_noun", "denotes_a_group_or_collection_of_people_or_things", COLLECTIVE),
    ("abstract_noun", "cannot_be_perceived_by_the_senses", ABSTRACT),
    ("concrete_noun", "perceived_by_the_senses_physical_or_tangible", CONCRETE),
    ("countable_noun", "can_be_counted", COUNTABLE),
    ("uncountable_noun", "impossible_to_count", UNCOUNTABLE),
];

/// The whole citation a row's answer carries, as one contiguous run.
fn citation(sentence: &str) -> String {
    format!("\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("language/noun-type.adj"))
        .expect("read shipped noun-type.adj");
    adj[adj.find("table noun_type").expect("table")..].to_string()
}

#[test]
fn noun_type_recall_binds_the_definition_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type(common_noun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"generic_name_of_an_item_in_a_class_or_group\""),
        "common_noun means generic_name_of_an_item_in_a_class_or_group: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("grammarly.com") && contains(trust)` (#15209).
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(
        out.contains(&format!("\"citations\":[{{{}}}]", citation(COMMON))),
        "the common-noun sentence is the only citation: {out}"
    );
}

#[test]
fn noun_type_reverse_binds_the_type_for_that_definition() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type($T, cannot_be_perceived_by_the_senses)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"abstract_noun\""),
        "the shipped cannot_be_perceived_by_the_senses example is abstract_noun: {out}"
    );
}

#[test]
fn noun_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type(possessive_noun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "possessive_noun is a real noun type the source mentions but not one of the six tabled here -- honest abstention, never invented: {out}"
    );
}

#[test]
fn noun_type_recall_binds_a_newly_added_row_directly() {
    let dir = scratch("direct_new");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type(concrete_noun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"D\":\"perceived_by_the_senses_physical_or_tangible\""),
        "concrete_noun means perceived_by_the_senses_physical_or_tangible: {out}"
    );
}

#[test]
fn noun_type_reverse_binds_a_newly_added_row() {
    let dir = scratch("reverse_new");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type($T, impossible_to_count)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"uncountable_noun\""),
        "the shipped impossible_to_count example is uncountable_noun: {out}"
    );
}

#[test]
fn noun_type_abstains_honestly_on_a_bundled_fact_candidate() {
    let dir = scratch("abstain_bundled");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"noun-type.adj\"\n\
         ? noun_type(proper_noun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "proper_noun is a real noun type the same source page defines, but its sentence bundles the naming function with a separate capitalization rule -- honest abstention, never invented: {out}"
    );
}

#[test]
fn every_noun_type_answer_carries_its_own_sentence() {
    for (kind, def, sentence) in ROWS {
        let dir = scratch(&format!("row_{kind}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"noun-type.adj\"\n? noun_type($T, {def})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {kind}: {out}");
        assert!(out.contains(&format!("\"T\":\"{kind}\"")), "{def} binds {kind}: {out}");
        assert!(
            out.contains(&format!("\"citations\":[{{{}}}]", citation(sentence))),
            "{kind}: its own sentence, whole, and the only citation: {out}"
        );
        for (other, _, other_sentence) in ROWS {
            if other != kind {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {kind}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {kind}: {out}");
    }
}

#[test]
fn the_countable_rows_carry_their_whole_sentences() {
    // The header's truth table quotes both sentences only up to a comma or a
    // parenthetical, with no closing period; on the page each goes on. The
    // rows carry the whole sentence, never the cut form.
    let body = shipped_table();
    for cut in [
        "Countable nouns can be counted, even if the resulting number would be extraordinarily high\"",
        "Uncountable nouns, or mass nouns, are nouns that are impossible to count\"",
    ] {
        assert!(!body.contains(cut), "the table carries no cut quote: {cut}");
    }
    assert!(COUNTABLE.ends_with("(like the number of humans in the world)."));
    assert!(UNCOUNTABLE.ends_with("homogeneous physical substances (milk, sand, air)."));
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (kind, def, sentence) in ROWS {
        let expected = format!("    row ({kind}, {def}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({kind}, {def}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 6, "six row sources");
    assert!(!body.contains("cites "), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n")),
        "the envelope is the article's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{COMMON}\"\n    locator")), "not the common-noun sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["common", "collective", "abstract", "concrete", "count", "generic", "group", "senses", "tangible", "class", "mass"] {
        assert!(!folded.contains(word), "the envelope must name no type or definition, but contains {word:?}");
    }
}
