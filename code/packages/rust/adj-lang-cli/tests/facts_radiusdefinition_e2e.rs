//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/radius-definition.adj`) driven through the
//! built CLI: a native `table` naming what "radius" measures for a circle
//! vs. a sphere -- a sibling to the already-shipped `circle-parts.adj`
//! (which is keyed by circle PART, not by shape), decoding the second
//! clause of a sentence that table's own `source` field already quotes in
//! full. Resolves binding-query recall (both directions) with the source's
//! citation, and abstains on a solid (cube) the cited sentence does not
//! name -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_radiusdefinition_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geometry/radius-definition.adj");
    std::fs::copy(&src, dir.join("radius-definition.adj"))
        .expect("copy shipped radius-definition.adj");
}

#[test]
fn radius_definition_recalls_both_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"radius-definition.adj\"\n\
         ? radius_definition(circle, $D)\n\
         ? radius_definition(sphere, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"radius_definition(circle, center_to_perimeter)\""),
        "circle's radius definition is center_to_perimeter: {out}"
    );
    assert!(
        out.contains("\"term\":\"radius_definition(sphere, center_to_surface)\""),
        "sphere's radius definition is center_to_surface: {out}"
    );
    assert!(
        out.contains("mathworld.wolfram.com") && out.contains("\"trust\":\"authoritative\""),
        "carries the MathWorld citation: {out}"
    );
}

#[test]
fn radius_definition_recalls_backward_from_a_bound_description() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"radius-definition.adj\"\n\
         ? radius_definition($Shape, center_to_perimeter)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"radius_definition(circle, center_to_perimeter)\""),
        "center_to_perimeter names circle: {out}"
    );
}

#[test]
fn radius_definition_abstains_honestly_on_cube() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"radius-definition.adj\"\n\
         ? radius_definition(cube, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "cube is not one of the two shapes the cited sentence names -- honest abstention: {out}"
    );
}
