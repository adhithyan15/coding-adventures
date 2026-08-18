//! End-to-end test for the agriculture FACTS library
//! (`adj-facts-stdlib/agriculture/farm-animal-product-use.adj`)
//! driven through the built CLI: a native `table` naming the use a source
//! already states for a farm animal's product, decoded from a clause
//! already sitting unused inside `farm-animals.adj`'s own already-quoted
//! per-row CFSPH provenance -- a sibling to that table (and to
//! `farm-animal-secondary-product.adj` and
//! `farm-animal-product-processing.adj`, whose own headers already flag
//! this exact rabbit clause as neither a second product nor a processing
//! method). Resolves binding-query recall (both directions) with the
//! source's citation, and abstains on a real, already-tabled animal/product
//! pair (goat, milk) whose own cited span states a processing method, not a
//! use -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_farmanimalproductuse_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("agriculture/farm-animal-product-use.adj");
    std::fs::copy(&src, dir.join("farm-animal-product-use.adj"))
        .expect("copy shipped farm-animal-product-use.adj");
}

#[test]
fn farm_animal_product_use_recalls_rabbit_wool_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-product-use.adj\"\n\
         ? farm_animal_product_use(rabbit, wool, $Use)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"farm_animal_product_use(rabbit, wool, fiber_arts)\""),
        "rabbit wool is used for fiber arts: {out}"
    );
    assert!(
        out.contains("cfsph.iastate.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the CFSPH citation: {out}"
    );
}

#[test]
fn farm_animal_product_use_recalls_backward_to_rabbit_wool() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-product-use.adj\"\n\
         ? farm_animal_product_use($Animal, $Product, fiber_arts)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"farm_animal_product_use(rabbit, wool, fiber_arts)\""),
        "fiber_arts names rabbit's wool: {out}"
    );
}

#[test]
fn farm_animal_product_use_abstains_honestly_on_goat_milk() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-product-use.adj\"\n\
         ? farm_animal_product_use(goat, milk, $Use)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "goat's own cited span states a processing method for milk, not a use -- honest abstention: {out}"
    );
}
