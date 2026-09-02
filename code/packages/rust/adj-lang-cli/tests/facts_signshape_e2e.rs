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

#[test]
fn a_stop_sign_is_an_octagon() {
    let dir = scratch("stop");
    place(&dir);
    let program = case(&dir, "sign_shape(stop_sign, $S)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle elsewhere in this
    // file matched only part of the sentence, which let the citation be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once. See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"Regulatory signs shall be rectangular unless specifically designated otherwise.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"bindings\":{\"S\":\"octagon\"}"),
        "the STOP sign is an octagon: {out}"
    );
    assert!(
        out.contains("The STOP sign shall be an octagon with a white legend and border on a red background."),
        "carries the grounding sentence verbatim: {out}"
    );
    assert!(
        out.contains("mutcd.fhwa.dot.gov/htm/2009/part2/part2b.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the MUTCD citation: {out}"
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
    let dir = scratch("locators");
    place(&dir);
    let program = case(&dir, "sign_shape(stop_sign, $S)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // THE STRUCTURAL NOVELTY OF THIS LIBRARY IS THAT ITS CORROBORATIONS
    // POINT AT TWO DIFFERENT PAGES, and that property needs a joint
    // assertion rather than a bare locator scan. Asserting only that
    // "part2b.htm" appears somewhere would survive swapping every locator,
    // because part2b would still be present -- and part2c would never be
    // checked at all. These pin sentence-to-locator pairs instead.
    assert!(
        out.contains(
            "\"source\":\"The NO PASSING ZONE (W14-3) sign (see Figure 2C-8) shall be a \
             pennant-shaped isosceles triangle with its longer axis horizontal and pointing to \
             the right.\",\"locator\":\"https://mutcd.fhwa.dot.gov/htm/2009/part2/part2c.htm\""
        ),
        "the pennant sentence is attributed to Chapter 2C: {out}"
    );
    assert!(
        out.contains(
            "\"source\":\"The STOP sign shall be an octagon with a white legend and border on a \
             red background.\",\"locator\":\"https://mutcd.fhwa.dot.gov/htm/2009/part2/part2b.htm\""
        ),
        "the STOP sentence is attributed to Chapter 2B: {out}"
    );
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
    // no such thing.
    assert!(
        out.contains("Except as provided in Paragraph 2 or unless specifically designated otherwise, all warning signs shall be diamond-shaped"),
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
