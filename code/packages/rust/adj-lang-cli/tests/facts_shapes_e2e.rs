//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/shapes.adj`) driven through the built CLI:
//! a native `table` of polygon → number-of-sides resolves a binding-query recall
//! with the source's citation, and abstains on a non-polygon — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn geometry_shapes_recall_binds_side_count_with_citation() {
    let dir = scratch("shapes");
    // Copy the shipped geometry table beside the entry program and import it.
    let src = facts_stdlib().join("geometry/shapes.adj");
    std::fs::copy(&src, dir.join("shapes.adj")).expect("copy shipped shapes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"shapes.adj\"\n\
         ? polygon_sides(hexagon, $Sides)\n\
         ? polygon_sides(triangle, $Sides)\n\
         ? polygon_sides(circle, $Sides)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A hexagon has six sides; a triangle has three — the recalled counts.
    assert!(out.contains("\"Sides\":\"6\""), "hexagon → 6: {out}");
    assert!(out.contains("\"Sides\":\"3\""), "triangle → 3: {out}");
    // The answer carries the MathWorld citation as its proof.
    assert!(
        out.contains("mathworld.wolfram.com") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A circle is not a polygon — honest abstention, never a fabricated count.
    assert!(out.contains("\"abstained\":true"), "circle abstains: {out}");
}
