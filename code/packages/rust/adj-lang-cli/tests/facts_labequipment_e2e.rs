//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/lab-equipment.adj`) driven through the built
//! CLI: a native `table` of common lab equipment → its use resolves
//! binding-query recalls (forward and backward) with the source's LibreTexts
//! citation, and abstains on a tool not in the table (stir rod) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factslabeq_{tag}_{}", std::process::id()));
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
fn chemistry_equipment_use_recall_binds_use_with_citation() {
    let dir = scratch("labequipment");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/lab-equipment.adj");
    std::fs::copy(&src, dir.join("lab-equipment.adj")).expect("copy shipped lab-equipment.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"lab-equipment.adj\"\n\
         ? equipment_use(beaker, $U)\n\
         ? equipment_use(funnel, $U)\n\
         ? equipment_use($E, heat)\n\
         ? equipment_use(stir_rod, $U)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A beaker holds liquid; a funnel transfers it — the recalled use verbs
    // (forward binds), each copied verbatim from the source sentence.
    assert!(out.contains("\"U\":\"hold\""), "beaker -> hold: {out}");
    assert!(out.contains("\"U\":\"transfer\""), "funnel -> transfer: {out}");
    // The relation runs BACKWARD: bind the use `heat`, recall the tool — the
    // Bunsen burner, the open-flame heat source.
    assert!(
        out.contains("\"E\":\"bunsen_burner\""),
        "heat -> bunsen_burner (reverse recall to the heat source): {out}"
    );
    // The answer carries the LibreTexts citation as its proof, at consensus trust
    // (a secondary teaching reference, honestly tiered).
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation at consensus trust: {out}"
    );
    // "stir_rod" is not in the table — honest abstention, never a fabricated use.
    assert!(out.contains("\"abstained\":true"), "stir_rod abstains: {out}");
}
