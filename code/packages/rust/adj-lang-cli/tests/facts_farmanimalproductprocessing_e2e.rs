//! End-to-end test for the agriculture FACTS library
//! (`adj-facts-stdlib/agriculture/farm-animal-product-processing.adj`)
//! driven through the built CLI: a native `table` naming the processing
//! method a source already states for a farm animal's product, decoded
//! from a clause already sitting unused inside `farm-animals.adj`'s own
//! already-quoted per-row CFSPH provenance -- a sibling to that table
//! (and to `farm-animal-secondary-product.adj`, whose own header already
//! flags this exact goat clause as a processing note rather than a second
//! product). Resolves binding-query recall (both directions) with the
//! source's citation, and abstains on a real, already-tabled animal/product
//! pair (sheep, wool) whose own cited span states no processing method --
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
    let dir = std::env::temp_dir().join(format!("adjcli_farmanimalproductprocessing_{tag}_{}", std::process::id()));
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

fn place_lib(dir: &Path) {
    let src = facts_stdlib().join("agriculture/farm-animal-product-processing.adj");
    std::fs::copy(&src, dir.join("farm-animal-product-processing.adj"))
        .expect("copy shipped farm-animal-product-processing.adj");
}

#[test]
fn farm_animal_product_processing_recalls_goat_milk_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-product-processing.adj\"\n\
         ? farm_animal_product_processing(goat, milk, $Processing)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"farm_animal_product_processing(goat, milk, pasteurized)\""),
        "goat milk may be pasteurized: {out}"
    );
    assert!(
        out.contains("cfsph.iastate.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the CFSPH citation: {out}"
    );
}

#[test]
fn farm_animal_product_processing_recalls_backward_to_goat_milk() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-product-processing.adj\"\n\
         ? farm_animal_product_processing($Animal, $Product, pasteurized)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"farm_animal_product_processing(goat, milk, pasteurized)\""),
        "pasteurized names goat's milk: {out}"
    );
}

#[test]
fn farm_animal_product_processing_abstains_honestly_on_sheep_wool() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-product-processing.adj\"\n\
         ? farm_animal_product_processing(sheep, wool, $Processing)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "sheep's own cited span states no processing method for wool -- honest abstention: {out}"
    );
}
