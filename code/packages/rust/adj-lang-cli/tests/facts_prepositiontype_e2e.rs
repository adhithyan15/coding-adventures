//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/preposition-type.adj`) driven through the
//! built CLI: a native `table` naming three preposition types and what
//! each actually shows, quoted verbatim from Grammarly's "Prepositions:
//! Definition, Types, and Examples" article. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the place row's own sentence, the primary source of all three answers; it
//! is now the article's framing sentence, which every row overrides. Each
//! row's citation is pinned whole, and the direction sentence carries the
//! page's U+2019 (the header had quoted it with a straight apostrophe).

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_preposition_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/preposition-type.adj");
    std::fs::copy(&src, dir.join("preposition-type.adj")).expect("copy shipped preposition-type.adj");
}

const LOCATOR: &str = "https://www.grammarly.com/blog/parts-of-speech/prepositions/";
const ENVELOPE: &str = "However, the most common prepositions fit into four main categories, with a fifth category for additional types.";
const PLACE: &str = "Prepositions of place show where something is or where something happened.";
const TIME: &str = "Prepositions of time show when something happened or will happen (and sometimes its duration).";
const DIRECTION: &str = "Prepositions of direction or movement show how something is moving or which way it\u{2019}s going.";

/// (type, description, that row's own sentence)
const ROWS: [(&str, &str, &str); 3] = [
    ("preposition_of_place", "shows_where_something_is_or_where_something_happened", PLACE),
    ("preposition_of_time", "shows_when_something_happened_or_will_happen", TIME),
    ("preposition_of_direction", "shows_how_something_is_moving_or_which_way_its_going", DIRECTION),
];

/// The whole citation a row's answer carries, as one contiguous run.
fn citation(sentence: &str) -> String {
    format!("\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("language/preposition-type.adj"))
        .expect("read shipped preposition-type.adj");
    adj[adj.find("table preposition_type").expect("table")..].to_string()
}

#[test]
fn preposition_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"preposition-type.adj\"\n\
         ? preposition_type(preposition_of_time, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"shows_when_something_happened_or_will_happen\""),
        "preposition_of_time means shows_when_something_happened_or_will_happen: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("grammarly.com") && contains(trust)` (#15209).
    assert!(out.contains(&citation(TIME)), "carries the time sentence, whole: {out}");
}

#[test]
fn preposition_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"preposition-type.adj\"\n\
         ? preposition_type($T, shows_how_something_is_moving_or_which_way_its_going)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"preposition_of_direction\""),
        "the shipped shows_how_something_is_moving_or_which_way_its_going example is preposition_of_direction: {out}"
    );
}

#[test]
fn preposition_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"preposition-type.adj\"\n\
         ? preposition_type(preposition_of_manner_cause_or_purpose, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "preposition_of_manner_cause_or_purpose is a real category the source covers but bundles three distinct functions, not one of the three clean single-concept types tabled here -- honest abstention, never invented: {out}"
    );
}

#[test]
fn every_type_answer_carries_its_own_sentence() {
    for (kind, desc, sentence) in ROWS {
        let dir = scratch(&format!("row_{kind}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"preposition-type.adj\"\n? preposition_type($T, {desc})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {kind}: {out}");
        assert!(out.contains(&format!("\"T\":\"{kind}\"")), "{desc} binds {kind}: {out}");
        assert!(out.contains(&citation(sentence)), "{kind}: its own sentence, whole: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != kind {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {kind}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {kind}: {out}");
    }
}

#[test]
fn the_direction_row_carries_the_pages_curly_apostrophe() {
    // The header quoted this sentence with a STRAIGHT apostrophe while calling
    // it verbatim; in that form it occurs on no page. The row and the header
    // now carry U+2019.
    let body = shipped_table();
    assert!(DIRECTION.contains("it\u{2019}s going"), "the direction sentence uses U+2019");
    assert!(!body.contains("which way it's going"), "no straight-apostrophe form in the table");
    let adj = std::fs::read_to_string(facts_stdlib().join("language/preposition-type.adj")).unwrap();
    assert!(!adj.contains("which way it's going"), "no straight-apostrophe form anywhere in the file, header included");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (kind, desc, sentence) in ROWS {
        let expected = format!("    row ({kind}, {desc}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({kind}, {desc}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("\n        cites "), "no row corroboration");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n")),
        "the envelope is the article's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{PLACE}\"\n    locator")), "not the place sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["place", "time", "direction", "movement", "where", "when", "moving"] {
        assert!(!folded.contains(word), "the envelope must name no type or what it shows, but contains {word:?}");
    }
}
