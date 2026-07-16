//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/basic-tastes.adj`) driven through the built CLI:
//! a native `table` of taste → is-basic-taste membership resolves a binding-query
//! recall with the source's citation, and abstains on a non-taste — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn biology_basic_tastes_recall_binds_membership_with_citation() {
    let dir = scratch("tastes");
    // Copy the shipped basic-tastes table beside the entry program and import it.
    let src = facts_stdlib().join("biology/basic-tastes.adj");
    std::fs::copy(&src, dir.join("basic-tastes.adj")).expect("copy shipped basic-tastes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"basic-tastes.adj\"\n\
         ? basic_taste(sweet, $Is)\n\
         ? basic_taste(umami, $Is)\n\
         ? basic_taste(spicy, $Is)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Sweet and umami are both among the five basic tastes — the recalled membership.
    // Two binds, both `yes`; `.matches` counts occurrences of the bound value.
    assert!(out.contains("\"Is\":\"yes\""), "sweet/umami → yes: {out}");
    assert!(
        out.matches("\"Is\":\"yes\"").count() >= 2,
        "both sweet and umami bind to yes: {out}"
    );
    // The answer carries the NIDCD (.gov) citation as its proof, at authoritative trust.
    assert!(
        out.contains("nidcd.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Spicy is a heat/pain sensation, not one of the five basic tastes —
    // honest abstention, never a fabricated `yes`.
    assert!(out.contains("\"abstained\":true"), "spicy abstains: {out}");
}
