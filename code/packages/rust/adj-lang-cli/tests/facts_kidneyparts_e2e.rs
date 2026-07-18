//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/kidney-parts.adj`) driven through the built CLI:
//! a native `table` of the structural parts of the kidney / urinary system →
//! what each one is or does resolves binding-query recalls (forward AND
//! backward) with the source's NCI SEER Training Modules citation, and abstains
//! on a word that is not a urinary-tract part (the alveolus) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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
fn anatomy_kidney_parts_recall_binds_role_with_citation() {
    let dir = scratch("kidneyparts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/kidney-parts.adj");
    std::fs::copy(&src, dir.join("kidney-parts.adj")).expect("copy shipped kidney-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"kidney-parts.adj\"\n\
         ? kidney_part(renal_cortex, $Role)\n\
         ? kidney_part(nephron, $Role)\n\
         ? kidney_part(ureter, $Role)\n\
         ? kidney_part($Part, collects_urine)\n\
         ? kidney_part(alveolus, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The renal cortex is the outer region, the nephron is the filtering unit,
    // and the ureter carries urine down to the bladder — the recalled roles
    // (forward binds).
    assert!(
        out.contains("\"Role\":\"outer_region\""),
        "renal_cortex → outer_region: {out}"
    );
    assert!(
        out.contains("\"Role\":\"filtering_unit\""),
        "nephron → filtering_unit: {out}"
    );
    assert!(
        out.contains("\"Role\":\"carries_urine_to_bladder\""),
        "ureter → carries_urine_to_bladder: {out}"
    );
    // The relation runs BACKWARD: bind the role `collects_urine`, recall its
    // part.
    assert!(
        out.contains("\"Part\":\"renal_pelvis\""),
        "collects_urine → renal_pelvis (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training Modules citation as its proof, at
    // the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The alveolus is a lung structure, not a part of the kidney / urinary
    // system — honest abstention, never a fabricated role.
    assert!(out.contains("\"abstained\":true"), "alveolus abstains: {out}");
}
