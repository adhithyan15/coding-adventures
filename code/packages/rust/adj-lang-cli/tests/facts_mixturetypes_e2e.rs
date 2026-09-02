//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/mixture-types.adj`) driven through the built
//! CLI: a native `table` of mixture kind → the everyday example the source names
//! resolves binding-query recalls (forward and backward) with the source's
//! LibreTexts citation, and abstains on a kind not in the table (alloy) — 0
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsmix_{tag}_{}", std::process::id()));
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
fn chemistry_mixture_example_recall_binds_example_with_citation() {
    let dir = scratch("mixturetypes");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/mixture-types.adj");
    std::fs::copy(&src, dir.join("mixture-types.adj")).expect("copy shipped mixture-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"mixture-types.adj\"\n\
         ? mixture_example(colloid, $Example)\n\
         ? mixture_example(suspension, $Example)\n\
         ? mixture_example($Kind, vegetable_soup)\n\
         ? mixture_example(alloy, $Example)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THE WHOLE CITATION, anchored on its JSON key and closed by the
    // terminating quote. This sentence carries a qualifier, so a
    // truncation would silently drop meaning -- the defect issue #13916
    // shipped. Pinning a fragment narrows that hole rather than closing
    // it, because `contains` on a fragment cannot see what precedes or
    // follows it. See issue #13918.
    assert!(
        out.contains("\"source\":\"The salt water described above is homogeneous because the dissolved salt is evenly distributed throughout the entire salt water sample. … When the salt is thoroughly mixed into the water in this glass, it will form a solution. … Vegetable soup is a heterogeneous mixture. … The salad dressing in this bottle is a suspension. … Homogenized milk is a colloid.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A colloid's everyday example is milk; a suspension's is salad dressing —
    // the recalled example values (forward binds).
    assert!(out.contains("\"Example\":\"milk\""), "colloid -> milk: {out}");
    assert!(
        out.contains("\"Example\":\"salad_dressing\""),
        "suspension -> salad_dressing: {out}"
    );
    // The relation runs BACKWARD: bind the example vegetable_soup, recall the
    // kind the source classifies it as — heterogeneous.
    assert!(
        out.contains("\"Kind\":\"heterogeneous\""),
        "vegetable_soup -> heterogeneous (reverse recall): {out}"
    );
    // The answer carries the LibreTexts citation as its proof, at consensus trust
    // (a secondary teaching reference, honestly tiered).
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation at consensus trust: {out}"
    );
    // "alloy" is not in the table — honest abstention, never a fabricated example.
    assert!(out.contains("\"abstained\":true"), "alloy abstains: {out}");
}
