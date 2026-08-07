//! End-to-end test for the mathematics FACTS library
//! (`adj-facts-stdlib/mathematics/place-value.adj`) driven through the built CLI:
//! a native `table` of place-name → the number it is worth resolves a
//! binding-query recall with the source's citation, and abstains on a
//! non-place-name — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factspv_{tag}_{}", std::process::id()));
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
fn mathematics_place_value_recall_binds_value_with_citation() {
    let dir = scratch("placevalue");
    // Copy the shipped mathematics table beside the entry program and import it.
    let src = facts_stdlib().join("mathematics/place-value.adj");
    std::fs::copy(&src, dir.join("place-value.adj")).expect("copy shipped place-value.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"place-value.adj\"\n\
         ? place_value(hundreds, $V)\n\
         ? place_value(tens, $V)\n\
         ? place_value(dozen, $V)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The hundreds column is worth 100; the tens column is worth 10 — the recalled
    // values that compose straight into number decomposition.
    assert!(out.contains("\"V\":\"100\""), "hundreds -> 100: {out}");
    assert!(out.contains("\"V\":\"10\""), "tens -> 10: {out}");
    // The answer carries the Cuemath citation and its honest trust tier as proof.
    assert!(
        out.contains("cuemath.com/numbers/place-value") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation with locator and trust tier: {out}"
    );
    // "dozen" is not a place-value column — honest abstention, never a fabricated value.
    assert!(out.contains("\"abstained\":true"), "dozen abstains: {out}");
}
