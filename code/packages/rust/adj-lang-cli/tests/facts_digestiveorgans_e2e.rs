//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/digestive-organs.adj`) driven through the built
//! CLI: a native `table` of digestive-system organ → primary function resolves a
//! binding-query recall with the source's NIH NIDDK / NCI SEER Training
//! (authoritative) citation, runs the relation backward (function → the organ
//! that owns it), and abstains on a non-digestive organ (the heart) — 0 model
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
    let dir =
        std::env::temp_dir().join(format!("adjcli_factsdigest_{tag}_{}", std::process::id()));
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
fn anatomy_digestive_organs_recall_binds_function_with_citation() {
    let dir = scratch("digestiveorgans");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/digestive-organs.adj");
    std::fs::copy(&src, dir.join("digestive-organs.adj"))
        .expect("copy shipped digestive-organs.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"digestive-organs.adj\"\n\
         ? organ_function(stomach, $Job)\n\
         ? organ_function(small_intestine, $Job)\n\
         ? organ_function(liver, $Job)\n\
         ? organ_function($Organ, absorbs_water)\n\
         ? organ_function(heart, $Job)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Each named organ binds its headline function — a single lowercase token
    // echoed verbatim from the source sentence.
    assert!(out.contains("\"Job\":\"break_down_food\""), "stomach → break_down_food: {out}");
    assert!(
        out.contains("\"Job\":\"absorbs_nutrients\""),
        "small_intestine → absorbs_nutrients: {out}"
    );
    assert!(out.contains("\"Job\":\"bile\""), "liver → bile: {out}");
    // The relation runs backward: the function `absorbs_water` recalls the organ
    // that owns it — the large intestine.
    assert!(
        out.contains("\"Organ\":\"large_intestine\""),
        "absorbs_water → large_intestine (reverse recall): {out}"
    );
    // The answer carries the primary NIH / NCI citation as its proof, at
    // authoritative trust.
    assert!(
        (out.contains("seer.cancer.gov") || out.contains("niddk.nih.gov"))
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The heart is a circulatory organ, not a digestive one — honest abstention,
    // never a fabricated function.
    assert!(out.contains("\"abstained\":true"), "unknown organ abstains: {out}");
}
