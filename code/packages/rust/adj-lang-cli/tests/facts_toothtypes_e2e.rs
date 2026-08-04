//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/tooth-types.adj`) driven through the built CLI:
//! a native `table` of tooth type → its job resolves a binding-query recall
//! with the source's citation, runs the relation backward (job → tooth types),
//! and abstains on something that is not a tooth — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factstooth_{tag}_{}", std::process::id()));
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
fn anatomy_tooth_types_recall_binds_function_with_citation() {
    let dir = scratch("toothtypes");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/tooth-types.adj");
    std::fs::copy(&src, dir.join("tooth-types.adj")).expect("copy shipped tooth-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"tooth-types.adj\"\n\
         ? tooth_function(incisors, $Job)\n\
         ? tooth_function(canines, $Job)\n\
         ? tooth_function($T, grinding)\n\
         ? tooth_function(gums, $Job)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Incisors cut food into pieces; canines tear — the recalled jobs, each a
    // plain lowercase atom copied from the source's own wording.
    assert!(out.contains("\"Job\":\"cutting\""), "incisors → cutting: {out}");
    assert!(out.contains("\"Job\":\"tearing\""), "canines → tearing: {out}");
    // The relation runs backward: the job `grinding` recalls the wide back
    // teeth — both premolars and molars do the grinding.
    assert!(out.contains("\"T\":\"premolars\""), "grinding → premolars (reverse recall): {out}");
    assert!(out.contains("\"T\":\"molars\""), "grinding → molars (reverse recall): {out}");
    // The answer carries the NIH/NLM (ncbi.nlm.nih.gov) citation as its proof.
    assert!(
        out.contains("ncbi.nlm.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // "gums" are not a tooth type — honest abstention, never a fabricated job.
    assert!(out.contains("\"abstained\":true"), "unknown tooth abstains: {out}");
}
