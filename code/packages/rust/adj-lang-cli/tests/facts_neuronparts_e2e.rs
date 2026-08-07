//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/neuron-parts.adj`) driven through the built CLI:
//! a native `table` of neuron-part → job resolves a binding-query recall with
//! the source's citation, runs the relation backward (job → part), and abstains
//! on a structure that is not one of the neuron's parts — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsneuron_{tag}_{}", std::process::id()));
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
fn biology_neuron_parts_recall_binds_function_with_citation() {
    let dir = scratch("neuronparts");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/neuron-parts.adj");
    std::fs::copy(&src, dir.join("neuron-parts.adj")).expect("copy shipped neuron-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"neuron-parts.adj\"\n\
         ? neuron_part_function(dendrites, $F)\n\
         ? neuron_part_function(axon, $F)\n\
         ? neuron_part_function($P, speeds_signals)\n\
         ? neuron_part_function(synapse, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Dendrites receive the signal; the axon transmits it — the recalled job atoms.
    assert!(out.contains("\"F\":\"receive_signals\""), "dendrites → receive_signals: {out}");
    assert!(out.contains("\"F\":\"transmit_signals\""), "axon → transmit_signals: {out}");
    // The relation runs backward: the job speeds_signals recalls the myelin sheath.
    assert!(
        out.contains("\"P\":\"myelin_sheath\""),
        "speeds_signals → myelin_sheath (reverse recall): {out}"
    );
    // The answer carries the NIH / NCBI Bookshelf (.gov) citation as its proof.
    assert!(
        out.contains("ncbi.nlm.nih.gov/books/NBK441977/")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // "synapse" is the gap between neurons, not a part of the neuron itself —
    // honest abstention, never a fabricated function.
    assert!(out.contains("\"abstained\":true"), "non-part abstains: {out}");
}
