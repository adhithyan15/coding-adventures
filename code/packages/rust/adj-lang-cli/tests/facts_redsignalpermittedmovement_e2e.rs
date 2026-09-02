//! End-to-end test for the transportation FACTS library
//! (`adj-facts-stdlib/transportation/red-signal-permitted-movement.adj`)
//! driven through the built CLI: a one-column `table` recording which
//! movements a steady CIRCULAR RED signal permits, grounding the FHWA's
//! Manual on Uniform Traffic Control Devices.
//!
//! READ THIS TABLE NEXT TO ITS GREEN SIBLING. Green permits `turn_right`
//! OUTRIGHT; red permits only `turn_right_after_stopping`. Because the
//! qualifier is part of the value, the two relations share NO movement
//! atom, and `the_two_signals_share_no_movement_atom` asserts exactly that
//! -- a driver who reads the tables as saying the same thing about right
//! turns has made the mistake these libraries exist to prevent.
//!
//! ALL QUERIES ARE IN VARIABLE FORM, and that is forced rather than
//! stylistic: a fully-ground query on a one-column table is routed to the
//! hypothesis/ranking path and produces NO `recall` entry at all, so it can
//! neither answer nor abstain. Membership must be read from the bound set.
//!
//! Every assertion uses the JOINT binding form, the negative needles carry
//! their closing quote so a hedged atom cannot satisfy them by prefix, and
//! EVERY ROW is pinned -- the gap review found in the sibling colour
//! library, where four of nine rows had no assertion at all.
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsredsignal_{tag}_{}", std::process::id()));
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

/// Place the red library alone.
fn place(dir: &Path) {
    let src = facts_stdlib().join("transportation/red-signal-permitted-movement.adj");
    std::fs::copy(&src, dir.join("red-signal-permitted-movement.adj"))
        .expect("copy shipped red-signal-permitted-movement.adj");
}

/// Place the red library AND the green sibling it contrasts with.
fn place_both(dir: &Path) {
    place(dir);
    let src = facts_stdlib().join("transportation/green-signal-permitted-movement.adj");
    std::fs::copy(&src, dir.join("green-signal-permitted-movement.adj"))
        .expect("copy shipped green-signal-permitted-movement.adj");
}

fn case(dir: &Path, imports: &[&str], query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    let mut program = String::new();
    for i in imports {
        program.push_str(&format!("import \"{i}\"\n"));
    }
    program.push_str(&format!("? {query}\n"));
    std::fs::write(&path, program).unwrap();
    path
}

#[test]
fn a_red_signal_permits_exactly_two_movements_both_after_stopping() {
    let dir = scratch("both");
    place(&dir);
    let program = case(
        &dir,
        &["red-signal-permitted-movement.adj"],
        "red_signal_permitted_movement($M)",
    );

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle elsewhere in this
    // file matched only part of the sentence, which let the citation be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once. See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"Except when a traffic control device is in place prohibiting a turn on red or a steady RED ARROW signal indication is displayed, vehicular traffic facing a steady CIRCULAR RED signal indication is permitted to enter the intersection to turn right, or to turn left from a one-way street into a one-way street, after stopping.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // EVERY row pinned, not a sample.
    assert!(
        out.contains("\"bindings\":{\"M\":\"turn_right_after_stopping\"}"),
        "right on red is permitted, after stopping: {out}"
    );
    assert!(
        out.contains(
            "\"bindings\":{\"M\":\"turn_left_from_one_way_street_into_one_way_street_after_stopping\"}"
        ),
        "the narrow one-way-to-one-way left is permitted, after stopping: {out}"
    );
    // CARDINALITY, not just membership. Pinning both rows proves nothing is
    // MISSING; it cannot prove nothing was INVENTED. Review showed that
    // adding a third row -- `turn_right_on_red_arrow`, `reverse`, even a
    // duplicate -- survived the entire suite, because no assertion counted
    // the answers. That is the exact failure mode a fabricating generator
    // produces, and it is the mirror image of the row-coverage lesson from
    // the sibling colour library.
    // Counted on "citations":[ rather than on the binding needle: each
    // ANSWER carries exactly one citations array, whereas the binding
    // pattern also appears inside the governing section keyed by the
    // instantiated goal, which would make the count 4 and the assertion
    // opaque. Verified to track an invented row (2 -> 3).
    assert_eq!(
        out.matches("\"citations\":[").count(),
        2,
        "red permits EXACTLY two movements -- no invented third row: {out}"
    );
    assert!(
        out.contains("Except when a traffic control device is in place prohibiting a turn on red"),
        "the citation carries the exception clause it is used to justify: {out}"
    );
    assert!(
        out.contains("mutcd.fhwa.dot.gov/htm/2009/part4/part4d.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the MUTCD citation: {out}"
    );
}

#[test]
fn the_unqualified_movements_are_not_permitted_on_red() {
    let dir = scratch("unqualified");
    place(&dir);
    let program = case(
        &dir,
        &["red-signal-permitted-movement.adj"],
        "red_signal_permitted_movement($M)",
    );

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // THE ASSERTION THAT CARRIES THE LIBRARY. Right-on-red is never
    // permitted WITHOUT stopping, so a bare `turn_right` is a claim this
    // source does not make. The needle carries its closing quote, so the
    // legitimate `turn_right_after_stopping` cannot satisfy it by prefix --
    // the real value has `_` where the needle has `"`.
    assert!(
        !out.contains("\"M\":\"turn_right\""),
        "an unqualified right turn must not be permitted on red: {out}"
    );
    assert!(
        !out.contains("\"M\":\"turn_left\""),
        "an unqualified left turn must not be permitted on red -- the permission is only \
         from a one-way street into a one-way street: {out}"
    );
    // Neither of these is permitted on red at all.
    assert!(
        !out.contains("\"M\":\"straight_through\"") && !out.contains("\"M\":\"u_turn\""),
        "proceeding straight or making a U-turn is not permitted on red: {out}"
    );
}

#[test]
fn the_two_signals_share_no_movement_atom() {
    let dir = scratch("contrast");
    place_both(&dir);

    let program = case(
        &dir,
        &["red-signal-permitted-movement.adj", "green-signal-permitted-movement.adj"],
        "green_signal_permitted_movement($M)",
    );
    let (ok, green) = run(&program);
    assert!(ok, "cli should succeed: {green}");
    // Green's right turn is UNQUALIFIED, which is precisely what red's is
    // not. This is the positive control for the negative above: it proves
    // the bare atom exists in the vocabulary and is simply absent from red.
    assert!(
        green.contains("\"bindings\":{\"M\":\"turn_right\"}"),
        "green permits an unqualified right turn: {green}"
    );
    assert!(
        green.contains("\"bindings\":{\"M\":\"straight_through\"}")
            && green.contains("\"bindings\":{\"M\":\"u_turn\"}"),
        "green permits proceeding straight and a U-turn: {green}"
    );
    // And green does NOT carry red's hedged atoms.
    assert!(
        !green.contains("\"M\":\"turn_right_after_stopping\""),
        "green's permission is not conditioned on stopping: {green}"
    );

    let program = case(
        &dir,
        &["red-signal-permitted-movement.adj", "green-signal-permitted-movement.adj"],
        "red_signal_permitted_movement($M)",
    );
    let (ok, red) = run(&program);
    assert!(ok, "cli should succeed: {red}");
    assert!(
        red.contains("\"bindings\":{\"M\":\"turn_right_after_stopping\"}")
            && !red.contains("\"M\":\"turn_right\""),
        "red's permission is conditioned on stopping and carries no bare atom: {red}"
    );
}

#[test]
fn importing_both_tables_keeps_them_separate() {
    let dir = scratch("separate");
    place_both(&dir);
    let program = case(
        &dir,
        &["red-signal-permitted-movement.adj", "green-signal-permitted-movement.adj"],
        "red_signal_permitted_movement($M)",
    );

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // With BOTH libraries loaded, the red relation must still return only
    // red's two movements. If the relations ever merged, green's four
    // movements would leak in and the contrast the pair exists to teach
    // would silently vanish.
    for leaked in ["straight_through", "u_turn"] {
        assert!(
            !out.contains(&format!("\"M\":\"{leaked}\"")),
            "green's {leaked} must not leak into the red relation: {out}"
        );
    }
    assert!(
        out.contains("\"bindings\":{\"M\":\"turn_right_after_stopping\"}"),
        "control: the red relation still answers with both tables loaded: {out}"
    );
}
