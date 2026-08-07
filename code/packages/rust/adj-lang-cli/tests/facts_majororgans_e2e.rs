//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/major-organs.adj`) driven through the built CLI:
//! a native `table` of major human organ → its main job resolves a binding-query
//! recall with the source's citation (forward AND reverse), and abstains on a
//! word that is not one of these organs — 0 model calls.

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
fn biology_major_organs_recall_binds_function_with_citation() {
    let dir = scratch("majororgans");
    // Copy the shipped major-organs table beside the entry program and import it.
    let src = facts_stdlib().join("biology/major-organs.adj");
    std::fs::copy(&src, dir.join("major-organs.adj")).expect("copy shipped major-organs.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"major-organs.adj\"\n\
         ? organ_function(heart, $Job)\n\
         ? organ_function(kidneys, $Job)\n\
         ? organ_function($Organ, gas_exchange)\n\
         ? organ_function(bone, $Job)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The heart pumps blood; the kidneys filter blood — the recalled jobs.
    assert!(out.contains("\"Job\":\"pumps_blood\""), "heart → pumps_blood: {out}");
    assert!(out.contains("\"Job\":\"filter_blood\""), "kidneys → filter_blood: {out}");
    // The query also runs BACKWARD: bind the job, recall the organ that does it.
    assert!(out.contains("\"Organ\":\"lungs\""), "gas_exchange → lungs: {out}");
    // The answer carries the NIH / NHLBI citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("nhlbi.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A bone is not one of these organs — honest abstention, never a fabricated job.
    assert!(out.contains("\"abstained\":true"), "bone abstains: {out}");
}
