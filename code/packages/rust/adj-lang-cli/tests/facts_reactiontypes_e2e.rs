//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/reaction-types.adj`) driven through the built
//! CLI: a native `table` of the five basic chemical-reaction types → the
//! defining token each is described by resolves binding-query recalls (forward
//! AND backward) with the source's LibreTexts citation, and abstains on a word
//! that is not one of the five basic reaction types (neutralization) — 0 model
//! calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the combination sentence, the primary source of all five answers; it is now
//! a framing sentence from the page, which every row overrides. The
//! single-replacement sentence carries the page's U+00A0 after "fourth". Each
//! answer's citations array is pinned whole.

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

const LOCATOR: &str = "https://chem.libretexts.org/Courses/Anoka-Ramsey_Community_College/Introduction_to_Chemistry/07:_Chemical_Reactions/7.05:_Classifying_Chemical_Reactions";
const ENVELOPE: &str = "The key to success is to find useful ways to categorize reactions.";
const COMBINATION: &str = "A combination reaction is a reaction in which two or more substances combine to form a single new substance.";
const DECOMPOSITION: &str = "A decomposition reaction is a reaction in which a compound breaks down into two or more simpler substances.";
const SINGLE: &str = "A fourth\u{a0}type of reaction is the single replacement reaction, in which one element replaces a similar element in a compound.";
const DOUBLE: &str = "A double-replacement reaction (also called double-displacement reaction) is a reaction in which the positive and negative ions of two ionic compounds exchange places to form two new compounds.";
const COMBUSTION: &str = "A combustion reaction is a reaction in which a substance reacts with oxygen gas, releasing energy in the form of light and heat.";

/// (type, defining token, that row's own sentence)
const ROWS: [(&str, &str, &str); 5] = [
    ("combination", "two_or_more_combine", COMBINATION),
    ("decomposition", "breaks_down", DECOMPOSITION),
    ("single_replacement", "one_element_replaces", SINGLE),
    ("double_replacement", "ions_exchange_places", DOUBLE),
    ("combustion", "reacts_with_oxygen", COMBUSTION),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(sentence: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("chemistry/reaction-types.adj"))
        .expect("read shipped reaction-types.adj");
    adj[adj.find("table reaction_defining").expect("table")..].to_string()
}

fn ask(tag: &str, query: &str) -> String {
    let dir = scratch(tag);
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/reaction-types.adj");
    std::fs::copy(&src, dir.join("reaction-types.adj")).expect("copy shipped reaction-types.adj");
    std::fs::write(dir.join("case.adj"), format!("import \"reaction-types.adj\"\n{query}")).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn chemistry_reaction_types_recall_binds_defining_with_citation() {
    let out = ask(
        "reactiontypes",
        "? reaction_defining(combination, $Defining)\n\
         ? reaction_defining(combustion, $Defining)\n\
         ? reaction_defining(double_replacement, $Defining)\n\
         ? reaction_defining($Type, breaks_down)\n\
         ? reaction_defining(neutralization, $Defining)\n",
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A combination reaction combines two or more substances, combustion burns
    // in oxygen, double-replacement swaps ions — the recalled defining tokens
    // (forward binds).
    assert!(
        out.contains("\"Defining\":\"two_or_more_combine\""),
        "combination → two_or_more_combine: {out}"
    );
    assert!(
        out.contains("\"Defining\":\"reacts_with_oxygen\""),
        "combustion → reacts_with_oxygen: {out}"
    );
    assert!(
        out.contains("\"Defining\":\"ions_exchange_places\""),
        "double_replacement → ions_exchange_places: {out}"
    );
    // The relation runs BACKWARD: bind the defining token `breaks_down`, recall
    // its reaction type.
    assert!(
        out.contains("\"Type\":\"decomposition\""),
        "breaks_down → decomposition (reverse recall): {out}"
    );
    // Each answer carries its own row's LibreTexts sentence, whole, at the
    // `consensus` tier — not `contains("chem.libretexts.org") && contains(trust)`
    // (#15209), and no longer the combination sentence for every answer (#14986).
    assert_eq!(out.matches("\"citations\":[").count(), 4, "four answers: {out}");
    for sentence in [COMBINATION, COMBUSTION, DOUBLE, DECOMPOSITION] {
        assert!(out.contains(&only_citation(sentence)), "an answer carries only its own sentence: {sentence}: {out}");
    }
    // Neutralization is a reaction but not one of the five basic classification
    // types in this table — honest abstention, never a fabricated definition.
    assert!(
        out.contains("\"abstained\":true"),
        "neutralization abstains: {out}"
    );
}

#[test]
fn every_defining_answer_carries_its_own_sentence() {
    for (kind, defining, sentence) in ROWS {
        let out = ask(&format!("row_{kind}"), &format!("? reaction_defining($Type, {defining})\n"));
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {defining}: {out}");
        assert!(out.contains(&format!("\"Type\":\"{kind}\"")), "{defining} is the {kind} reaction's: {out}");
        assert!(out.contains(&only_citation(sentence)), "{kind}: its own sentence, whole, and the only citation: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != kind {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {kind}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {kind}: {out}");
    }
}

#[test]
fn the_single_replacement_row_carries_the_pages_no_break_space() {
    // The header quoted this sentence with a plain space after "fourth"; the page
    // writes U+00A0 there, and the plain-space form occurs on no page.
    let body = shipped_table();
    assert_eq!(SINGLE.matches('\u{a0}').count(), 1, "one U+00A0, after \"fourth\"");
    assert!(body.contains(&format!("        source \"{SINGLE}\"\n")), "shipped in the page's characters");
    assert!(!body.contains("A fourth type of reaction"), "no plain-space form in the table");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (kind, defining, sentence) in ROWS {
        let expected = format!("    row ({kind}, {defining}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({kind}, {defining}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 5, "five row sources");
    assert!(!body.contains("cites "), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{COMBINATION}\"\n    locator")), "not the combination sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["combination", "decomposition", "replacement", "combustion", "combine", "break", "oxygen", "exchange"] {
        assert!(!folded.contains(word), "the envelope must name no type or defining word, but contains {word:?}");
    }
}
