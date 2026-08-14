//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/insulin-glucagon-trigger.adj`) driven through
//! the built CLI: a native `table` recording the blood-glucose condition
//! that triggers insulin vs. glucagon release -- a sibling to the
//! already-shipped `hormone-glands.adj` (which only carries which gland
//! secretes each hormone), decoding the trigger-condition clause already
//! sitting unused inside two of that table's own per-hormone header
//! quotes. Resolves forward and backward recall queries with the source's
//! citation -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_insulinglucagontrigger_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/insulin-glucagon-trigger.adj");
    std::fs::copy(&src, dir.join("insulin-glucagon-trigger.adj"))
        .expect("copy shipped insulin-glucagon-trigger.adj");
}

#[test]
fn insulin_glucagon_trigger_recalls_insulin_as_high_glucose_with_citation() {
    let dir = scratch("insulin");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"insulin-glucagon-trigger.adj\"\n\
         ? secretion_trigger(insulin, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"secretion_trigger(insulin, high)\""),
        "insulin should be triggered by high blood glucose: {out}"
    );
    assert!(
        out.contains("seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NCI SEER citation: {out}"
    );
}

#[test]
fn insulin_glucagon_trigger_backward_recalls_glucagon_for_low_glucose() {
    let dir = scratch("low");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"insulin-glucagon-trigger.adj\"\n\
         ? secretion_trigger($Hormone, low)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"secretion_trigger(glucagon, low)\""),
        "glucagon should be the only recalled low-glucose trigger: {out}"
    );
    assert!(
        !out.contains("secretion_trigger(insulin, low)"),
        "insulin is triggered by high glucose, not low: {out}"
    );
}

#[test]
fn insulin_glucagon_trigger_abstains_on_a_hormone_with_no_glucose_trigger() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"insulin-glucagon-trigger.adj\"\n\
         ? secretion_trigger(thyroxine, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "thyroxine has no blood-glucose trigger in the cited span -- honest abstention expected: {out}"
    );
}
