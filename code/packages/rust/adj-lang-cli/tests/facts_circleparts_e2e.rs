//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/circle-parts.adj`) driven through the built CLI:
//! a native `table` of circle parts → the defining phrase the source states
//! resolves binding-query recalls (forward AND backward) with the source's
//! Wolfram MathWorld citation, and abstains on a word that is not one of these
//! circle parts (a vertex) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factcp_{tag}_{}", std::process::id()));
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
fn geometry_circle_parts_recall_binds_description_with_citation() {
    let dir = scratch("circleparts");
    // Copy the shipped geometry table beside the entry program and import it.
    let src = facts_stdlib().join("geometry/circle-parts.adj");
    std::fs::copy(&src, dir.join("circle-parts.adj")).expect("copy shipped circle-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"circle-parts.adj\"\n\
         ? circle_part(radius, $D)\n\
         ? circle_part(diameter, $D)\n\
         ? circle_part(circumference, $D)\n\
         ? circle_part(chord, $D)\n\
         ? circle_part($Part, perimeter)\n\
         ? circle_part(vertex, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The radius runs center to perimeter, the diameter is the maximum distance
    // across, the circumference is the perimeter, and a chord's ends lie on the
    // circle — the recalled descriptions (forward binds).
    assert!(
        out.contains("\"D\":\"center_to_perimeter\""),
        "radius → center_to_perimeter: {out}"
    );
    assert!(
        out.contains("\"D\":\"maximum_distance_across\""),
        "diameter → maximum_distance_across: {out}"
    );
    assert!(
        out.contains("\"D\":\"perimeter\""),
        "circumference → perimeter: {out}"
    );
    assert!(
        out.contains("\"D\":\"ends_on_circle\""),
        "chord → ends_on_circle: {out}"
    );
    // The relation runs BACKWARD: bind the description `perimeter`, recall its
    // circle part.
    assert!(
        out.contains("\"Part\":\"circumference\""),
        "perimeter → circumference (reverse recall): {out}"
    );
    // The answer carries the MathWorld citation as its proof, at the
    // `authoritative` trust tier for a primary mathematics reference.
    assert!(
        out.contains("mathworld.wolfram.com") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A vertex is a corner of a polygon, not a part of a circle — honest
    // abstention, never a fabricated description.
    assert!(out.contains("\"abstained\":true"), "vertex abstains: {out}");
}

const CHORD_PIN: &str = r#""bindings":{"D":"ends_on_circle"},"citations":[{"source":"The distance from the center of a circle to its perimeter, or from the center of a sphere to its surface.","locator":"https://mathworld.wolfram.com/Radius.html","trust":"authoritative","corroborations":[{"source":"In plane geometry, a chord is the line segment joining two points on a curve. The term is often used to describe a line segment whose ends lie on a circle.","locator":"https://mathworld.wolfram.com/Chord.html""#;

const CIRC_PIN: &str = r#""bindings":{"Part":"circumference"},"citations":[{"source":"The distance from the center of a circle to its perimeter, or from the center of a sphere to its surface.","locator":"https://mathworld.wolfram.com/Radius.html","trust":"authoritative","corroborations":[{"source":"In plane geometry, a chord is the line segment joining two points on a curve. The term is often used to describe a line segment whose ends lie on a circle.","locator":"https://mathworld.wolfram.com/Chord.html"},{"source":"In the work, the term \"circumference\" is used to mean the perimeter of a circle.","locator":"https://mathworld.wolfram.com/Circumference.html""#;

#[test]
fn circle_part_chord_answer_carries_its_mathworld_corroboration_intact() {
    let dir = scratch("cite_chord");
    std::fs::copy(
        facts_stdlib().join("geometry/circle-parts.adj"),
        dir.join("circle-parts.adj"),
    )
    .expect("copy shipped circle-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"circle-parts.adj\"\n? circle_part(chord, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // ANCHORED and JOINT -- bindings, envelope, and corroboration in one span.
    assert!(
        out.contains(CHORD_PIN),
        "chord's answer carries the MathWorld Chord sentence verbatim: {out}"
    );
}

#[test]
fn circle_part_circumference_corroboration_survives_its_embedded_quotes() {
    let dir = scratch("cite_circ");
    std::fs::copy(
        facts_stdlib().join("geometry/circle-parts.adj"),
        dir.join("circle-parts.adj"),
    )
    .expect("copy shipped circle-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"circle-parts.adj\"\n? circle_part($Part, perimeter)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // MathWorld writes the term in quotation marks, so this is the FIRST
    // string in the stdlib to use the lexer's `\"` escape (STRING is
    // `"([^"\\]|\\.)*"`). The pin covers the whole round trip: escaped in
    // the .adj, a real quote in the value, re-escaped in the JSON.
    //
    // The page emits `&quot;`, which my extraction did not decode -- the
    // sentence came back NOT FOUND and would have been recorded as a false
    // blocker had the negative been trusted.
    assert!(
        out.contains(CIRC_PIN),
        "circumference's corroboration keeps its embedded quotes: {out}"
    );
}
