//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/skin-layer-function.adj`) driven through the
//! built CLI: a native `table` naming the function of two named skin
//! layers, decoded from clauses already sitting unused inside
//! `skin-layers.adj`'s own already-quoted NCI SEER source sentences -- a
//! sibling to that table. Resolves binding-query recall (both directions,
//! and a two-answer forward recall for subcutaneous) with the source's
//! citation, and abstains on a real, already-tabled layer (dermis) whose
//! own quote states only its location and thickness, never a function --
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
    let dir = std::env::temp_dir().join(format!("adjcli_skinlayerfunction_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/skin-layer-function.adj");
    std::fs::copy(&src, dir.join("skin-layer-function.adj"))
        .expect("copy shipped skin-layer-function.adj");
}

#[test]
fn skin_layer_function_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"skin-layer-function.adj\"\n\
         ? skin_layer_function(epidermis, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"skin_layer_function(epidermis, protects_body)\""),
        "the epidermis protects the body: {out}"
    );
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NCI SEER citation: {out}"
    );
}

#[test]
fn skin_layer_function_recalls_backward_to_subcutaneous() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"skin-layer-function.adj\"\n\
         ? skin_layer_function($Layer, shock_absorber)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"skin_layer_function(subcutaneous, shock_absorber)\""),
        "shock_absorber recalls the subcutaneous layer: {out}"
    );
}

#[test]
fn skin_layer_function_abstains_honestly_on_dermis() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"skin-layer-function.adj\"\n\
         ? skin_layer_function(dermis, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "dermis is a real, already-tabled skin layer but its own quote states only its location and thickness, never a function -- honest abstention: {out}"
    );
}
