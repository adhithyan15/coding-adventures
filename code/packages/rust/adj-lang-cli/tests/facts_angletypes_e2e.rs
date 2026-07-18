//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/angle-types.adj`) driven through the built CLI:
//! a native `table` of the five angle types → the measure-condition that
//! defines each one resolves binding-query recalls (forward AND backward) with
//! the source's Mathematics LibreTexts citation, and abstains on a word that is
//! not one of the five angle types (a circle) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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
fn geometry_angle_types_recall_binds_condition_with_citation() {
    let dir = scratch("angletypes");
    // Copy the shipped geometry table beside the entry program and import it.
    let src = facts_stdlib().join("geometry/angle-types.adj");
    std::fs::copy(&src, dir.join("angle-types.adj")).expect("copy shipped angle-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"angle-types.adj\"\n\
         ? angle_condition(acute, $Condition)\n\
         ? angle_condition(obtuse, $Condition)\n\
         ? angle_condition(reflex, $Condition)\n\
         ? angle_condition($Type, equals_90)\n\
         ? angle_condition(circle, $Condition)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Acute is smaller than a right angle, obtuse is bigger than one, reflex is
    // bigger than a straight angle — the recalled conditions (forward binds).
    assert!(
        out.contains("\"Condition\":\"between_0_and_90\""),
        "acute → between_0_and_90: {out}"
    );
    assert!(
        out.contains("\"Condition\":\"between_90_and_180\""),
        "obtuse → between_90_and_180: {out}"
    );
    assert!(
        out.contains("\"Condition\":\"greater_than_180\""),
        "reflex → greater_than_180: {out}"
    );
    // The relation runs BACKWARD: bind the condition `equals_90`, recall its
    // angle type.
    assert!(
        out.contains("\"Type\":\"right\""),
        "equals_90 → right (reverse recall): {out}"
    );
    // The answer carries the Mathematics LibreTexts citation as its proof, at
    // the `consensus` trust tier for an open educational teaching text.
    assert!(
        out.contains("math.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // A circle is not one of the five angle types — honest abstention, never a
    // fabricated condition.
    assert!(out.contains("\"abstained\":true"), "circle abstains: {out}");
}
