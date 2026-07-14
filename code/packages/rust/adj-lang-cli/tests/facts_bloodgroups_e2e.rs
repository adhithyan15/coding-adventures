//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/blood-groups.adj`) driven through the built CLI:
//! a native `table` of ABO blood type → antigen(s) on the red cells resolves a
//! binding-query recall with the source's citation, and abstains on a label that
//! is not an ABO phenotype — 0 model calls.

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
fn biology_blood_groups_recall_binds_antigen_with_citation() {
    let dir = scratch("bloodgroups");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/blood-groups.adj");
    std::fs::copy(&src, dir.join("blood-groups.adj")).expect("copy shipped blood-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"blood-groups.adj\"\n\
         ? blood_type_antigen(a, $Antigen)\n\
         ? blood_type_antigen(ab, $Antigen)\n\
         ? blood_type_antigen(o, $Antigen)\n\
         ? blood_type_antigen(rh_positive, $Antigen)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Type A carries the A antigen; type AB carries both — the recalled atoms.
    assert!(out.contains("\"Antigen\":\"a_antigen\""), "type a → a_antigen: {out}");
    assert!(
        out.contains("\"Antigen\":\"a_and_b_antigens\""),
        "type ab → a_and_b_antigens: {out}"
    );
    // Type O positively carries NO ABO antigen (why O cells are universal donors).
    assert!(
        out.contains("\"Antigen\":\"no_antigens\""),
        "type o → no_antigens: {out}"
    );
    // The answer carries the NCBI Bookshelf citation as its proof.
    assert!(
        out.contains("ncbi.nlm.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // An Rh label is not an ABO phenotype — honest abstention, never a fabricated antigen.
    assert!(out.contains("\"abstained\":true"), "rh_positive abstains: {out}");
}
