//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/vitamin-deficiency-symptom.adj`) driven
//! through the built CLI: a native `table` recording, for five of the
//! seven vitamins already tabled in `vitamins.adj`, the SYMPTOM described
//! in the same already-quoted NIH span that names the deficiency disease
//! -- a sibling decoding the symptom half of each already-verified quote.
//! Resolves forward and backward recall queries with the source's
//! citation, plus honest abstention on vitamin_c (whose cited span names
//! scurvy but states no symptom) -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_vitamindeficiencysymptom_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/vitamin-deficiency-symptom.adj");
    std::fs::copy(&src, dir.join("vitamin-deficiency-symptom.adj"))
        .expect("copy shipped vitamin-deficiency-symptom.adj");
}

#[test]
fn vitamin_deficiency_symptom_recalls_vitamin_d_bone_symptom_with_citation() {
    let dir = scratch("vitamind");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vitamin-deficiency-symptom.adj\"\n\
         ? vitamin_deficiency_symptom(vitamin_d, $Symptom)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"vitamin_deficiency_symptom(vitamin_d, soft_weak_deformed_painful_bones)\""),
        "vitamin_d deficiency should recall the cited bone symptom: {out}"
    );
    assert!(
        out.contains("ods.od.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIH ODS citation: {out}"
    );
}

#[test]
fn vitamin_deficiency_symptom_backward_recalls_vitamin_b12_for_tired_and_weak() {
    let dir = scratch("b12");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vitamin-deficiency-symptom.adj\"\n\
         ? vitamin_deficiency_symptom($V, tired_and_weak)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"vitamin_deficiency_symptom(vitamin_b12, tired_and_weak)\""),
        "vitamin_b12 should be the only recalled tired-and-weak vitamin: {out}"
    );
    assert!(
        !out.contains("vitamin_deficiency_symptom(vitamin_b9, tired_and_weak)"),
        "vitamin_b9's cited symptom is weakness_and_fatigue, not tired_and_weak: {out}"
    );
}

#[test]
fn vitamin_deficiency_symptom_abstains_on_vitamin_c() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vitamin-deficiency-symptom.adj\"\n\
         ? vitamin_deficiency_symptom(vitamin_c, $Symptom)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "vitamin_c's cited span names scurvy but states no symptom -- honest abstention expected: {out}"
    );
}
