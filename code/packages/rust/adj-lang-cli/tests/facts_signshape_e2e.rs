//! End-to-end test for the transportation FACTS library
//! (`adj-facts-stdlib/transportation/sign-shape.adj`) driven through the
//! built CLI: a native `table` recording the shape the federal standard
//! specifies for a traffic sign, grounding the FHWA's Manual on Uniform
//! Traffic Control Devices.
//!
//! THIS TABLE HOLDS A DEFAULT AND ITS OWN COUNTEREXAMPLES, which is what
//! makes the hedges load-bearing rather than decorative. The MUTCD says
//! regulatory signs "shall be rectangular unless specifically designated
//! otherwise" -- and then designates otherwise for STOP (an octagon) and
//! YIELD (a downward-pointing equilateral triangle), both of which ARE
//! regulatory signs. Likewise NO PASSING ZONE is a warning sign that is
//! not a diamond.
//!
//! Recorded flatly, those rows would contradict each other. They do not,
//! because each default carries its defeasibility inside its own atom. The
//! bare-shape abstentions below are what prove the hedge has not been
//! quietly dropped -- and unlike earlier libraries in this stdlib, where a
//! dropped hedge would have cost a shade of confidence, here it would make
//! the library assert something the same document refutes two sections
//! later.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to
//! be the regulatory default, so the STOP answer cited "Regulatory signs
//! shall be rectangular" first. The envelope is now Chapter 2A's framing
//! sentence, which every row overrides, and each row's citation is pinned
//! whole.
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
    let dir = std::env::temp_dir().join(format!("adjcli_factssignshape_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("transportation/sign-shape.adj");
    std::fs::copy(&src, dir.join("sign-shape.adj")).expect("copy shipped sign-shape.adj");
}

fn case(dir: &Path, query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    std::fs::write(&path, format!("import \"sign-shape.adj\"\n? {query}\n")).unwrap();
    path
}

const PART2A: &str = "https://mutcd.fhwa.dot.gov/htm/2009/part2/part2a.htm";
const PART2B: &str = "https://mutcd.fhwa.dot.gov/htm/2009/part2/part2b.htm";
const PART2C: &str = "https://mutcd.fhwa.dot.gov/htm/2009/part2/part2c.htm";

const ENVELOPE: &str = "Standardized colors and shapes are specified so that the several classes of traffic signs can be promptly recognized.";
const REGULATORY: &str = "Regulatory signs shall be rectangular unless specifically designated otherwise.";
const STOP: &str = "The STOP sign shall be an octagon with a white legend and border on a red background.";
const YIELD: &str = "The YIELD (R1-2) sign (see Figure 2B-1) shall be a downward-pointing equilateral triangle with a wide red border and the legend YIELD in red on a white background.";
const WARNING: &str = "Except as provided in Paragraph 2 or unless specifically designated otherwise, all warning signs shall be diamond-shaped (square with one diagonal vertical) with a black legend and border on a yellow background.";
const NO_PASSING_ZONE: &str = "The NO PASSING ZONE (W14-3) sign (see Figure 2C-8) shall be a pennant-shaped isosceles triangle with its longer axis horizontal and pointing to the right.";

/// (sign, shape, its own sentence, the chapter that sentence was measured on)
const ROWS: [(&str, &str, &str, &str); 5] = [
    ("regulatory_sign", "rectangular_unless_specifically_designated_otherwise", REGULATORY, PART2B),
    ("stop_sign", "octagon", STOP, PART2B),
    ("yield_sign", "downward_pointing_equilateral_triangle", YIELD, PART2B),
    ("warning_sign", "diamond_shaped_unless_specifically_designated_otherwise", WARNING, PART2C),
    ("no_passing_zone_sign", "pennant_shaped_isosceles_triangle", NO_PASSING_ZONE, PART2C),
];

/// The whole citation a row's answer carries, as one contiguous run.
fn citation(sentence: &str, locator: &str) -> String {
    format!("\"source\":\"{sentence}\",\"locator\":\"{locator}\",\"trust\":\"authoritative\",\"corroborations\":[]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("transportation/sign-shape.adj"))
        .expect("read shipped sign-shape.adj");
    adj[adj.find("table sign_shape").expect("table")..].to_string()
}

#[test]
fn a_stop_sign_is_an_octagon() {
    let dir = scratch("stop");
    place(&dir);
    let program = case(&dir, "sign_shape(stop_sign, $S)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN, now the whole contiguous run. This used
    // to pin the REGULATORY DEFAULT as the STOP answer's primary source -- a
    // sentence saying regulatory signs are rectangular, grounding an
    // octagon. See issues #13916, #13918 and #14986.
    assert!(
        out.contains(&citation(STOP, PART2B)),
        "the citation is the STOP sentence at Chapter 2B, whole: {out}"
    );
    assert!(!out.contains(REGULATORY), "the regulatory default does not ground STOP: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"bindings\":{\"S\":\"octagon\"}"),
        "the STOP sign is an octagon: {out}"
    );
}

#[test]
fn the_reverse_lookup_names_the_eight_sided_sign() {
    let dir = scratch("reverse");
    place(&dir);
    let program = case(&dir, "sign_shape($Sign, octagon)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The question a child actually asks, and the direction the confusion
    // runs in.
    assert!(
        out.contains("\"bindings\":{\"Sign\":\"stop_sign\"}"),
        "the eight-sided sign is the STOP sign: {out}"
    );
    // ALL FOUR other signs, not a hand-picked two. An earlier version
    // excluded only yield_sign and regulatory_sign, so adding
    // `row (warning_sign, octagon)` passed every test while the message
    // claimed "no other sign may be returned as an octagon" -- the message
    // asserting more than the check.
    for other in [
        "yield_sign",
        "regulatory_sign",
        "warning_sign",
        "no_passing_zone_sign",
    ] {
        assert!(
            !out.contains(&format!("\"Sign\":\"{other}\"")),
            "no other sign may be returned as an octagon, but {other} was: {out}"
        );
    }
}

#[test]
fn each_sentence_is_attributed_to_the_chapter_it_came_from() {
    // THE ROWS POINT AT TWO DIFFERENT CHAPTERS, and that property needs a
    // joint assertion rather than a bare locator scan: asserting only that
    // "part2b.htm" appears somewhere would survive swapping every locator.
    // Each row's answer is pinned to its own sentence-and-chapter pair, and
    // must carry no other row's sentence and not the envelope.
    for (sign, shape, sentence, locator) in ROWS {
        let dir = scratch(&format!("locators_{sign}"));
        place(&dir);
        let program = case(&dir, &format!("sign_shape({sign}, $S)"));
        let (ok, out) = run(&program);
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {sign}: {out}");
        assert!(
            out.contains(&format!("\"bindings\":{{\"S\":\"{shape}\"}}")),
            "{sign} is {shape}: {out}"
        );
        assert!(
            out.contains(&citation(sentence, locator)),
            "{sign} carries its own sentence at its own chapter, whole: {out}"
        );
        for (other, _, other_sentence, _) in ROWS {
            if other != sign {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {sign}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {sign}: {out}");
    }
}

#[test]
fn both_defaults_keep_their_defeasibility_in_the_atom() {
    let dir = scratch("hedged");
    place(&dir);
    let program = case(&dir, "sign_shape(regulatory_sign, $S)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains(
            "\"bindings\":{\"S\":\"rectangular_unless_specifically_designated_otherwise\"}"
        ),
        "the regulatory default carries its own defeasibility: {out}"
    );

    let dir = scratch("hedged2");
    place(&dir);
    let program = case(&dir, "sign_shape(warning_sign, $S)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains(
            "\"bindings\":{\"S\":\"diamond_shaped_unless_specifically_designated_otherwise\"}"
        ),
        "the warning default carries its own defeasibility: {out}"
    );
    // THE CITATION MUST CARRY THE QUALIFIER IT JUSTIFIES. The manual's
    // sentence OPENS with its exception clause; quoting from "all warning
    // signs" would have been tidier and would have left an atom saying
    // "unless designated otherwise" backed by a quotation appearing to say
    // no such thing. The warning row's own source now carries it, whole.
    assert!(
        out.contains(&citation(WARNING, PART2C)),
        "the quoted evidence includes the exception clause it is used to justify: {out}"
    );
}

#[test]
fn the_unqualified_shape_claims_abstain() {
    let dir = scratch("bare");
    place(&dir);
    // Variable form deliberately: a fully-bound query that matches nothing
    // produces NO recall entry at all rather than an abstention.
    let program = case(&dir, "sign_shape($Sign, rectangular)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // THE POINT. Asking which signs are simply "rectangular" is asking for
    // a claim the MUTCD never makes -- and its own next sections, which
    // give STOP an octagon and YIELD a triangle, are the proof that the
    // unqualified reading is false.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "the unqualified rectangular claim is not stated by this source: {out}"
    );

    let dir = scratch("bare2");
    place(&dir);
    let program = case(&dir, "sign_shape($Sign, diamond_shaped)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "the unqualified diamond claim is not stated either: {out}"
    );

    // POSITIVE CONTROL: an unhedged shape the manual DOES state flatly
    // still binds, so the two abstentions above cannot pass against a
    // library that answers nothing.
    let dir = scratch("bare_control");
    place(&dir);
    let program = case(&dir, "sign_shape($Sign, octagon)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"bindings\":{\"Sign\":\"stop_sign\"}"),
        "control: a flatly-stated shape still binds: {out}"
    );
}

#[test]
fn a_sign_governed_only_by_the_default_abstains() {
    let dir = scratch("speedlimit");
    place(&dir);
    let program = case(&dir, "sign_shape(speed_limit_sign, $S)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // A SPEED LIMIT sign is a regulatory sign, so the rectangular default
    // governs it -- but the default is DEFEASIBLE, and STOP and YIELD are
    // standing proof that inferring a specific sign's shape from it is
    // unsound. Deriving "rectangular" here would be reasoning presented as
    // recall.
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"reason\":\"no_grounded_support\""),
        "a shape must not be inferred from a defeasible default: {out}"
    );

    // Positive control, same reason as above.
    let dir = scratch("speedlimit_control");
    place(&dir);
    let program = case(&dir, "sign_shape(no_passing_zone_sign, $S)");
    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"bindings\":{\"S\":\"pennant_shaped_isosceles_triangle\"}"),
        "control: a named sign with a stated shape still binds: {out}"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (sign, shape, sentence, locator) in ROWS {
        let expected = format!(
            "    row ({sign}, {shape}) {{\n        source \"{sentence}\"\n        locator \"{locator}\"\n    }}"
        );
        assert!(body.contains(&expected), "row ({sign}, {shape}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 5, "five row sources");
    assert_eq!(body.matches("\n        locator \"").count(), 5, "five row locators");
    assert!(!body.contains("\n        trust "), "no row restates trust");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(!body.contains("\n        cites "), "no row corroboration");
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{PART2A}\"\n    trust authoritative\n")),
        "the envelope is Chapter 2A's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{REGULATORY}\"\n    locator")), "not the regulatory default as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["stop", "yield", "warning", "regulatory", "octagon", "triangle", "diamond", "pennant", "rectangular"] {
        assert!(!folded.contains(word), "the envelope must name no sign or shape, but contains {word:?}");
    }
}
