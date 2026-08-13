//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/quadrilateral-alt-name.adj`) driven through
//! the built CLI: a native `table` naming the alternate word MathWorld's
//! source states for a rhombus -- a sibling to the already-shipped
//! `quadrilateral-types.adj` (which only carries each shape's defining
//! PROPERTY, not an alternate name), decoding a clause already sitting
//! unused inside that table's own provenance block. Resolves binding-query
//! recall (both directions) with the source's citation, and abstains on a
//! shape (square) the cited spans give no alternate name for -- 0 model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_quadaltname_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geometry/quadrilateral-alt-name.adj");
    std::fs::copy(&src, dir.join("quadrilateral-alt-name.adj"))
        .expect("copy shipped quadrilateral-alt-name.adj");
}

#[test]
fn quadrilateral_alt_name_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"quadrilateral-alt-name.adj\"\n\
         ? quadrilateral_alt_name(rhombus, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"quadrilateral_alt_name(rhombus, equilateral_parallelogram)\""),
        "rhombus's alt name is equilateral_parallelogram: {out}"
    );
    assert!(
        out.contains("mathworld.wolfram.com") && out.contains("\"trust\":\"authoritative\""),
        "carries the MathWorld citation: {out}"
    );
}

#[test]
fn quadrilateral_alt_name_recalls_backward_from_a_bound_alt_name() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"quadrilateral-alt-name.adj\"\n\
         ? quadrilateral_alt_name($Shape, equilateral_parallelogram)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"quadrilateral_alt_name(rhombus, equilateral_parallelogram)\""),
        "equilateral_parallelogram names rhombus: {out}"
    );
}

#[test]
fn quadrilateral_alt_name_abstains_honestly_on_square() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"quadrilateral-alt-name.adj\"\n\
         ? quadrilateral_alt_name(square, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "square has no alternate name in the cited spans -- honest abstention: {out}"
    );
}
