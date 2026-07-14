//! End-to-end test for the LANGUAGE FACTS library
//! (`adj-facts-stdlib/language/opposites.adj`) driven through the built CLI:
//! a native `table` of word → its opposite (antonym) resolves a binding-query
//! recall with the dictionary's citation, and abstains on a word with no
//! shipped opposite — 0 model calls.

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
fn language_opposites_recall_binds_antonym_with_citation() {
    let dir = scratch("opposites");
    // Copy the shipped opposites table beside the entry program and import it.
    let src = facts_stdlib().join("language/opposites.adj");
    std::fs::copy(&src, dir.join("opposites.adj")).expect("copy shipped opposites.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"opposites.adj\"\n\
         ? opposite(hot, $Word)\n\
         ? opposite(happy, $Word)\n\
         ? opposite(purple, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The opposite of hot is cold; the opposite of happy is sad — recalled.
    assert!(out.contains("\"Word\":\"cold\""), "hot → cold: {out}");
    assert!(out.contains("\"Word\":\"sad\""), "happy → sad: {out}");
    // The answer carries the Wiktionary citation as its proof, at consensus trust.
    assert!(
        out.contains("en.wiktionary.org/wiki/hot") && out.contains("\"trust\":\"consensus\""),
        "carries the dictionary citation: {out}"
    );
    // No opposite is shipped for `purple` — honest abstention, never invented.
    assert!(out.contains("\"abstained\":true"), "purple abstains: {out}");
}
