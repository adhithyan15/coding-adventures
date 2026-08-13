//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/earth-layer-thickness.adj`) driven through the
//! built CLI: a native `table` naming the THICKNESS (in km) USGS states for
//! an Earth internal layer, where the source states one -- a sibling to the
//! already-shipped `earth-layers.adj` (which only carries ONE physical-state
//! fact per layer), decoding spans already sitting unused inside that
//! table's own header and provenance block. Resolves binding-query recall
//! (both directions) with the source's citation, and abstains on a layer
//! (the crust) the cited spans give no thickness figure for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_earthlayerthickness_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geology/earth-layer-thickness.adj");
    std::fs::copy(&src, dir.join("earth-layer-thickness.adj"))
        .expect("copy shipped earth-layer-thickness.adj");
}

#[test]
fn earth_layer_thickness_recalls_all_three_figures_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"earth-layer-thickness.adj\"\n\
         ? earth_layer_thickness(mantle, $ThicknessKm)\n\
         ? earth_layer_thickness(outer_core, $ThicknessKm)\n\
         ? earth_layer_thickness(inner_core, $ThicknessKm)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"earth_layer_thickness(mantle, 2900)\""),
        "mantle is 2900 km thick: {out}"
    );
    assert!(
        out.contains("\"term\":\"earth_layer_thickness(outer_core, 2200)\""),
        "outer_core is 2200 km thick: {out}"
    );
    assert!(
        out.contains("\"term\":\"earth_layer_thickness(inner_core, 1250)\""),
        "inner_core is 1250 km thick: {out}"
    );
    assert!(
        out.contains("pubs.usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}

#[test]
fn earth_layer_thickness_recalls_backward_from_a_bound_thickness() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"earth-layer-thickness.adj\"\n\
         ? earth_layer_thickness($Layer, 1250)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"earth_layer_thickness(inner_core, 1250)\""),
        "1250 km names the inner_core: {out}"
    );
}

#[test]
fn earth_layer_thickness_abstains_honestly_on_the_crust() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"earth-layer-thickness.adj\"\n\
         ? earth_layer_thickness(crust, $ThicknessKm)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the crust's cited span gives no thickness figure -- honest abstention: {out}"
    );
}
