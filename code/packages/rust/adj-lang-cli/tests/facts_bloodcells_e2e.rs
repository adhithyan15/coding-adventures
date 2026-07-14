//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/blood-cell-types.adj`) driven through the built
//! CLI: a native `table` of blood-cell-type → main-function resolves a
//! binding-query recall with the source's citation, runs the relation backward
//! (function → cell type), and abstains on a cell that is not a blood cell —
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsbloodcells_{tag}_{}", std::process::id()));
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
fn biology_blood_cells_recall_binds_function_with_citation() {
    let dir = scratch("bloodcells");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/blood-cell-types.adj");
    std::fs::copy(&src, dir.join("blood-cell-types.adj")).expect("copy shipped blood-cell-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"blood-cell-types.adj\"\n\
         ? blood_cell_function(red_blood_cells, $F)\n\
         ? blood_cell_function(platelets, $F)\n\
         ? blood_cell_function($C, fight_infection)\n\
         ? blood_cell_function(neuron, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Red blood cells carry oxygen; platelets do the clotting — the recalled
    // main-function atoms.
    assert!(out.contains("\"F\":\"carry_oxygen\""), "red_blood_cells → carry_oxygen: {out}");
    assert!(out.contains("\"F\":\"clotting\""), "platelets → clotting: {out}");
    // The relation runs backward: the job fight_infection recalls white_blood_cells.
    assert!(
        out.contains("\"C\":\"white_blood_cells\""),
        "fight_infection → white_blood_cells (reverse recall): {out}"
    );
    // The answer carries the MedlinePlus (NIH, .gov) citation as its proof.
    assert!(
        out.contains("medlineplus.gov/blood.html")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A neuron is a nerve cell, not a blood cell — honest abstention, never a
    // fabricated function.
    assert!(out.contains("\"abstained\":true"), "non-blood-cell abstains: {out}");
}
