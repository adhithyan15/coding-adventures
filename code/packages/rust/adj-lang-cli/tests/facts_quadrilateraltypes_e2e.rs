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
