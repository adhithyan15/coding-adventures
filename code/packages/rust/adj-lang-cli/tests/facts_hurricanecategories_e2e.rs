//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/hurricane-categories.adj`) driven through the
//! built CLI: a native `table` of the five Saffir-Simpson Hurricane Wind Scale
//! categories → their NHC damage descriptor resolves binding-query recalls
//! (forward AND backward) with the NOAA / National Hurricane Center citation,
//! and abstains on a key that is not one of the five categories (category_6) —
//! 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the category_1 span, so a category-3, -4 or -5 answer was warranted
//! primarily by a sentence about category 1.
//!
//! CARE WITH THE DUPLICATE VALUE: category_4 and category_5 share the
//! descriptor `catastrophic_damage`, because the NHC page opens both
//! paragraphs with "Catastrophic damage will occur." Their SPANS differ, so a
//! negative arm may assert that another row's SPAN is absent, never that the
//! descriptor atom is.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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
fn meteorology_hurricane_categories_recall_binds_descriptor_with_citation() {
    let dir = scratch("hurricanecategories");
    // Copy the shipped meteorology table beside the entry program and import it.
    let src = facts_stdlib().join("meteorology/hurricane-categories.adj");
    std::fs::copy(&src, dir.join("hurricane-categories.adj"))
        .expect("copy shipped hurricane-categories.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"hurricane-categories.adj\"\n\
         ? damage_level(category_1, $Descriptor)\n\
         ? damage_level(category_3, $Descriptor)\n\
         ? damage_level(category_4, $Descriptor)\n\
         ? damage_level(category_5, $Descriptor)\n\
         ? damage_level($Category, catastrophic_damage)\n\
         ? damage_level(category_6, $Descriptor)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");

    // (a) A category-1 hurricane produces "some damage" — the recalled
    // descriptor (forward bind) — and the answer carries the NHC citation at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("\"Descriptor\":\"some_damage\""),
        "category_1 → some_damage: {out}"
    );
    // This was `contains("nhc.noaa.gov") && contains("\"trust\":\"authoritative\"")`,
    // which any NHC citation satisfies and which constrains no sentence text.
    // Before the RS-5e conversion it was satisfied by the category_1 span
    // riding on every answer -- including the category_5 one. The needle is now
    // the whole citations array, closing on both the corroborations `]` and the
    // citations `]`.
    assert!(
        out.contains(&only_citation(CAT1)),
        "the category_1 answer carries the NHC sentence about category 1, whole: {out}"
    );
    assert!(
        out.contains(&only_citation(CAT5)),
        "the category_5 answer carries the sentence about category 5, not category 1: {out}"
    );
    assert!(!out.contains(ENVELOPE), "the envelope is primary for no answer: {out}");

    // A category 3 brings "devastating damage"; a category 4 and a category 5
    // both bring "catastrophic damage" (the honest duplicate — the NHC source
    // opens both paragraphs with the same "Catastrophic damage will occur").
    assert!(
        out.contains("\"Descriptor\":\"devastating_damage\""),
        "category_3 → devastating_damage: {out}"
    );
    assert!(
        out.contains("\"Descriptor\":\"catastrophic_damage\""),
        "category_4 / category_5 → catastrophic_damage: {out}"
    );

    // The relation runs BACKWARD: bind the descriptor `catastrophic_damage`, and
    // recall the categories that carry it — HONESTLY both category_4 AND
    // category_5, because the source states the same descriptor for each.
    assert!(
        out.contains("\"Category\":\"category_4\""),
        "catastrophic_damage → category_4 (reverse recall): {out}"
    );
    assert!(
        out.contains("\"Category\":\"category_5\""),
        "catastrophic_damage → category_5 (reverse recall): {out}"
    );

    // (b) There is no category 6 on the Saffir-Simpson scale — honest
    // abstention, never a fabricated damage word.
    assert!(
        out.contains("\"abstained\":true"),
        "category_6 abstains: {out}"
    );
}

const LOCATOR: &str = "https://www.nhc.noaa.gov/aboutsshws.php";
const ENVELOPE: &str = "The Saffir-Simpson Hurricane Wind Scale estimates potential property damage.";
const CAT1: &str = "Very dangerous winds will produce some damage: Well-constructed frame homes could have damage to roof, shingles, vinyl siding and gutters.";
const CAT2: &str = "Extremely dangerous winds will cause extensive damage: Well-constructed frame homes could sustain major roof and siding damage.";
const CAT3: &str = "Devastating damage will occur: Well-built framed homes may incur major damage or removal of roof decking and gable ends.";
/// Shares its VALUE with category_5 but not its span.
const CAT4: &str = "Catastrophic damage will occur: Well-built framed homes can sustain severe damage with loss of most of the roof structure and/or some exterior walls.";
/// Shares its VALUE with category_4 but not its span.
const CAT5: &str = "Catastrophic damage will occur: A high percentage of framed homes will be destroyed, with total roof failure and wall collapse.";

/// (category, its descriptor atom, the NHC sentence stating that descriptor)
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("category_1", "some_damage", CAT1),
        ("category_2", "extensive_damage", CAT2),
        ("category_3", "devastating_damage", CAT3),
        ("category_4", "catastrophic_damage", CAT4),
        ("category_5", "catastrophic_damage", CAT5),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("meteorology/hurricane-categories.adj"))
        .expect("read shipped hurricane-categories.adj");
    adj[adj.find("table damage_level").expect("table")..].to_string()
}

#[test]
fn every_category_answer_carries_only_the_sentence_about_that_category() {
    for (category, descriptor, span) in scale() {
        let dir = scratch(&format!("cat_{category}"));
        let src = facts_stdlib().join("meteorology/hurricane-categories.adj");
        std::fs::copy(&src, dir.join("hurricane-categories.adj"))
            .expect("copy shipped hurricane-categories.adj");
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"hurricane-categories.adj\"\n? damage_level({category}, $Descriptor)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {category}: {out}");
        assert!(
            out.contains(&format!("\"Descriptor\":\"{descriptor}\"")),
            "{category} -> {descriptor}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{category}: the NHC sentence about it, whole, and the only citation: {out}"
        );
        // Spans are exclusive even where VALUES are shared: category_4 and
        // category_5 both mean catastrophic_damage, so only the span may be
        // asserted absent here, never the descriptor atom.
        for other in [CAT1, CAT2, CAT3, CAT4, CAT5] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another category's sentence must not reach {category}: {out}"
                );
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {category}: {out}");
    }
}

#[test]
fn the_two_catastrophic_categories_cite_different_paragraphs() {
    // The honest duplicate: the page opens both paragraphs with "Catastrophic
    // damage will occur." and then describes different outcomes. Sharing a
    // value must not become sharing a citation -- an edit that deduplicated by
    // value would leave the forward and backward binds passing unchanged.
    let body = shipped_table();
    assert!(
        body.contains(&format!("        source \"{CAT4}\"\n")),
        "category_4 cites the paragraph about category 4: {body}"
    );
    assert!(
        body.contains(&format!("        source \"{CAT5}\"\n")),
        "category_5 cites the paragraph about category 5: {body}"
    );
    assert_ne!(CAT4, CAT5, "the two spans must not be the same sentence");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the tier.
    let body = shipped_table();
    assert_eq!(body.matches("\n        source \"").count(), 5, "five row sources");
    // Keyword-anchored, not indent-scoped: a row-level `cites` at eight spaces
    // must fail this too, without false-positiving on prose in a comment.
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "no corroboration at any indent"
    );
    // Indent-independent, like the `cites` arm above -- but `cites` can assert
    // ABSENCE because no table-level `cites` exists, whereas the envelope has a
    // `locator` and a `trust` of its own. So the pin is that there is EXACTLY
    // one of each: a row restating either, at any indent, shows up as a second.
    assert_eq!(
        body.lines().filter(|l| l.trim_start().starts_with("locator ")).count(),
        1,
        "exactly one locator line, the envelope's: {body}"
    );
    assert_eq!(
        body.lines().filter(|l| l.trim_start().starts_with("trust ")).count(),
        1,
        "exactly one trust line, the envelope's: {body}"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n"
        )),
        "the envelope is the page's scale-defining sentence, at the authoritative tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{CAT1}\"\n    locator")),
        "not the category_1 span as the envelope again"
    );
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    // The BARE word, not "category 1".."category 5": the header and the
    // CHANGELOG both claim the envelope names NO category, and a list of
    // space-separated needles would let "category-3" or "Category Three"
    // through while contradicting that claim. One needle subsumes all five.
    for word in ["category", "some damage", "extensive damage", "devastating", "catastrophic"] {
        assert!(
            !shipped_envelope.contains(word),
            "the shipped envelope must name no category and no damage value, but contains {word:?}"
        );
    }
}
