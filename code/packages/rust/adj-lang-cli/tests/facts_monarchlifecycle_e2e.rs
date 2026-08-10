//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/monarch-life-cycle.adj`) driven through the
//! built CLI: a native `table` naming each of the monarch butterfly's four
//! life-cycle stages as a number, quoted verbatim from the USDA Forest
//! Service's "Monarch Butterfly Biology" page -- a genuinely NEW content
//! shape for this loop's science sweep: not an instrument-measures-quantity
//! table (like `chemistry/measuring-tools.adj`/
//! `meteorology/weather-instruments.adj`) and not an ordinal-WORD bridge
//! (like the four already-shipped season/planet/moon-phase/mitosis
//! libraries) -- a plain numbered life-cycle-stage recall table, applying
//! `earth-science/water-cycle.adj`'s established shape to a biological
//! process. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_monarchlifecycle_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/monarch-life-cycle.adj");
    std::fs::copy(&src, dir.join("monarch-life-cycle.adj"))
        .expect("copy shipped monarch-life-cycle.adj");
}

#[test]
fn monarch_life_stage_recall_binds_the_order_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"monarch-life-cycle.adj\"\n\
         ? monarch_life_stage(pupa, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"N\":\"3\""),
        "pupa is the third life stage: {out}"
    );
    assert!(
        out.contains("fs.usda.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USDA citation: {out}"
    );
}

#[test]
fn monarch_life_stage_reverse_binds_the_stage_for_that_order() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"monarch-life-cycle.adj\"\n\
         ? monarch_life_stage($S, 1)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"egg\""),
        "egg is the first life stage: {out}"
    );
}

#[test]
fn monarch_life_stage_abstains_honestly_on_a_non_monarch_stage() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"monarch-life-cycle.adj\"\n\
         ? monarch_life_stage(nymph, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"nymph\" is the incomplete-metamorphosis term (e.g. grasshoppers), not one of the monarch's four stages -- honest abstention, never invented: {out}"
    );
}
