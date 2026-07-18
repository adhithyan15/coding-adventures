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
