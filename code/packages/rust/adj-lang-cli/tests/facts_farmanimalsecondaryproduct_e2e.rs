//! End-to-end test for the agriculture FACTS library
//! (`adj-facts-stdlib/agriculture/farm-animal-secondary-product.adj`)
//! driven through the built CLI: a native `table` naming the second
//! product the SAME CFSPH source spans already state for three farm
//! animals -- a sibling to the already-shipped `farm-animals.adj` (which
//! only carries each animal's single, source-stated product), decoding
//! the second-product half of spans already sitting unused inside that
//! table's own per-row provenance block. Resolves binding-query recall
//! (both directions) with the source's citation, and abstains on an
//! animal (goat) the cited spans give no second product for -- 0 model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_farmanimalsecondaryproduct_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("agriculture/farm-animal-secondary-product.adj");
    std::fs::copy(&src, dir.join("farm-animal-secondary-product.adj"))
        .expect("copy shipped farm-animal-secondary-product.adj");
}

#[test]
fn farm_animal_secondary_product_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-secondary-product.adj\"\n\
         ? farm_animal_secondary_product(chicken, $Product)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"farm_animal_secondary_product(chicken, meat)\""),
        "a chicken also gives meat: {out}"
    );
    assert!(
        out.contains("cfsph.iastate.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the CFSPH citation: {out}"
    );
}

#[test]
fn farm_animal_secondary_product_recalls_backward_from_a_bound_product() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-secondary-product.adj\"\n\
         ? farm_animal_secondary_product($Animal, milk)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"farm_animal_secondary_product(sheep, milk)\""),
        "milk names sheep as a secondary producer: {out}"
    );
}

#[test]
fn farm_animal_secondary_product_abstains_honestly_on_goat() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-secondary-product.adj\"\n\
         ? farm_animal_secondary_product(goat, $Product)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "goat has no secondary product in the cited span -- honest abstention: {out}"
    );
}
