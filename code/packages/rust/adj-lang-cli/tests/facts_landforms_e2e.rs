//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/landforms.adj`) driven through the built CLI:
//! a native `table` of common landform → the short defining descriptor its
//! source states resolves binding-query recalls (forward AND backward) with the
//! USGS-hosted Feature Type Thesaurus citation, and abstains on `ocean` (a body
//! of water, not a landform) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factslf_{tag}_{}", std::process::id()));
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
fn landforms_recall_binds_description_with_citation() {
    let dir = scratch("landforms");
    // Copy the shipped geography table beside the entry program and import it.
    let src = facts_stdlib().join("geography/landforms.adj");
    std::fs::copy(&src, dir.join("landforms.adj")).expect("copy shipped landforms.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"landforms.adj\"\n\
         ? landform_description(mountain, $Desc)\n\
         ? landform_description(canyon, $Desc)\n\
         ? landform_description(plateau, $Desc)\n\
         ? landform_description($Landform, deep_narrow)\n\
         ? landform_description(ocean, $Desc)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A mountain projects above its surroundings; a canyon is narrow and deep;
    // a plateau is a flat, elevated area — the recalled descriptors, straight
    // from the grounded rows (forward binds).
    assert!(
        out.contains("\"Desc\":\"projects_above_surroundings\""),
        "mountain → projects_above_surroundings: {out}"
    );
    assert!(out.contains("\"Desc\":\"deep_narrow\""), "canyon → deep_narrow: {out}");
    assert!(
        out.contains("\"Desc\":\"flat_elevated\""),
        "plateau → flat_elevated: {out}"
    );
    // The relation runs BACKWARD: bind the descriptor `deep_narrow`, recall its
    // landform.
    assert!(
        out.contains("\"Landform\":\"canyon\""),
        "deep_narrow → canyon (reverse recall): {out}"
    );
    // The answer carries the USGS citation (authoritative, a .gov source) as proof.
    assert!(
        out.contains("apps.usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS source citation: {out}"
    );
    // An ocean is a body of water, not a landform — honest abstention, never a
    // fabricated descriptor.
    assert!(out.contains("\"abstained\":true"), "ocean abstains: {out}");
}
