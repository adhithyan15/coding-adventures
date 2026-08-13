//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/quadrilateral-secondary-property.adj`)
//! driven through the built CLI: a native `table` naming a SECOND
//! defining property MathWorld's source states for a quadrilateral, where
//! the source states two -- a sibling to the already-shipped
//! `quadrilateral-types.adj` (which only carries the FIRST/primary
//! property per shape), decoding spans already sitting unused inside that
//! table's own provenance block. Resolves binding-query recall (both
//! directions) with the source's citation, and abstains on a shape
//! (square) the cited spans give no second property for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_quadsecondaryproperty_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geometry/quadrilateral-secondary-property.adj");
    std::fs::copy(&src, dir.join("quadrilateral-secondary-property.adj"))
        .expect("copy shipped quadrilateral-secondary-property.adj");
}

#[test]
fn quadrilateral_secondary_property_recalls_both_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"quadrilateral-secondary-property.adj\"\n\
         ? quadrilateral_secondary_property(rectangle, $Property)\n\
         ? quadrilateral_secondary_property(parallelogram, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"quadrilateral_secondary_property(rectangle, opposite_sides_equal_length)\""),
        "rectangle's secondary property is opposite_sides_equal_length: {out}"
    );
    assert!(
        out.contains("\"term\":\"quadrilateral_secondary_property(parallelogram, opposite_angles_equal)\""),
        "parallelogram's secondary property is opposite_angles_equal: {out}"
    );
    assert!(
        out.contains("mathworld.wolfram.com") && out.contains("\"trust\":\"authoritative\""),
        "carries the MathWorld citation: {out}"
    );
}

#[test]
fn quadrilateral_secondary_property_recalls_backward_from_a_bound_property() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"quadrilateral-secondary-property.adj\"\n\
         ? quadrilateral_secondary_property($Shape, opposite_angles_equal)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"quadrilateral_secondary_property(parallelogram, opposite_angles_equal)\""),
        "opposite_angles_equal names parallelogram: {out}"
    );
}

#[test]
fn quadrilateral_secondary_property_abstains_honestly_on_square() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"quadrilateral-secondary-property.adj\"\n\
         ? quadrilateral_secondary_property(square, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "square has no second property in the cited spans -- honest abstention: {out}"
    );
}
