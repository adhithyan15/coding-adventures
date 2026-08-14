//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/vertebrate-thermoregulation.adj`) driven
//! through the built CLI: a native `table` recording whether each of the
//! five vertebrate classes is ectothermic or endothermic -- a sibling to
//! the already-shipped `vertebrate-groups.adj` (which only carries one
//! distinctive body-covering trait per class), decoding the
//! ectothermic/endothermic clause already sitting unused inside that
//! table's own per-class header quotes. Resolves forward and backward
//! recall queries with the source's citation -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_vertebratethermoregulation_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/vertebrate-thermoregulation.adj");
    std::fs::copy(&src, dir.join("vertebrate-thermoregulation.adj"))
        .expect("copy shipped vertebrate-thermoregulation.adj");
}

#[test]
fn vertebrate_thermoregulation_recalls_bird_as_endothermic_with_citation() {
    let dir = scratch("bird");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vertebrate-thermoregulation.adj\"\n\
         ? vertebrate_thermoregulation(bird, $Type)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"vertebrate_thermoregulation(bird, endothermic)\""),
        "bird should recall as endothermic: {out}"
    );
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn vertebrate_thermoregulation_backward_recalls_all_ectothermic_classes() {
    let dir = scratch("ecto");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vertebrate-thermoregulation.adj\"\n\
         ? vertebrate_thermoregulation($Class, ectothermic)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for class in ["fish", "amphibian", "reptile"] {
        assert!(
            out.contains(&format!("\"term\":\"vertebrate_thermoregulation({class}, ectothermic)\"")),
            "{class} should be recalled as ectothermic: {out}"
        );
    }
    assert!(
        !out.contains("vertebrate_thermoregulation(bird, ectothermic)"),
        "bird is endothermic, not ectothermic: {out}"
    );
    assert!(
        !out.contains("vertebrate_thermoregulation(mammal, ectothermic)"),
        "mammal is endothermic, not ectothermic: {out}"
    );
}

#[test]
fn vertebrate_thermoregulation_covers_all_five_classes_without_abstention() {
    let dir = scratch("noabstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vertebrate-thermoregulation.adj\"\n\
         ? vertebrate_thermoregulation(fish, $T1)\n\
         ? vertebrate_thermoregulation(amphibian, $T2)\n\
         ? vertebrate_thermoregulation(reptile, $T3)\n\
         ? vertebrate_thermoregulation(bird, $T4)\n\
         ? vertebrate_thermoregulation(mammal, $T5)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        !out.contains("\"abstained\":true"),
        "all five vertebrate classes have a thermoregulation type on record -- no abstention expected: {out}"
    );
}
