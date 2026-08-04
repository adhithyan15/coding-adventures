//! End-to-end test for the MUSIC FACTS library
//! (`adj-facts-stdlib/music/solfege.adj`) driven through the built CLI: a native
//! `table` of the seven movable-do solfège syllables → their 1-based major-scale
//! degree resolves forward AND reverse binding-query recalls with the
//! encyclopedia's citation, and abstains on a chromatic syllable that has no
//! shipped row — 0 answer-time model calls.

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
fn music_solfege_recall_binds_degree_forward_and_reverse() {
    let dir = scratch("solfege");
    // Copy the shipped solfège table beside the entry program and import it.
    let src = facts_stdlib().join("music/solfege.adj");
    std::fs::copy(&src, dir.join("solfege.adj")).expect("copy shipped solfege.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"solfege.adj\"\n\
         ? solfege_degree(do, $N)\n\
         ? solfege_degree(sol, $N)\n\
         ? solfege_degree($S, 3)\n\
         ? solfege_degree(di, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward: do is the tonic (1st degree); sol is the dominant (5th).
    assert!(out.contains("\"N\":\"1\""), "do → 1: {out}");
    assert!(out.contains("\"N\":\"5\""), "sol → 5: {out}");
    // Reverse: the third scale degree is mi (binds the other column).
    assert!(out.contains("\"S\":\"mi\""), "degree 3 → mi: {out}");
    // The answer carries the Wikipedia Solfège citation as its proof, at consensus trust.
    assert!(
        out.contains("wikipedia.org/wiki/Solf") && out.contains("\"trust\":\"consensus\""),
        "carries the encyclopedia citation: {out}"
    );
    // `di` is a chromatic (raised-do) syllable, not one of the 7 diatonic rows —
    // honest abstention, never invented.
    assert!(out.contains("\"abstained\":true"), "chromatic syllable di abstains: {out}");
}
