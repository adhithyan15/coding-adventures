//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/heart-chambers.adj`) driven through the built CLI:
//! a native `table` of heart-chamber → function resolves binding-query recalls
//! (forward and backward) with the source's NIH citation, and abstains on a
//! non-chamber (the aorta) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn anatomy_heart_chambers_recall_binds_function_with_citation() {
    let dir = scratch("heartchambers");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/heart-chambers.adj");
    std::fs::copy(&src, dir.join("heart-chambers.adj")).expect("copy shipped heart-chambers.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"heart-chambers.adj\"\n\
         ? heart_chamber_function(right_atrium, $Job)\n\
         ? heart_chamber_function(right_ventricle, $Job)\n\
         ? heart_chamber_function($C, pumps_blood_to_body)\n\
         ? heart_chamber_function(aorta, $Job)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The right atrium receives blood from the body; the right ventricle pumps
    // it on to the lungs — the recalled jobs (forward binds).
    assert!(
        out.contains("\"Job\":\"receives_blood_from_body\""),
        "right_atrium → receives_blood_from_body: {out}"
    );
    assert!(
        out.contains("\"Job\":\"pumps_blood_to_lungs\""),
        "right_ventricle → pumps_blood_to_lungs: {out}"
    );
    // The relation runs BACKWARD: bind the job, recall the chamber.
    assert!(
        out.contains("\"C\":\"left_ventricle\""),
        "pumps_blood_to_body → left_ventricle (reverse recall): {out}"
    );
    // The answer carries the NIH / NLM StatPearls citation as its proof.
    assert!(
        out.contains("ncbi.nlm.nih.gov/books/NBK470256")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The aorta is not a chamber — honest abstention, never a fabricated job.
    assert!(out.contains("\"abstained\":true"), "aorta abstains: {out}");
}
