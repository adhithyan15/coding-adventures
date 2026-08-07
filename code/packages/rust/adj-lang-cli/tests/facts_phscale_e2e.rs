//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/ph-scale.adj`) driven through the built CLI:
//! a native `table` of common substance → approximate pH resolves binding-query
//! recalls (forward and backward) with the source's LibreTexts citation, and
//! abstains on a substance not in the table (dish soap) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsph_{tag}_{}", std::process::id()));
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
fn chemistry_substance_ph_recall_binds_ph_with_citation() {
    let dir = scratch("phscale");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/ph-scale.adj");
    std::fs::copy(&src, dir.join("ph-scale.adj")).expect("copy shipped ph-scale.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"ph-scale.adj\"\n\
         ? substance_ph(lemon_juice, $P)\n\
         ? substance_ph(ammonia, $P)\n\
         ? substance_ph($S, 7.0)\n\
         ? substance_ph(dish_soap, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Lemon juice is strongly acidic (pH 2.2); ammonia is strongly basic (12.5) —
    // the recalled pH values (forward binds).
    assert!(out.contains("\"P\":\"2.2\""), "lemon_juice -> 2.2: {out}");
    assert!(out.contains("\"P\":\"12.5\""), "ammonia -> 12.5: {out}");
    // The relation runs BACKWARD: bind the neutral pH 7.0, recall the substance —
    // pure water, the middle of the scale.
    assert!(
        out.contains("\"S\":\"pure_water\""),
        "7.0 -> pure_water (reverse recall to the neutral point): {out}"
    );
    // The answer carries the LibreTexts citation as its proof, at consensus trust
    // (a secondary teaching reference, honestly tiered).
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation at consensus trust: {out}"
    );
    // "dish_soap" is not in the table — honest abstention, never a fabricated pH.
    assert!(out.contains("\"abstained\":true"), "dish_soap abstains: {out}");
}
