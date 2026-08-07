//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/skin-layers.adj`) driven through the built CLI:
//! a native `table` of skin layer → defining property resolves a binding-query
//! recall with the source's NCI SEER Training citation, runs the relation
//! backward (property → layer), and abstains on a non-layer (a bone) —
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsskin_{tag}_{}", std::process::id()));
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
fn anatomy_skin_layers_recall_binds_property_with_citation() {
    let dir = scratch("skinlayers");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/skin-layers.adj");
    std::fs::copy(&src, dir.join("skin-layers.adj")).expect("copy shipped skin-layers.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"skin-layers.adj\"\n\
         ? skin_layer_property(epidermis, $P)\n\
         ? skin_layer_property(dermis, $P)\n\
         ? skin_layer_property(subcutaneous, $P)\n\
         ? skin_layer_property($L, fat)\n\
         ? skin_layer_property(bone, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Each layer binds to the defining descriptor the source states verbatim.
    assert!(out.contains("\"P\":\"outermost\""), "epidermis → outermost: {out}");
    assert!(out.contains("\"P\":\"thickest\""), "dermis → thickest: {out}");
    assert!(out.contains("\"P\":\"fat\""), "subcutaneous → fat: {out}");
    // The relation runs backward: the descriptor `fat` recalls the subcutaneous layer.
    assert!(
        out.contains("\"L\":\"subcutaneous\""),
        "fat → subcutaneous (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training citation as its proof.
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A bone is not a skin layer — honest abstention, never a fabricated descriptor.
    assert!(out.contains("\"abstained\":true"), "unknown layer abstains: {out}");
}
