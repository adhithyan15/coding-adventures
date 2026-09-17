//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/hurricane-category-home-damage.adj`)
//! driven through the built CLI: a native `table` naming the SPECIFIC
//! well-built-home damage effect the NHC describes for each Saffir-Simpson
//! hurricane category -- a sibling to the already-shipped
//! `hurricane-categories.adj` (which only carries ONE generic damage word
//! per category), reading the same NHC sentences for a different column.
//! Resolves binding-query recall (both directions) with the source's
//! citation, and abstains on a category that is not one of the five
//! Saffir-Simpson categories -- 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the category_1 span, so a category-2, -3, -4 or -5 answer was warranted
//! primarily by a sentence about category 1.
//!
//! All five VALUES here are distinct, unlike the sibling table's duplicated
//! `catastrophic_damage`. The negative arms still assert absence of SPANS
//! rather than of atoms, so they do not quietly become wrong if a future edit
//! collapses two values onto one word.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_hurricanehomedamage_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("meteorology/hurricane-category-home-damage.adj");
    std::fs::copy(&src, dir.join("hurricane-category-home-damage.adj"))
        .expect("copy shipped hurricane-category-home-damage.adj");
}

#[test]
fn hurricane_home_damage_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"hurricane-category-home-damage.adj\"\n\
         ? hurricane_home_damage(category_1, $HomeDamageEffect)\n\
         ? hurricane_home_damage(category_5, $HomeDamageEffect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"hurricane_home_damage(category_1, damage_to_roof_shingles_vinyl_siding_and_gutters)\""),
        "category_1 effect: {out}"
    );
    assert!(
        out.contains("\"term\":\"hurricane_home_damage(category_5, total_roof_failure_and_wall_collapse)\""),
        "category_5 effect: {out}"
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
    assert!(
        !out.contains(ENVELOPE),
        "the envelope is primary for no answer: {out}"
    );
}

#[test]
fn hurricane_home_damage_recalls_backward_from_a_bound_effect() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"hurricane-category-home-damage.adj\"\n\
         ? hurricane_home_damage($Category, total_roof_failure_and_wall_collapse)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Category\":\"category_5\""),
        "total roof failure and wall collapse names category_5: {out}"
    );
}

#[test]
fn hurricane_home_damage_abstains_honestly_on_category_6() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"hurricane-category-home-damage.adj\"\n\
         ? hurricane_home_damage(category_6, $HomeDamageEffect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "there is no category 6 on the Saffir-Simpson scale -- honest abstention: {out}"
    );
}

const LOCATOR: &str = "https://www.nhc.noaa.gov/aboutsshws.php";
const ENVELOPE: &str = "The Saffir-Simpson Hurricane Wind Scale estimates potential property damage.";
const CAT1: &str = "Very dangerous winds will produce some damage: Well-constructed frame homes could have damage to roof, shingles, vinyl siding and gutters.";
const CAT2: &str = "Extremely dangerous winds will cause extensive damage: Well-constructed frame homes could sustain major roof and siding damage.";
const CAT3: &str = "Devastating damage will occur: Well-built framed homes may incur major damage or removal of roof decking and gable ends.";
const CAT4: &str = "Catastrophic damage will occur: Well-built framed homes can sustain severe damage with loss of most of the roof structure and/or some exterior walls.";
const CAT5: &str = "Catastrophic damage will occur: A high percentage of framed homes will be destroyed, with total roof failure and wall collapse.";

/// (category, its structural-effect atom, the NHC sentence stating that effect)
///
/// Unlike the sibling `damage_level` table, all five VALUES here are distinct --
/// the page's home-damage detail differs category by category even where its
/// summary damage word repeats. The negative arms below still assert absence of
/// SPANS rather than of atoms, so this test does not quietly become wrong if a
/// future edit ever collapses two values onto one word.
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("category_1", "damage_to_roof_shingles_vinyl_siding_and_gutters", CAT1),
        ("category_2", "major_roof_and_siding_damage", CAT2),
        ("category_3", "major_damage_or_removal_of_roof_decking_and_gable_ends", CAT3),
        (
            "category_4",
            "severe_damage_with_loss_of_most_of_the_roof_structure_and_or_some_exterior_walls",
            CAT4,
        ),
        ("category_5", "total_roof_failure_and_wall_collapse", CAT5),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
///
/// This is what replaces `contains("nhc.noaa.gov") && contains("\"trust\":...")`:
/// that needle is satisfied by ANY NHC citation and constrains no sentence text,
/// so before the RS-5e conversion it was satisfied by the category-1 span riding
/// on every answer.
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(
        facts_stdlib().join("meteorology/hurricane-category-home-damage.adj"),
    )
    .expect("read shipped hurricane-category-home-damage.adj");
    adj[adj.find("table hurricane_home_damage").expect("table")..].to_string()
}

#[test]
fn every_category_answer_carries_only_the_sentence_about_that_category() {
    for (category, effect, span) in scale() {
        let dir = scratch(&format!("cat_{category}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!(
                "import \"hurricane-category-home-damage.adj\"\n? hurricane_home_damage({category}, $HomeDamageEffect)\n"
            ),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "one answer for {category}: {out}"
        );
        assert!(
            out.contains(&format!("\"HomeDamageEffect\":\"{effect}\"")),
            "{category} -> {effect}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{category}: the NHC sentence about it, whole, and the only citation: {out}"
        );
        for other in [CAT1, CAT2, CAT3, CAT4, CAT5] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another category's sentence must not reach {category}: {out}"
                );
            }
        }
        assert!(
            !out.contains(ENVELOPE),
            "the envelope is not primary for {category}: {out}"
        );
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the tier.
    let body = shipped_table();
    assert_eq!(
        body.matches("\n        source \"").count(),
        5,
        "five row sources"
    );
    // Keyword-anchored, not indent-scoped: a row-level `cites` at eight spaces
    // must fail this too, without false-positiving on prose in a comment.
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "no corroboration at any indent"
    );
    // Indent-independent, unlike the `cites` arm above -- `cites` can assert
    // ABSENCE because no table-level `cites` exists, whereas the envelope has a
    // `locator` and a `trust` of its own. So the pin is EXACTLY one of each: a
    // row restating either, at any indent, shows up as a second.
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("locator "))
            .count(),
        1,
        "exactly one locator line, the envelope's: {body}"
    );
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("trust "))
            .count(),
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
    // The BARE word, not "category 1".."category 5": the header claims the
    // envelope names NO category, and space-separated needles would let
    // "category-3" or "Category Three" through.
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    for word in ["category", "roof", "siding", "gutters", "wall collapse"] {
        assert!(
            !shipped_envelope.contains(word),
            "the shipped envelope must name no category and no structural effect, but contains {word:?}"
        );
    }
}

#[test]
fn every_row_value_is_supported_by_the_span_that_row_carries() {
    // The "does the span name its row key?" check, in the form this schema
    // needs: the VALUE is a structural-effect phrase, so the span must contain
    // that phrase. This is what `element-categories` (#15318) failed.
    let body = shipped_table();
    for (category, effect, span) in scale() {
        assert!(
            body.contains(&format!("        source \"{span}\"\n")),
            "{category} carries its own span: {body}"
        );
        let normalized: String = span
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        let phrase = effect.replace("_and_or_", " and or ").replace('_', " ");
        assert!(
            normalized.contains(&phrase),
            "{category}: the value {effect:?} must be stated by its own span, \
             but {phrase:?} is not in {normalized:?}"
        );
    }
}
