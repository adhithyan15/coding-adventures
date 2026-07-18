//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/spine-regions.adj`) driven through the built CLI:
//! a native `table` of vertebral-column region → number of vertebrae resolves a
//! binding-query recall with the NCBI Bookshelf "Anatomy, Back, Vertebral
//! Column" citation, runs the relation backward (count → region), and abstains
//! on a structure that is not a spine region (the skull) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factstemp_{tag}_{}", std::process::id()));
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
fn anatomy_spine_regions_recall_binds_count_with_citation() {
    let dir = scratch("spineregions");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/spine-regions.adj");
    std::fs::copy(&src, dir.join("spine-regions.adj")).expect("copy shipped spine-regions.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"spine-regions.adj\"\n\
         ? spine_region_vertebrae(cervical, $N)\n\
         ? spine_region_vertebrae(thoracic, $N)\n\
         ? spine_region_vertebrae(coccygeal, $N)\n\
         ? spine_region_vertebrae($R, 5)\n\
         ? spine_region_vertebrae(skull, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each region to the number the source states, each a
    // plain integer.
    assert!(out.contains("\"N\":\"7\""), "cervical → 7: {out}");
    assert!(out.contains("\"N\":\"12\""), "thoracic → 12: {out}");
    assert!(out.contains("\"N\":\"4\""), "coccygeal → 4: {out}");
    // The relation runs BACKWARD: the count 5 recalls both lumbar and sacral.
    assert!(
        out.contains("\"R\":\"lumbar\""),
        "5 → lumbar (reverse recall): {out}"
    );
    assert!(
        out.contains("\"R\":\"sacral\""),
        "5 → sacral (reverse recall): {out}"
    );
    // The answer carries the NCBI locator + trust tier as its proof.
    assert!(
        out.contains("ncbi.nlm.nih.gov") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // The skull is not a region of the vertebral column — honest abstention,
    // never a fabricated count.
    assert!(
        out.contains("\"abstained\":true"),
        "ungrounded structure abstains: {out}"
    );
}
