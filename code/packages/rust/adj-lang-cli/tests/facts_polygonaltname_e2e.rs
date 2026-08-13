//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/polygon-alt-name.adj`) driven through the
//! built CLI: a native `table` naming the alternate word MathWorld's
//! source states for a triangle -- a sibling to the already-shipped
//! `shapes.adj` (which only carries each polygon's side COUNT, not an
//! alternate name), decoding the parenthetical half of a span already
//! sitting unused inside that table's own `source` field. Resolves
//! binding-query recall (both directions) with the source's citation, and
//! abstains on a polygon (quadrilateral) the cited span gives no
//! alternate name for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_polygonaltname_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geometry/polygon-alt-name.adj");
    std::fs::copy(&src, dir.join("polygon-alt-name.adj"))
        .expect("copy shipped polygon-alt-name.adj");
}

#[test]
fn polygon_alt_name_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"polygon-alt-name.adj\"\n\
         ? polygon_alt_name(triangle, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"polygon_alt_name(triangle, trigon)\""),
        "triangle's alt name is trigon: {out}"
    );
    assert!(
        out.contains("mathworld.wolfram.com") && out.contains("\"trust\":\"authoritative\""),
        "carries the MathWorld citation: {out}"
    );
}

#[test]
fn polygon_alt_name_recalls_backward_from_a_bound_alt_name() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"polygon-alt-name.adj\"\n\
         ? polygon_alt_name($Shape, trigon)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"polygon_alt_name(triangle, trigon)\""),
        "trigon names triangle: {out}"
    );
}

#[test]
fn polygon_alt_name_abstains_honestly_on_quadrilateral() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"polygon-alt-name.adj\"\n\
         ? polygon_alt_name(quadrilateral, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "quadrilateral has no alternate name in the cited span -- honest abstention: {out}"
    );
}
