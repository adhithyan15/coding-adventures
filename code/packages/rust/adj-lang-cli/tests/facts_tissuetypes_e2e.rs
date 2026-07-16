//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/tissue-types.adj`) driven through the built CLI:
//! a native `table` of the four basic human tissue types → a representative
//! example / location resolves binding-query recalls (forward AND backward)
//! with the source's NCI SEER Training Modules citation, and abstains on a word
//! that is not one of the four basic tissue types (the epidermis) — 0 model
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
fn biology_tissue_types_recall_binds_example_with_citation() {
    let dir = scratch("tissuetypes");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/tissue-types.adj");
    std::fs::copy(&src, dir.join("tissue-types.adj")).expect("copy shipped tissue-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"tissue-types.adj\"\n\
         ? tissue_example(epithelial, $Example)\n\
         ? tissue_example(muscle, $Example)\n\
         ? tissue_example(nervous, $Example)\n\
         ? tissue_example($Tissue, bone_or_blood)\n\
         ? tissue_example(epidermis, $Example)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Epithelial tissue covers and lines, muscle tissue is cardiac and skeletal,
    // nervous tissue is the brain and nerves — the recalled examples (forward
    // binds).
    assert!(
        out.contains("\"Example\":\"covering_lining\""),
        "epithelial → covering_lining: {out}"
    );
    assert!(
        out.contains("\"Example\":\"cardiac_or_skeletal\""),
        "muscle → cardiac_or_skeletal: {out}"
    );
    assert!(
        out.contains("\"Example\":\"brain_and_nerves\""),
        "nervous → brain_and_nerves: {out}"
    );
    // The relation runs BACKWARD: bind the example `bone_or_blood`, recall its
    // tissue type.
    assert!(
        out.contains("\"Tissue\":\"connective\""),
        "bone_or_blood → connective (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training Modules citation as its proof, at
    // the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The epidermis is a structure, not one of the four basic tissue types —
    // honest abstention, never a fabricated example.
    assert!(out.contains("\"abstained\":true"), "epidermis abstains: {out}");
}
