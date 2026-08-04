//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/cell-organelles.adj`) driven through the built CLI:
//! a native `table` of organelle → primary-function resolves a binding-query
//! recall with the source's citation, runs the relation backward
//! (function → organelle), and abstains on a non-organelle — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsbio_{tag}_{}", std::process::id()));
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
fn biology_organelles_recall_binds_function_with_citation() {
    let dir = scratch("organelles");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/cell-organelles.adj");
    std::fs::copy(&src, dir.join("cell-organelles.adj")).expect("copy shipped cell-organelles.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"cell-organelles.adj\"\n\
         ? organelle_function(mitochondrion, $F)\n\
         ? organelle_function(ribosome, $F)\n\
         ? organelle_function($O, photosynthesis)\n\
         ? organelle_function(smartphone, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The mitochondrion produces energy; the ribosome makes protein — the
    // recalled primary-function atoms.
    assert!(out.contains("\"F\":\"produces_energy\""), "mitochondrion → produces_energy: {out}");
    assert!(out.contains("\"F\":\"makes_protein\""), "ribosome → makes_protein: {out}");
    // The relation runs backward: the function photosynthesis recalls chloroplast.
    assert!(
        out.contains("\"O\":\"chloroplast\""),
        "photosynthesis → chloroplast (reverse recall): {out}"
    );
    // The answer carries the Wikipedia citation as its proof.
    assert!(
        out.contains("en.wikipedia.org/wiki/Organelle") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // "smartphone" is not a cell organelle — honest abstention, never a
    // fabricated function.
    assert!(out.contains("\"abstained\":true"), "non-organelle abstains: {out}");
}
