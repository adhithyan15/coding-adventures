//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/heat-transfer-example.adj`) driven through
//! the built CLI: a native `table` recording, for each of the three
//! heat-transfer modes, the everyday EXAMPLE the same already-cited NASA
//! sentence gives -- a sibling decoding the parenthetical `(e.g., ...)`
//! half of each already-verified quote used by `heat-transfer.adj`.
//! Resolves forward and backward recall queries with the source's
//! citation, plus honest abstention on a non-heat-transfer word -- 0
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_heattransferexample_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/heat-transfer-example.adj");
    std::fs::copy(&src, dir.join("heat-transfer-example.adj"))
        .expect("copy shipped heat-transfer-example.adj");
}

#[test]
fn heat_transfer_example_recalls_conduction_example_with_citation() {
    let dir = scratch("conduction");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heat-transfer-example.adj\"\n\
         ? heat_transfer_example(conduction, $Example)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"heat_transfer_example(conduction, hot_chocolate_cup)\""),
        "conduction should recall its cited example: {out}"
    );
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn heat_transfer_example_backward_recalls_convection_for_warm_air() {
    let dir = scratch("convection");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heat-transfer-example.adj\"\n\
         ? heat_transfer_example($Mode, warm_air_rising)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"heat_transfer_example(convection, warm_air_rising)\""),
        "convection should be the only recalled warm-air-rising mode: {out}"
    );
    assert!(
        !out.contains("heat_transfer_example(radiation, warm_air_rising)"),
        "radiation's cited example is sunlight, not warm air rising: {out}"
    );
}

#[test]
fn heat_transfer_example_abstains_on_evaporation() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heat-transfer-example.adj\"\n\
         ? heat_transfer_example(evaporation, $ExampleEvap)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "evaporation is a phase change, not one of the three heat-transfer modes -- honest abstention expected: {out}"
    );
}
