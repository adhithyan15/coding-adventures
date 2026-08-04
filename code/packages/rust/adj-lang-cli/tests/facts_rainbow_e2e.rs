//! End-to-end test for the optics FACTS library
//! (`adj-facts-stdlib/optics/rainbow-colors.adj`) driven through the built CLI:
//! a native `table` of rainbow-color → order-in-the-visible-spectrum resolves a
//! binding-query recall with the NASA citation, and abstains on a non-spectral
//! color — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsr_{tag}_{}", std::process::id()));
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
fn optics_rainbow_recall_binds_color_order_with_citation() {
    let dir = scratch("rainbow");
    // Copy the shipped optics table beside the entry program and import it.
    let src = facts_stdlib().join("optics/rainbow-colors.adj");
    std::fs::copy(&src, dir.join("rainbow-colors.adj")).expect("copy shipped rainbow-colors.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"rainbow-colors.adj\"\n\
         ? rainbow_color_order(green, $N)\n\
         ? rainbow_color_order(violet, $N)\n\
         ? rainbow_color_order(brown, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Green is the fourth color; violet is the seventh — the recalled orders.
    assert!(out.contains("\"N\":\"4\""), "green → 4: {out}");
    assert!(out.contains("\"N\":\"7\""), "violet → 7: {out}");
    // The answer carries the NASA locator + trust tier as its proof.
    assert!(
        out.contains("imagine.gsfc.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Brown is not a spectral color — honest abstention, never a fabricated order.
    assert!(out.contains("\"abstained\":true"), "brown abstains: {out}");
}
