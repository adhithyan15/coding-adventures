//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/plant-life-cycle.adj`) driven through the
//! built CLI: a native `table` naming each of a flowering plant's three
//! early life-cycle stages as a number, quoted verbatim from Ducksters'
//! "Flowering Plants" (Biology for Kids) article -- a sibling library to
//! the already-shipped `monarch-life-cycle.adj`/`frog-life-cycle.adj`,
//! applying the SAME plain numbered life-cycle-stage recall shape to a
//! different organism. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_plantlifecycle_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/plant-life-cycle.adj");
    std::fs::copy(&src, dir.join("plant-life-cycle.adj"))
        .expect("copy shipped plant-life-cycle.adj");
}

#[test]
fn plant_life_stage_recall_binds_the_order_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-life-cycle.adj\"\n\
         ? plant_life_stage(germination, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"N\":\"2\""),
        "germination is the second life stage: {out}"
    );
    assert!(
        out.contains("ducksters.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Ducksters citation: {out}"
    );
}

#[test]
fn plant_life_stage_reverse_binds_the_stage_for_that_order() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-life-cycle.adj\"\n\
         ? plant_life_stage($S, 1)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"seed\""),
        "seed is the first life stage: {out}"
    );
}

#[test]
fn plant_life_stage_abstains_honestly_on_an_untabled_stage_name() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-life-cycle.adj\"\n\
         ? plant_life_stage(flowering, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"flowering\" is a real later stage the source describes but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
