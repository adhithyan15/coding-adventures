//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/skin-layer-alt-name.adj`) driven through the
//! built CLI: a native `table` naming the everyday alternate name for the
//! subcutaneous skin layer, decoded from a clause already sitting unused
//! inside `skin-layers.adj`'s own already-quoted NCI SEER source sentence
//! -- a sibling to that table. Resolves binding-query recall (both
//! directions) with the source's citation, and abstains on a real,
//! already-tabled layer (epidermis) whose own quote never supplies an
//! alternate name -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_skinlayeraltname_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/skin-layer-alt-name.adj");
    std::fs::copy(&src, dir.join("skin-layer-alt-name.adj"))
        .expect("copy shipped skin-layer-alt-name.adj");
}

#[test]
fn skin_layer_alt_name_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"skin-layer-alt-name.adj\"\n\
         ? skin_layer_alt_name(subcutaneous, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"skin_layer_alt_name(subcutaneous, hypodermis)\""),
        "the subcutaneous layer is also known as the hypodermis: {out}"
    );
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NCI SEER citation: {out}"
    );
}

#[test]
fn skin_layer_alt_name_recalls_backward_to_subcutaneous() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"skin-layer-alt-name.adj\"\n\
         ? skin_layer_alt_name($Layer, hypodermis)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"skin_layer_alt_name(subcutaneous, hypodermis)\""),
        "hypodermis recalls the subcutaneous layer: {out}"
    );
}

#[test]
fn skin_layer_alt_name_abstains_honestly_on_epidermis() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"skin-layer-alt-name.adj\"\n\
         ? skin_layer_alt_name(epidermis, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "epidermis is a real, already-tabled skin layer but its own quote never supplies an alternate name -- honest abstention: {out}"
    );
}
