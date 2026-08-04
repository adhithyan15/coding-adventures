//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/brain-parts.adj`) driven through the built CLI:
//! a native `table` of brain part → primary function resolves a binding-query
//! recall with the source's NCI SEER Training citation, runs the relation
//! backward (function → part), and abstains on a non-part (a neuron) — 0 model
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsbrain_{tag}_{}", std::process::id()));
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
fn anatomy_brain_parts_recall_binds_function_with_citation() {
    let dir = scratch("brainparts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/brain-parts.adj");
    std::fs::copy(&src, dir.join("brain-parts.adj")).expect("copy shipped brain-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"brain-parts.adj\"\n\
         ? brain_part_function(cerebellum, $Job)\n\
         ? brain_part_function(hippocampus, $Job)\n\
         ? brain_part_function($Part, breathing)\n\
         ? brain_part_function(neuron, $Job)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The cerebellum coordinates voluntary movement; the hippocampus does
    // memory — the recalled functions, each a single verbatim token/phrase.
    assert!(
        out.contains("\"Job\":\"coordination_of_voluntary_movement\""),
        "cerebellum → coordination_of_voluntary_movement: {out}"
    );
    assert!(out.contains("\"Job\":\"memory\""), "hippocampus → memory: {out}");
    // The relation runs backward: the function `breathing` recalls the brainstem.
    assert!(
        out.contains("\"Part\":\"brainstem\""),
        "breathing → brainstem (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A neuron is a cell, not one of these gross brain parts — honest
    // abstention, never a fabricated function.
    assert!(out.contains("\"abstained\":true"), "unknown part abstains: {out}");
}
