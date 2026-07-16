//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/lung-lobes.adj`) driven through the built CLI:
//! a native `table` of lung → lobe count resolves a binding-query recall with
//! the source's NCI SEER Training citation, runs the relation backward
//! (count → lung), and abstains on a non-lung (the trachea) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factslung_{tag}_{}", std::process::id()));
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
fn anatomy_lung_lobes_recall_binds_count_with_citation() {
    let dir = scratch("lunglobes");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/lung-lobes.adj");
    std::fs::copy(&src, dir.join("lung-lobes.adj")).expect("copy shipped lung-lobes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"lung-lobes.adj\"\n\
         ? lung_lobe_count(right_lung, $N)\n\
         ? lung_lobe_count(left_lung, $N)\n\
         ? lung_lobe_count($L, 3)\n\
         ? lung_lobe_count(trachea, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The right lung has three lobes; the left lung has two — the recalled
    // counts, each a plain number.
    assert!(out.contains("\"N\":\"3\""), "right_lung → 3: {out}");
    assert!(out.contains("\"N\":\"2\""), "left_lung → 2: {out}");
    // The relation runs backward: the count 3 recalls the right lung.
    assert!(
        out.contains("\"L\":\"right_lung\""),
        "3 → right_lung (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training citation as its proof.
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The trachea is not a lung — honest abstention, never a fabricated count.
    assert!(out.contains("\"abstained\":true"), "unknown lung abstains: {out}");
}
