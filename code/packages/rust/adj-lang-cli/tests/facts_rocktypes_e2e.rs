//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/rock-types.adj`) driven through the built
//! CLI: a native `table` of rock type → how it forms resolves a binding-query
//! recall with the NPS citation, and abstains on `magma` (the molten material
//! rock forms FROM, not one of the three rock types) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsrt_{tag}_{}", std::process::id()));
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
fn rock_types_recall_binds_formation_with_citation() {
    let dir = scratch("rocktypes");
    // Copy the shipped earth-science table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/rock-types.adj");
    std::fs::copy(&src, dir.join("rock-types.adj")).expect("copy shipped rock-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"rock-types.adj\"\n\
         ? rock_formation(igneous, $How)\n\
         ? rock_formation(metamorphic, $How)\n\
         ? rock_formation(magma, $How)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Igneous rock is cooled magma; metamorphic rock is made by heat and
    // pressure — the recalled formations, straight from the grounded rows.
    assert!(out.contains("\"How\":\"cooled_magma\""), "igneous → cooled_magma: {out}");
    assert!(
        out.contains("\"How\":\"heat_and_pressure\""),
        "metamorphic → heat_and_pressure: {out}"
    );
    // The answer carries the NPS citation (authoritative, a .gov source) as proof.
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS source citation: {out}"
    );
    // Magma is the molten material rock forms FROM, not a rock type — honest
    // abstention, never a fabricated formation.
    assert!(out.contains("\"abstained\":true"), "magma abstains: {out}");
}
