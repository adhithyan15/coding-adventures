//! End-to-end test for the agriculture FACTS library
//! (`adj-facts-stdlib/agriculture/farm-animal-product-texture.adj`)
//! driven through the built CLI: a native `table` naming the physical
//! texture descriptor a source already states for a farm animal's product,
//! decoded from a clause already sitting unused inside `farm-animals.adj`'s
//! own already-quoted per-row CFSPH provenance -- a sibling to that table
//! (and to `farm-animal-secondary-product.adj`,
//! `farm-animal-product-processing.adj`, and `farm-animal-product-use.adj`,
//! which decode the same rabbit clause's other axes: second product,
//! processing method, and use, respectively). Resolves binding-query recall
//! (both directions) with the source's citation, and abstains on a real,
//! already-tabled animal/product pair (goat, milk) whose own cited span
//! states a processing method, not a texture -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_farmanimalproducttexture_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("agriculture/farm-animal-product-texture.adj");
    std::fs::copy(&src, dir.join("farm-animal-product-texture.adj"))
        .expect("copy shipped farm-animal-product-texture.adj");
}

#[test]
fn farm_animal_product_texture_recalls_rabbit_wool_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-product-texture.adj\"\n\
         ? farm_animal_product_texture(rabbit, wool, $Texture)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"farm_animal_product_texture(rabbit, wool, soft)\""),
        "rabbit wool is soft: {out}"
    );
    assert!(
        out.contains("cfsph.iastate.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the CFSPH citation: {out}"
    );
}

#[test]
fn farm_animal_product_texture_recalls_backward_to_rabbit_wool() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-product-texture.adj\"\n\
         ? farm_animal_product_texture($Animal, $Product, soft)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"farm_animal_product_texture(rabbit, wool, soft)\""),
        "soft names rabbit's wool: {out}"
    );
}

#[test]
fn farm_animal_product_texture_abstains_honestly_on_goat_milk() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-product-texture.adj\"\n\
         ? farm_animal_product_texture(goat, milk, $Texture)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "goat's own cited span states a processing method for milk, not a texture -- honest abstention: {out}"
    );
}
