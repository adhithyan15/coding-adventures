//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/rock-type-formation-component.adj`) driven
//! through the built CLI: a native `table` recording, for two of the three
//! basic rock types already tabled in `rock-type.adj`, each individual
//! material or agent an already-cited USGS sentence lists as forming it --
//! a sibling decoding each listed item as its own row instead of folding
//! the whole clause into one compound `formation_process` atom. Resolves
//! forward (multi-answer) and backward recall queries with the source's
//! citation, plus honest abstention on igneous (whose cited span names
//! only one material) -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rocktypeformationcomponent_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geology/rock-type-formation-component.adj");
    std::fs::copy(&src, dir.join("rock-type-formation-component.adj"))
        .expect("copy shipped rock-type-formation-component.adj");
}

#[test]
fn rock_type_formation_component_recalls_metamorphic_components_with_citation() {
    let dir = scratch("metamorphic");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rock-type-formation-component.adj\"\n\
         ? rock_type_formation_component(metamorphic, $Component)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"rock_type_formation_component(metamorphic, high_heat)\""),
        "metamorphic should recall high_heat: {out}"
    );
    assert!(
        out.contains("\"term\":\"rock_type_formation_component(metamorphic, high_pressure)\""),
        "metamorphic should recall high_pressure: {out}"
    );
    assert!(
        out.contains("\"term\":\"rock_type_formation_component(metamorphic, hot_mineral_rich_fluids)\""),
        "metamorphic should recall hot_mineral_rich_fluids: {out}"
    );
    assert!(
        out.contains("usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}

#[test]
fn rock_type_formation_component_backward_recalls_metamorphic_for_fluids() {
    let dir = scratch("fluids");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rock-type-formation-component.adj\"\n\
         ? rock_type_formation_component($Rock, hot_mineral_rich_fluids)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"rock_type_formation_component(metamorphic, hot_mineral_rich_fluids)\""),
        "metamorphic should be the only recalled rock for hot mineral-rich fluids: {out}"
    );
}

#[test]
fn rock_type_formation_component_abstains_on_igneous() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rock-type-formation-component.adj\"\n\
         ? rock_type_formation_component(igneous, $ComponentIgneous)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "igneous's cited span names only one material, no listed alternatives -- honest abstention expected: {out}"
    );
}
