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
//! Every assertion uses the JOINT binding form rather than independent
//! substring scans, both abstention tests carry positive controls, and the
//! two-locator property is pinned by sentence-to-locator PAIRS -- all three
//! being defects review caught in earlier libraries of this series.
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

#[test]
fn a_stop_signs_background_is_red() {
    let dir = scratch("stopbg");
    place(&dir);
    let program = case(&dir, "sign_element_color(stop_sign, background, $C)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"bindings\":{\"C\":\"red\"}"),
        "the STOP sign's background is red: {out}"
    );
    assert!(
        out.contains("The STOP sign shall be an octagon with a white legend and border on a red background."),
        "carries the grounding sentence verbatim: {out}"
    );
    assert!(
        out.contains("\"trust\":\"authoritative\""),
        "carries the MUTCD trust tier: {out}"
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
    let dir = scratch("locators");
    place(&dir);
    let program = case(&dir, "sign_element_color(stop_sign, background, $C)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The corroborations point at TWO different chapters. A bare scan for
    // one locator would survive swapping them all, so these pin
    // sentence-to-locator PAIRS instead -- the gap review found in the
    // sibling sign-shape library.
    assert!(
        out.contains(
            "\"source\":\"Except as provided in Paragraph 2 or unless specifically designated \
             otherwise, all warning signs shall be diamond-shaped (square with one diagonal \
             vertical) with a black legend and border on a yellow background.\",\
             \"locator\":\"https://mutcd.fhwa.dot.gov/htm/2009/part2/part2c.htm\""
        ),
        "the warning sentence is attributed to Chapter 2C: {out}"
    );
    assert!(
        out.contains(
            "\"source\":\"The STOP sign shall be an octagon with a white legend and border on a \
             red background.\",\"locator\":\"https://mutcd.fhwa.dot.gov/htm/2009/part2/part2b.htm\""
        ),
        "the STOP sentence is attributed to Chapter 2B: {out}"
    );
    // THE YIELD PAIR IS NOT OPTIONAL, and review proved why. Without it,
    // two separate regressions pass the whole suite: retargeting YIELD's
    // locator to Chapter 2C, and -- worse -- reinserting "(see Figure
    // 2B-1 )" with the spurious tag-stripping space, which is the EXACT
    // defect the sibling sign-shape library shipped in a draft one slice
    // ago. Pinning the sentence together with its locator closes both,
    // because the needle is byte-exact including the parenthesis.
    assert!(
        out.contains(
            "\"source\":\"The YIELD (R1-2) sign (see Figure 2B-1) shall be a downward-pointing \
             equilateral triangle with a wide red border and the legend YIELD in red on a white \
             background.\",\"locator\":\"https://mutcd.fhwa.dot.gov/htm/2009/part2/part2b.htm\""
        ),
        "the YIELD sentence is attributed to Chapter 2B, byte-exact: {out}"
    );
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
