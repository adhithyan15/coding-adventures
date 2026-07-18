//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/fungus-parts.adj`) driven through the built CLI:
//! a native `table` of fungus parts → the defining token / role the source
//! states resolves binding-query recalls (forward AND backward) with the
//! source's UNLV biology faculty citation, and abstains on a word that is not
//! one of these fungus parts (the root) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factfp_{tag}_{}", std::process::id()));
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
fn biology_fungus_parts_recall_binds_role_with_citation() {
    let dir = scratch("fungusparts");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/fungus-parts.adj");
    std::fs::copy(&src, dir.join("fungus-parts.adj")).expect("copy shipped fungus-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"fungus-parts.adj\"\n\
         ? fungus_part_role(hypha, $Role)\n\
         ? fungus_part_role(mycelium, $Role)\n\
         ? fungus_part_role(gills, $Role)\n\
         ? fungus_part_role(stalk, $Role)\n\
         ? fungus_part_role($Part, made_of_hyphae)\n\
         ? fungus_part_role(root, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A hypha is thread-like, the mycelium is made of hyphae, the gills hold the
    // spores, the stalk lifts the cap — the recalled roles (forward binds).
    assert!(
        out.contains("\"Role\":\"thread_like\""),
        "hypha → thread_like: {out}"
    );
    assert!(
        out.contains("\"Role\":\"made_of_hyphae\""),
        "mycelium → made_of_hyphae: {out}"
    );
    assert!(
        out.contains("\"Role\":\"holds_spores\""),
        "gills → holds_spores: {out}"
    );
    assert!(
        out.contains("\"Role\":\"lifts_cap\""),
        "stalk → lifts_cap: {out}"
    );
    // The relation runs BACKWARD: bind the role `made_of_hyphae`, recall its
    // fungus part.
    assert!(
        out.contains("\"Part\":\"mycelium\""),
        "made_of_hyphae → mycelium (reverse recall): {out}"
    );
    // The answer carries the UNLV biology citation as its proof, at the
    // `consensus` trust tier for a .edu teaching-summary source.
    assert!(
        out.contains("landau.faculty.unlv.edu") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // The root is not a fungus part — a fungus has no roots; the mycelium is
    // root-LIKE, not a root — honest abstention, never a fabricated role.
    assert!(out.contains("\"abstained\":true"), "root abstains: {out}");
}
