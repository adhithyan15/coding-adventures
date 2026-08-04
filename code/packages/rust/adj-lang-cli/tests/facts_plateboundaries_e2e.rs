//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/plate-boundaries.adj`) driven through the
//! built CLI: a native `table` of the three tectonic plate-boundary types → how
//! the plates move at each resolves binding-query recalls (forward AND backward)
//! with the source's National Park Service citation, and abstains on a word that
//! is not one of the three boundary types (the equator) — 0 model calls.

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
fn earth_science_plate_boundaries_recall_binds_motion_with_citation() {
    let dir = scratch("plateboundaries");
    // Copy the shipped earth-science table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/plate-boundaries.adj");
    std::fs::copy(&src, dir.join("plate-boundaries.adj"))
        .expect("copy shipped plate-boundaries.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"plate-boundaries.adj\"\n\
         ? boundary_motion(divergent, $Motion)\n\
         ? boundary_motion(convergent, $Motion)\n\
         ? boundary_motion(transform, $Motion)\n\
         ? boundary_motion($Boundary, slide_past)\n\
         ? boundary_motion(equator, $Motion)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Plates rip apart at a divergent boundary, one plate subducts at a
    // convergent boundary, and plates slide past at a transform boundary — the
    // recalled motions (forward binds).
    assert!(
        out.contains("\"Motion\":\"rip_apart\""),
        "divergent → rip_apart: {out}"
    );
    assert!(
        out.contains("\"Motion\":\"subducts\""),
        "convergent → subducts: {out}"
    );
    assert!(
        out.contains("\"Motion\":\"slide_past\""),
        "transform → slide_past: {out}"
    );
    // The relation runs BACKWARD: bind the motion `slide_past`, recall its
    // boundary type.
    assert!(
        out.contains("\"Boundary\":\"transform\""),
        "slide_past → transform (reverse recall): {out}"
    );
    // The answer carries the National Park Service citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The equator is a line of latitude, not one of the three plate-boundary
    // types — honest abstention, never a fabricated motion.
    assert!(out.contains("\"abstained\":true"), "equator abstains: {out}");
}
