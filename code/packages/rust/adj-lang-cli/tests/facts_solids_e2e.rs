//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/solid-shapes.adj`) driven through the built CLI:
//! a native `table` of solid → number-of-faces resolves a binding-query recall
//! with the source's citation, and abstains on a non-solid — 0 model calls.

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
fn geometry_solids_recall_binds_face_count_with_citation() {
    let dir = scratch("solids");
    // Copy the shipped geometry table beside the entry program and import it.
    let src = facts_stdlib().join("geometry/solid-shapes.adj");
    std::fs::copy(&src, dir.join("solid-shapes.adj")).expect("copy shipped solid-shapes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"solid-shapes.adj\"\n\
         ? solid_faces(cube, $Faces)\n\
         ? solid_faces(icosahedron, $Faces)\n\
         ? solid_faces(sphere, $Faces)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A cube has six faces; an icosahedron has twenty — the recalled counts.
    assert!(out.contains("\"Faces\":\"6\""), "cube → 6: {out}");
    assert!(out.contains("\"Faces\":\"20\""), "icosahedron → 20: {out}");
    // The answer carries the MathWorld citation as its proof.
    assert!(
        out.contains("mathworld.wolfram.com") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A sphere has no flat faces — honest abstention, never a fabricated count.
    assert!(out.contains("\"abstained\":true"), "sphere abstains: {out}");
}
