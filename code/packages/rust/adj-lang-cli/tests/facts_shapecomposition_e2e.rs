//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/shape-composition.adj`) driven through the
//! built CLI: the FIRST `rule`-derived (not table-only) content in
//! adj-facts-stdlib. A `rule` combines the already-shipped
//! `quadrilateral_property` table with a cited triangulation/diagonal
//! definition to derive `triangle_decomposition_count(shape, 2)` for each
//! named quadrilateral, carrying a full 2-step audit trail (the rule's own
//! citation and the underlying table fact's citation) — and abstains
//! honestly on a shape that isn't a quadrilateral (a triangle has no
//! diagonal to draw).

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "adjcli_shapecomposition_{tag}_{}",
        std::process::id()
    ));
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

fn place_stdlib(dir: &Path) {
    for rel in [
        "geometry/quadrilateral-types.adj",
        "geometry/shape-composition.adj",
    ] {
        let src = facts_stdlib().join(rel);
        std::fs::copy(&src, dir.join(rel.rsplit('/').next().unwrap()))
            .unwrap_or_else(|e| panic!("copy {rel}: {e}"));
    }
}

#[test]
fn triangle_decomposition_count_derives_two_with_full_audit_trail() {
    let dir = scratch("derive");
    place_stdlib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"shape-composition.adj\"\n\
         ? triangle_decomposition_count(square, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"N\":\"2\""),
        "a square decomposes into 2 triangles: {out}"
    );
    // The audit trail names BOTH steps: the rule's own citation (the general
    // triangulation/diagonal definitions) and the underlying quadrilateral
    // fact's citation (the square's own defining property) — an inspectable
    // two-step inference, not a single opaque answer.
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("mathworld.wolfram.com/Triangulation.html"),
        "carries the rule's own citation: {out}"
    );
    assert!(
        out.contains("\"kind\":\"fact\"") && out.contains("mathworld.wolfram.com/Square.html"),
        "carries the underlying quadrilateral fact's citation: {out}"
    );
    assert!(
        out.contains("\"trust\":\"consensus\""),
        "the rule's own derivation is consensus-tier, not authoritative: {out}"
    );
}

#[test]
fn triangle_decomposition_count_runs_backward_over_every_named_quadrilateral() {
    let dir = scratch("backward");
    place_stdlib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"shape-composition.adj\"\n\
         ? triangle_decomposition_count($Shape, 2)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for shape in ["square", "rectangle", "rhombus", "parallelogram", "trapezoid"] {
        assert!(
            out.contains(&format!("\"Shape\":\"{shape}\"")),
            "{shape} decomposes into 2 triangles (reverse recall): {out}"
        );
    }
}

#[test]
fn triangle_decomposition_count_abstains_on_a_non_quadrilateral() {
    let dir = scratch("abstain");
    place_stdlib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"shape-composition.adj\"\n\
         ? triangle_decomposition_count(triangle, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a triangle has no diagonal to draw — honest abstention, never a fabricated count: {out}"
    );
}
