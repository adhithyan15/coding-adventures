//! End-to-end test for the transportation FACTS library
//! (`adj-facts-stdlib/transportation/sign-element-color.adj`) driven
//! through the built CLI: a THREE-column `table` recording the colour the
//! federal standard specifies for each PART of a traffic sign, grounding
//! the FHWA's Manual on Uniform Traffic Control Devices.
//!
//! WHICH PART OF THE SIGN IS PART OF THE FACT. "A YIELD sign is red" is
//! FALSE as stated -- its BACKGROUND is white; red is its border and its
//! legend. A two-column `sign_color(sign, colour)` would flatten three
//! different claims into one and get the most recognisable sign in the
//! country wrong, while carrying a federal citation.
//!
//! `yield_signs_background_is_white_not_red` is the test that matters:
//! it pins the exact misconception the third column exists to prevent.
//!
//! EACH ROW CARRIES ITS OWN SIGN'S SENTENCE (RS-5e, #14986). The envelope
//! used to be the STOP sentence, the primary source of all nine answers; it
//! is now Chapter 2A's framing sentence, which every row overrides. The
//! per-row citations are pinned whole -- sentence, chapter locator, trust and
//! the empty corroboration list, as one contiguous run.
//!
//! Every assertion uses the JOINT binding form rather than independent
//! substring scans, and both abstention tests carry positive controls.
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
    let dir = std::env::temp_dir().join(format!("adjcli_factssigncolor_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("transportation/sign-element-color.adj");
    std::fs::copy(&src, dir.join("sign-element-color.adj"))
        .expect("copy shipped sign-element-color.adj");
}

fn case(dir: &Path, query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    std::fs::write(
        &path,
        format!("import \"sign-element-color.adj\"\n? {query}\n"),
    )
    .unwrap();
    path
}

const PART2A: &str = "https://mutcd.fhwa.dot.gov/htm/2009/part2/part2a.htm";
const PART2B: &str = "https://mutcd.fhwa.dot.gov/htm/2009/part2/part2b.htm";
const PART2C: &str = "https://mutcd.fhwa.dot.gov/htm/2009/part2/part2c.htm";

const ENVELOPE: &str = "Standardized colors and shapes are specified so that the several classes of traffic signs can be promptly recognized.";
const STOP: &str = "The STOP sign shall be an octagon with a white legend and border on a red background.";
const YIELD: &str = "The YIELD (R1-2) sign (see Figure 2B-1) shall be a downward-pointing equilateral triangle with a wide red border and the legend YIELD in red on a white background.";
const WARNING: &str = "Except as provided in Paragraph 2 or unless specifically designated otherwise, all warning signs shall be diamond-shaped (square with one diagonal vertical) with a black legend and border on a yellow background.";

/// (sign, element, color, that sign's sentence, the chapter it was measured on)
const ROWS: [(&str, &str, &str, &str, &str); 9] = [
    ("stop_sign", "background", "red", STOP, PART2B),
    ("stop_sign", "legend", "white", STOP, PART2B),
    ("stop_sign", "border", "white", STOP, PART2B),
    ("yield_sign", "background", "white", YIELD, PART2B),
    ("yield_sign", "legend", "red", YIELD, PART2B),
    ("yield_sign", "border", "red", YIELD, PART2B),
    ("warning_sign", "background", "yellow", WARNING, PART2C),
    ("warning_sign", "legend", "black", WARNING, PART2C),
    ("warning_sign", "border", "black", WARNING, PART2C),
];

/// The whole citation a row's answer carries: its sentence, its chapter, the
/// tier, and NO corroborations -- one contiguous run, so a swapped locator, a
/// changed tier or an added `cites` each break it.
fn citation(sentence: &str, locator: &str) -> String {
    format!("\"source\":\"{sentence}\",\"locator\":\"{locator}\",\"trust\":\"authoritative\",\"corroborations\":[]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("transportation/sign-element-color.adj"))
        .expect("read shipped sign-element-color.adj");
    adj[adj.find("table sign_element_color").expect("table")..].to_string()
}

#[test]
fn a_stop_signs_background_is_red() {
    let dir = scratch("stopbg");
    place(&dir);
    let program = case(&dir, "sign_element_color(stop_sign, background, $C)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN, now the whole contiguous run: the STOP
    // sentence, Chapter 2B, the tier and no corroborations. A fragment
    // needle once let a citation be truncated while the test stayed green
    // (issues #13916 and #13918).
    assert!(
        out.contains(&citation(STOP, PART2B)),
        "the citation is the STOP sentence at Chapter 2B, whole: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"bindings\":{\"C\":\"red\"}"),
        "the STOP sign's background is red: {out}"
    );
}

#[test]
fn yield_signs_background_is_white_not_red() {
    let dir = scratch("yieldbg");
    place(&dir);
    let program = case(&dir, "sign_element_color(yield_sign, background, $C)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // THE TEST THAT MATTERS. "A yield sign is red" is the misconception the
    // third column exists to prevent: red is its border and legend, and its
    // background is white. A two-column table would answer "red" here and
    // be wrong with a federal citation attached.
    assert!(
        out.contains("\"bindings\":{\"C\":\"white\"}"),
        "the YIELD sign's background is WHITE: {out}"
    );
    assert!(
        !out.contains("\"bindings\":{\"C\":\"red\"}"),
        "the background must not come back red: {out}"
    );
}

#[test]
fn both_red_parts_of_a_yield_sign_are_returned() {
    let dir = scratch("yieldred");
    place(&dir);
    let program = case(&dir, "sign_element_color(yield_sign, $E, red)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // Reverse on the ELEMENT: which parts are red? Both, and the source
    // states both, so returning only one would under-report the sentence.
    assert!(
        out.contains("\"bindings\":{\"E\":\"legend\"}")
            && out.contains("\"bindings\":{\"E\":\"border\"}"),
        "both the legend and the border are red: {out}"
    );
    assert!(
        !out.contains("\"bindings\":{\"E\":\"background\"}"),
        "the background is not red and must not be returned: {out}"
    );
}

#[test]
fn the_reverse_lookup_on_a_red_background_names_stop_alone() {
    let dir = scratch("redbg");
    place(&dir);
    let program = case(&dir, "sign_element_color($S, background, red)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"bindings\":{\"S\":\"stop_sign\"}"),
        "the red-backgrounded sign is STOP: {out}"
    );
    // All other signs in the table, not a hand-picked subset.
    for other in ["yield_sign", "warning_sign"] {
        assert!(
            !out.contains(&format!("\"S\":\"{other}\"")),
            "only STOP has a red background, but {other} was returned: {out}"
        );
    }
}

#[test]
fn each_sentence_is_attributed_to_the_chapter_it_came_from() {
    // The rows point at TWO different chapters, and a bare scan for one
    // locator would survive swapping them all. Each sign's answer is pinned
    // to its own sentence-and-chapter PAIR, and must carry neither other
    // sign's sentence nor the envelope. The YIELD pair is byte-exact
    // including "(see Figure 2B-1)", the parenthesis a tag-stripping space
    // once corrupted in the sibling sign-shape library.
    for (sign, sentence, locator) in [
        ("stop_sign", STOP, PART2B),
        ("yield_sign", YIELD, PART2B),
        ("warning_sign", WARNING, PART2C),
    ] {
        let dir = scratch(&format!("locators_{sign}"));
        place(&dir);
        let program = case(&dir, &format!("sign_element_color({sign}, background, $C)"));
        let (ok, out) = run(&program);
        assert!(ok, "cli should succeed: {out}");
        assert!(
            out.contains(&citation(sentence, locator)),
            "{sign} carries its own sentence at its own chapter: {out}"
        );
        for other in [STOP, YIELD, WARNING] {
            if other != sentence {
                assert!(!out.contains(other), "{sign} must not carry another sign's sentence: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {sign}: {out}");
        assert!(!out.contains(PART2A), "no {sign} answer cites Chapter 2A: {out}");
    }
}

#[test]
fn every_row_answer_carries_its_own_signs_sentence() {
    for (sign, element, color, sentence, locator) in ROWS {
        let dir = scratch(&format!("row_{sign}_{element}"));
        place(&dir);
        let program = case(&dir, &format!("sign_element_color({sign}, {element}, $C)"));
        let (ok, out) = run(&program);
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {sign} {element}: {out}");
        assert!(
            out.contains(&format!("\"bindings\":{{\"C\":\"{color}\"}}")),
            "{sign}'s {element} is {color}: {out}"
        );
        assert!(
            out.contains(&citation(sentence, locator)),
            "{sign} {element}: its sign's sentence at its chapter, whole: {out}"
        );
    }
}

#[test]
fn the_compressed_legend_and_border_phrases_bind_both_parts() {
    // The STOP and warning sentences use a COMPRESSED construction -- "a
    // white legend and border", "a black legend and border" -- where one
    // colour distributes over two nouns. Those are the easiest rows to
    // misread, and review showed four of the nine rows had no assertion at
    // all: mutating STOP's legend to black passed the entire suite. These
    // pin both halves of each compressed phrase.
    let dir = scratch("compressed");
    place(&dir);
    let program = case(&dir, "sign_element_color(stop_sign, $E, white)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"bindings\":{\"E\":\"legend\"}")
            && out.contains("\"bindings\":{\"E\":\"border\"}"),
        "STOP's white distributes over BOTH the legend and the border: {out}"
    );
    assert!(
        !out.contains("\"bindings\":{\"E\":\"background\"}"),
        "STOP's background is red, so it must not come back as white: {out}"
    );

    let dir = scratch("compressed2");
    place(&dir);
    let program = case(&dir, "sign_element_color(warning_sign, $E, black)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"bindings\":{\"E\":\"legend\"}")
            && out.contains("\"bindings\":{\"E\":\"border\"}"),
        "the warning sign's black distributes over BOTH the legend and the border: {out}"
    );
    assert!(
        !out.contains("\"bindings\":{\"E\":\"background\"}"),
        "the warning background is yellow, so it must not come back as black: {out}"
    );
}

#[test]
fn signs_whose_sentences_state_no_colour_abstain() {
    let dir = scratch("nocolour");
    place(&dir);
    // Variable form deliberately: a fully-bound query that matches nothing
    // produces NO recall entry at all rather than an abstention.
    let program = case(&dir, "sign_element_color(no_passing_zone_sign, $E, $C)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // Its sentence specifies a pennant shape and says nothing about
    // colour. Inferring "warning signs are yellow, so this is yellow"
    // would be reasoning presented as recall -- and the warning sentence
    // is itself defeasible, so the inference is not even sound.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a sign whose sentence states no colour has none here: {out}"
    );

    let dir = scratch("nocolour2");
    place(&dir);
    let program = case(&dir, "sign_element_color(regulatory_sign, $E, $C)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The rectangular default says nothing about colour, and STOP and
    // YIELD prove regulatory signs share no single scheme anyway.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "the regulatory class has no colour scheme here: {out}"
    );

    // POSITIVE CONTROL: a sign whose sentence DOES state colours still
    // binds, so neither abstention can pass against a library that answers
    // nothing at all.
    let dir = scratch("nocolour_control");
    place(&dir);
    let program = case(&dir, "sign_element_color(warning_sign, background, $C)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"bindings\":{\"C\":\"yellow\"}"),
        "control: a sign with stated colours still binds: {out}"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (sign, element, color, sentence, locator) in ROWS {
        let expected = format!(
            "    row ({sign}, {element}, {color}) {{\n        source \"{sentence}\"\n        locator \"{locator}\"\n    }}"
        );
        assert!(body.contains(&expected), "row ({sign}, {element}, {color}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 9, "nine row sources");
    assert_eq!(body.matches("\n        locator \"").count(), 9, "nine row locators");
    assert!(!body.contains("\n        trust "), "no row restates trust");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(!body.contains("\n        cites "), "no row corroboration");
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{PART2A}\"\n    trust authoritative\n")),
        "the envelope is Chapter 2A's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{STOP}\"")), "not the STOP sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["stop", "yield", "warning", "red ", "white", "yellow", "black", "legend", "border", "background"] {
        assert!(!folded.contains(word), "the envelope must name no sign, colour or element, but contains {word:?}");
    }
}
