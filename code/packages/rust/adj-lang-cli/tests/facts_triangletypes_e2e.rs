//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/triangle-types.adj`) driven through the built
//! CLI: a native `table` of the three triangle types CLASSIFIED BY THEIR SIDES
//! → the defining side-condition resolves binding-query recalls (forward AND
//! backward) with the source's Wolfram MathWorld citation, and abstains on a
//! word that is not one of the three side-classes (a `square`, which is not a
//! triangle at all) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factstri_{tag}_{}", std::process::id()));
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
fn geometry_triangle_types_recall_binds_side_condition_with_citation() {
    let dir = scratch("triangletypes");
    // Copy the shipped geometry table beside the entry program and import it.
    let src = facts_stdlib().join("geometry/triangle-types.adj");
    std::fs::copy(&src, dir.join("triangle-types.adj")).expect("copy shipped triangle-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"triangle-types.adj\"\n\
         ? triangle_sides(equilateral, $Condition)\n\
         ? triangle_sides(isosceles, $Condition)\n\
         ? triangle_sides(scalene, $Condition)\n\
         ? triangle_sides($Type, three_equal_sides)\n\
         ? triangle_sides(square, $Condition)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // An equilateral triangle has three equal sides, an isosceles two, a scalene
    // three unequal — the recalled side-conditions (forward binds).
    assert!(
        out.contains("\"Condition\":\"three_equal_sides\""),
        "equilateral → three_equal_sides: {out}"
    );
    assert!(
        out.contains("\"Condition\":\"two_equal_sides\""),
        "isosceles → two_equal_sides: {out}"
    );
    assert!(
        out.contains("\"Condition\":\"three_unequal_sides\""),
        "scalene → three_unequal_sides: {out}"
    );
    // The relation runs BACKWARD: bind the condition `three_equal_sides`, recall
    // its triangle type.
    assert!(
        out.contains("\"Type\":\"equilateral\""),
        "three_equal_sides → equilateral (reverse recall): {out}"
    );
    // The answer carries the Wolfram MathWorld citation as its proof, at the
    // `authoritative` trust tier.
    assert!(
        out.contains("mathworld.wolfram.com") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A square is not a triangle — honest abstention, never a fabricated
    // side-condition.
    assert!(out.contains("\"abstained\":true"), "square abstains: {out}");
}
