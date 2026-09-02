//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/quadrilateral-types.adj`) driven through the
//! built CLI: a native `table` of five common quadrilaterals → the ONE defining
//! property each one's source states resolves binding-query recalls (forward
//! AND backward) with the source's Wolfram MathWorld citation, and abstains on a
//! word that is not one of the five quadrilaterals (a triangle) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsq_{tag}_{}", std::process::id()));
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
fn geometry_quadrilateral_types_recall_binds_property_with_citation() {
    let dir = scratch("quadrilateraltypes");
    // Copy the shipped geometry table beside the entry program and import it.
    let src = facts_stdlib().join("geometry/quadrilateral-types.adj");
    std::fs::copy(&src, dir.join("quadrilateral-types.adj"))
        .expect("copy shipped quadrilateral-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"quadrilateral-types.adj\"\n\
         ? quadrilateral_property(rhombus, $Property)\n\
         ? quadrilateral_property(parallelogram, $Property)\n\
         ? quadrilateral_property(trapezoid, $Property)\n\
         ? quadrilateral_property($Shape, four_right_angles)\n\
         ? quadrilateral_property(triangle, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THE WHOLE CITATION, anchored on its JSON key and closed by the
    // terminating quote. This sentence carries a qualifier, so a
    // truncation would silently drop meaning -- the defect issue #13916
    // shipped. Pinning a fragment narrows that hole rather than closing
    // it, because `contains` on a fragment cannot see what precedes or
    // follows it. See issue #13918.
    assert!(
        out.contains("\"source\":\"The term 'square' can be used to mean either a square number or a geometric figure consisting of a convex quadrilateral with sides of equal length that are positioned at right angles to each other as illustrated above.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A rhombus has all sides the same length, a parallelogram has opposite
    // sides parallel, a trapezoid has two sides parallel — the recalled
    // properties (forward binds).
    assert!(
        out.contains("\"Property\":\"all_sides_same_length\""),
        "rhombus → all_sides_same_length: {out}"
    );
    assert!(
        out.contains("\"Property\":\"opposite_sides_parallel\""),
        "parallelogram → opposite_sides_parallel: {out}"
    );
    assert!(
        out.contains("\"Property\":\"two_sides_parallel\""),
        "trapezoid → two_sides_parallel: {out}"
    );
    // The relation runs BACKWARD: bind the property `four_right_angles`, recall
    // its quadrilateral.
    assert!(
        out.contains("\"Shape\":\"rectangle\""),
        "four_right_angles → rectangle (reverse recall): {out}"
    );
    // The answer carries the Wolfram MathWorld citation as its proof, at the
    // `authoritative` trust tier for a primary mathematics reference.
    assert!(
        out.contains("mathworld.wolfram.com") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A triangle has three sides, not four — it is not a quadrilateral, so a
    // recall abstains honestly, never a fabricated property.
    assert!(out.contains("\"abstained\":true"), "triangle abstains: {out}");
}

const RECT_PIN: &str = r#""bindings":{"Shape":"rectangle"},"citations":[{"source":"The term 'square' can be used to mean either a square number or a geometric figure consisting of a convex quadrilateral with sides of equal length that are positioned at right angles to each other as illustrated above.","locator":"https://mathworld.wolfram.com/Square.html","trust":"authoritative","corroborations":[{"source":"A rectangle is a closed planar quadrilateral with opposite sides of equal lengths a and b, and with four right angles.","locator":"https://mathworld.wolfram.com/Rectangle.html""#;

const TRAP_PIN: &str = r#""bindings":{"Property":"two_sides_parallel"},"citations":[{"source":"The term 'square' can be used to mean either a square number or a geometric figure consisting of a convex quadrilateral with sides of equal length that are positioned at right angles to each other as illustrated above.","locator":"https://mathworld.wolfram.com/Square.html","trust":"authoritative","corroborations":[{"source":"A rectangle is a closed planar quadrilateral with opposite sides of equal lengths a and b, and with four right angles.","locator":"https://mathworld.wolfram.com/Rectangle.html"},{"source":"A rhombus is a quadrilateral with both pairs of opposite sides parallel and all sides the same length, i.e., an equilateral parallelogram.","locator":"https://mathworld.wolfram.com/Rhombus.html"},{"source":"A parallelogram is a quadrilateral with opposite sides parallel (and therefore opposite angles equal).","locator":"https://mathworld.wolfram.com/Parallelogram.html"},{"source":"A trapezoid is a quadrilateral with two sides parallel.","locator":"https://mathworld.wolfram.com/Trapezoid.html""#;

#[test]
fn quadrilateral_rectangle_answer_carries_its_mathworld_corroboration_intact() {
    let dir = scratch("cite_rect");
    std::fs::copy(
        facts_stdlib().join("geometry/quadrilateral-types.adj"),
        dir.join("quadrilateral-types.adj"),
    )
    .expect("copy shipped quadrilateral-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"quadrilateral-types.adj\"\n? quadrilateral_property($Shape, four_right_angles)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // ANCHORED and JOINT: bindings + envelope + corroboration in ONE span,
    // ending on a closing quote.
    //
    // The "lengths a and b" here come from MathWorld's inline formulas, which
    // are `<img class="inlineformula" alt="a">` -- 9px images OF THOSE
    // LETTERS. Recovering the alt reproduces what the page displays. That is
    // NOT true of circle-parts' diameter, whose `<img alt="pi">` renders a pi
    // SYMBOL: there the alt NAMES the glyph rather than being it, so that row
    // is deliberately left uncited. Pinning the recovered form here is what
    // keeps the two cases from being collapsed back together.
    assert!(
        out.contains(RECT_PIN),
        "rectangle's answer carries the MathWorld Rectangle sentence verbatim: {out}"
    );
}

#[test]
fn quadrilateral_trapezoid_answer_carries_all_four_corroborations_in_order() {
    let dir = scratch("cite_trap");
    std::fs::copy(
        facts_stdlib().join("geometry/quadrilateral-types.adj"),
        dir.join("quadrilateral-types.adj"),
    )
    .expect("copy shipped quadrilateral-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"quadrilateral-types.adj\"\n? quadrilateral_property(trapezoid, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Spans the WHOLE corroboration list, so a reordering or a dropped middle
    // entry fails here even though each individual sentence would still be
    // present somewhere in the blob.
    assert!(
        out.contains(TRAP_PIN),
        "trapezoid's answer carries all four MathWorld sentences in order: {out}"
    );
}
