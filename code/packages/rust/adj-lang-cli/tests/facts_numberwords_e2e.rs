//! End-to-end test for the mathematics (early numeracy) FACTS library
//! (`adj-facts-stdlib/mathematics/number-words.adj`) driven through the built
//! CLI: a native `table` of counting number → English word resolves a
//! binding-query recall with the source's citation, binds both directions, and
//! abstains on a number the source does not spell out — 0 model calls.

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
fn mathematics_number_words_recall_binds_word_with_citation() {
    let dir = scratch("numberwords");
    // Copy the shipped mathematics table beside the entry program and import it.
    let src = facts_stdlib().join("mathematics/number-words.adj");
    std::fs::copy(&src, dir.join("number-words.adj")).expect("copy shipped number-words.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"number-words.adj\"\n\
         ? number_word(5, $W)\n\
         ? number_word(10, $W)\n\
         ? number_word($N, five)\n\
         ? number_word(99, $W)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // 5 is said "five"; 10 is said "ten" — the recalled words.
    assert!(out.contains("\"W\":\"five\""), "5 → five: {out}");
    assert!(out.contains("\"W\":\"ten\""), "10 → ten: {out}");
    // The relation binds in reverse too: the word "five" recovers the number 5.
    assert!(out.contains("\"N\":\"5\""), "five → 5: {out}");
    // The answer carries the Cuemath citation as its proof. Cuemath is a secondary
    // math-education reference, so the trust tier is `consensus`, not `authoritative`.
    assert!(
        out.contains("cuemath.com/numbers/number-names-1-to-10")
            && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // 99 is beyond the ten numbers this table ships — honest abstention, never a
    // fabricated word.
    assert!(out.contains("\"abstained\":true"), "99 abstains: {out}");
}
