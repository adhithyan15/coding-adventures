//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/body-systems.adj`) driven through the built CLI:
//! a native `table` of body-system → main-function resolves a binding-query
//! recall with the source's citation, runs the relation backward
//! (function → system), and abstains on a body part that is not a system —
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsbodysys_{tag}_{}", std::process::id()));
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
fn biology_body_systems_recall_binds_function_with_citation() {
    let dir = scratch("bodysystems");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/body-systems.adj");
    std::fs::copy(&src, dir.join("body-systems.adj")).expect("copy shipped body-systems.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"body-systems.adj\"\n\
         ? body_system_function(digestive, $F)\n\
         ? body_system_function(respiratory, $F)\n\
         ? body_system_function($S, moves_blood)\n\
         ? body_system_function(elbow, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The digestive system breaks down food; the respiratory system does
    // breathing — the recalled main-function atoms.
    assert!(out.contains("\"F\":\"breaks_down_food\""), "digestive → breaks_down_food: {out}");
    assert!(out.contains("\"F\":\"breathing\""), "respiratory → breathing: {out}");
    // The relation runs backward: the job moves_blood recalls the circulatory system.
    assert!(
        out.contains("\"S\":\"circulatory\""),
        "moves_blood → circulatory (reverse recall): {out}"
    );
    // The answer carries the MedlinePlus (NIH, .gov) citation as its proof.
    assert!(
        out.contains("medlineplus.gov/ency/imagepages/1090.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // "elbow" is a body part, not a body system — honest abstention, never a
    // fabricated function.
    assert!(out.contains("\"abstained\":true"), "non-system abstains: {out}");
}
