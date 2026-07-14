//! End-to-end test for the agriculture FACTS library
//! (`adj-facts-stdlib/agriculture/farm-animals.adj`) driven through the built CLI:
//! a native `table` of farm animal → product resolves a binding-query recall
//! with the source's citation, and abstains on a non-farm-animal — 0 model calls.

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
fn agriculture_farm_animals_recall_binds_product_with_citation() {
    let dir = scratch("farmanimals");
    // Copy the shipped agriculture table beside the entry program and import it.
    let src = facts_stdlib().join("agriculture/farm-animals.adj");
    std::fs::copy(&src, dir.join("farm-animals.adj")).expect("copy shipped farm-animals.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animals.adj\"\n\
         ? farm_animal_product(chicken, $Product)\n\
         ? farm_animal_product(sheep, $Product)\n\
         ? farm_animal_product(tiger, $Product)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A chicken gives eggs; a sheep gives wool — the recalled products.
    assert!(out.contains("\"Product\":\"eggs\""), "chicken → eggs: {out}");
    assert!(out.contains("\"Product\":\"wool\""), "sheep → wool: {out}");
    // The answer carries the Iowa State CFSPH citation (locator + trust) as its proof.
    assert!(
        out.contains("cfsph.iastate.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A tiger is not a farm animal — honest abstention, never a fabricated product.
    assert!(out.contains("\"abstained\":true"), "tiger abstains: {out}");
}
