//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/vitamins.adj`) driven through the built CLI:
//! a native `table` of vitamin → deficiency disease resolves binding-query
//! recalls (forward AND backward) with the source's NIH / ODS citation, and
//! abstains on a word that is not one of these grounded vitamins (vitamin_e,
//! deliberately left out) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsvit_{tag}_{}", std::process::id()));
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
fn biology_vitamins_recall_binds_deficiency_disease_with_citation() {
    let dir = scratch("vitamins");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/vitamins.adj");
    std::fs::copy(&src, dir.join("vitamins.adj")).expect("copy shipped vitamins.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"vitamins.adj\"\n\
         ? deficiency_disease(vitamin_c, $Disease)\n\
         ? deficiency_disease(vitamin_d, $Disease)\n\
         ? deficiency_disease(vitamin_a, $Disease)\n\
         ? deficiency_disease(vitamin_b1, $Disease)\n\
         ? deficiency_disease(vitamin_b3, $Disease)\n\
         ? deficiency_disease($V, megaloblastic_anemia)\n\
         ? deficiency_disease(vitamin_e, $Disease)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A lack of vitamin C causes scurvy; vitamin D, rickets; vitamin A,
    // xerophthalmia; thiamin (B1), beriberi; niacin (B3), pellagra — the
    // recalled diseases, each the word its NIH source uses (forward binds).
    assert!(out.contains("\"Disease\":\"scurvy\""), "vitamin_c → scurvy: {out}");
    assert!(out.contains("\"Disease\":\"rickets\""), "vitamin_d → rickets: {out}");
    assert!(
        out.contains("\"Disease\":\"xerophthalmia\""),
        "vitamin_a → xerophthalmia: {out}"
    );
    assert!(
        out.contains("\"Disease\":\"beriberi\""),
        "vitamin_b1 → beriberi: {out}"
    );
    assert!(
        out.contains("\"Disease\":\"pellagra\""),
        "vitamin_b3 → pellagra: {out}"
    );
    // The relation runs BACKWARD: bind the disease megaloblastic_anemia, recall
    // BOTH vitamins whose lack causes it — folate (B9) and B12.
    assert!(
        out.contains("\"V\":\"vitamin_b9\"") && out.contains("\"V\":\"vitamin_b12\""),
        "megaloblastic_anemia → vitamin_b9 ; vitamin_b12 (reverse recall): {out}"
    );
    // The answer carries the NIH / ODS citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("ods.od.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // vitamin_e is a real vitamin but was deliberately NOT grounded to a single
    // clean source-stated disease token, so it is not a row — an honest
    // abstention, never a fabricated disease.
    assert!(out.contains("\"abstained\":true"), "vitamin_e abstains: {out}");
}
