//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/seed-parts.adj`) driven through the built CLI:
//! a native `table` of seed parts → the defining role / function the source
//! states resolves binding-query recalls (forward AND backward) with the
//! source's USDA Forest Service citation, and abstains on a word that is not one
//! of these seed parts (the petal) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsp_{tag}_{}", std::process::id()));
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
fn biology_seed_parts_recall_binds_role_with_citation() {
    let dir = scratch("seedparts");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/seed-parts.adj");
    std::fs::copy(&src, dir.join("seed-parts.adj")).expect("copy shipped seed-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"seed-parts.adj\"\n\
         ? seed_part_role(seed_coat, $Role)\n\
         ? seed_part_role(cotyledon, $Role)\n\
         ? seed_part_role(embryo, $Role)\n\
         ? seed_part_role(radicle, $Role)\n\
         ? seed_part_role($Part, food_storage)\n\
         ? seed_part_role(petal, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The seedcoat is the seed-covering structure, the cotyledons are the site
    // of food storage, the embryo grows into a miniature plant, the radicle is
    // the rudimentary root — the recalled roles (forward binds).
    assert!(
        out.contains("\"Role\":\"covering\""),
        "seed_coat → covering: {out}"
    );
    assert!(
        out.contains("\"Role\":\"food_storage\""),
        "cotyledon → food_storage: {out}"
    );
    assert!(
        out.contains("\"Role\":\"miniature_plant\""),
        "embryo → miniature_plant: {out}"
    );
    assert!(
        out.contains("\"Role\":\"rudimentary_root\""),
        "radicle → rudimentary_root: {out}"
    );
    // The relation runs BACKWARD: bind the role `food_storage`, recall its seed
    // part.
    assert!(
        out.contains("\"Part\":\"cotyledon\""),
        "food_storage → cotyledon (reverse recall): {out}"
    );
    // The answer carries the USDA Forest Service citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government seed-biology
    // publication.
    assert!(
        out.contains("fs.usda.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The petal is a flower part, not a part of the seed — honest abstention,
    // never a fabricated role.
    assert!(out.contains("\"abstained\":true"), "petal abstains: {out}");
}
