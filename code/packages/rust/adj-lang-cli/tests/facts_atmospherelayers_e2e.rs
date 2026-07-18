//! End-to-end test for the earth-science ATMOSPHERE-LAYERS facts library
//! (`adj-facts-stdlib/earth-science/atmosphere-layers.adj`) driven through the
//! built CLI: a native `table` of atmosphere layer → distinctive feature
//! resolves a binding-query recall carrying the NASA "Earth's Atmosphere: A
//! Multi-layered Cake" citation, runs the relation backward (feature → layer),
//! and abstains on a word that is not one of the five layers (the `mantle`, a
//! layer of the solid Earth) — 0 model calls.

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
fn earth_science_atmosphere_layer_recall_binds_feature_with_citation() {
    let dir = scratch("atmospherelayers");
    // Copy the shipped atmosphere-layers table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/atmosphere-layers.adj");
    std::fs::copy(&src, dir.join("atmosphere-layers.adj"))
        .expect("copy shipped atmosphere-layers.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"atmosphere-layers.adj\"\n\
         ? atmosphere_layer_feature(troposphere, $Feature)\n\
         ? atmosphere_layer_feature(stratosphere, $Feature)\n\
         ? atmosphere_layer_feature(exosphere, $Feature)\n\
         ? atmosphere_layer_feature($Layer, auroras)\n\
         ? atmosphere_layer_feature(mantle, $Feature)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each layer to the distinctive feature NASA states.
    assert!(
        out.contains("\"Feature\":\"weather\""),
        "troposphere → weather: {out}"
    );
    assert!(
        out.contains("\"Feature\":\"ozone_layer\""),
        "stratosphere → ozone_layer: {out}"
    );
    assert!(
        out.contains("\"Feature\":\"highest\""),
        "exosphere → highest: {out}"
    );
    // The relation runs BACKWARD: the feature `auroras` recalls the thermosphere.
    assert!(
        out.contains("\"Layer\":\"thermosphere\""),
        "auroras → thermosphere (reverse recall): {out}"
    );
    // The answer carries the NASA (science.nasa.gov) citation as its proof, at
    // the authoritative trust tier of a primary U.S. government source.
    assert!(
        out.contains("science.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The `mantle` is a layer of the solid Earth, not the atmosphere — it is not
    // a row, so the recall abstains honestly, never a fabricated feature.
    assert!(
        out.contains("\"abstained\":true"),
        "ungrounded layer (mantle) abstains: {out}"
    );
}
