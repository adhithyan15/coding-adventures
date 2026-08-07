//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/ear-parts.adj`) driven through the built CLI:
//! a native `table` of ear structure → ear region resolves binding-query
//! recalls with the source's NIDCD "How Do We Hear?" citation, runs the
//! relation backward (region → structure, recalling the three middle-ear
//! ossicles), and abstains on a non-listed structure (the pinna) — 0 model
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsear_{tag}_{}", std::process::id()));
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
fn anatomy_ear_parts_recall_binds_region_with_citation() {
    let dir = scratch("earparts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/ear-parts.adj");
    std::fs::copy(&src, dir.join("ear-parts.adj")).expect("copy shipped ear-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"ear-parts.adj\"\n\
         ? ear_structure_region(cochlea, $R)\n\
         ? ear_structure_region(malleus, $R)\n\
         ? ear_structure_region(ear_canal, $R)\n\
         ? ear_structure_region($S, middle_ear)\n\
         ? ear_structure_region(pinna, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The cochlea sits in the inner ear; the malleus (an ossicle) in the middle
    // ear; the ear canal in the outer ear — the recalled regions, each a plain
    // token verbatim from the NIDCD path-of-sound description.
    assert!(out.contains("\"R\":\"inner_ear\""), "cochlea → inner_ear: {out}");
    assert!(out.contains("\"R\":\"middle_ear\""), "malleus → middle_ear: {out}");
    assert!(out.contains("\"R\":\"outer_ear\""), "ear_canal → outer_ear: {out}");
    // The relation runs backward: the region middle_ear recalls the three tiny
    // bones — malleus, incus, and stapes.
    assert!(
        out.contains("\"S\":\"malleus\"")
            && out.contains("\"S\":\"incus\"")
            && out.contains("\"S\":\"stapes\""),
        "middle_ear → malleus/incus/stapes (reverse recall): {out}"
    );
    // The answer carries the NIDCD citation as its proof.
    assert!(
        out.contains("nidcd.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The pinna is not a listed structure — honest abstention, never a
    // fabricated region.
    assert!(
        out.contains("\"abstained\":true"),
        "unknown structure abstains: {out}"
    );
}
