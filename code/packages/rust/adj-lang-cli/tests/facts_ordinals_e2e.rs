//! End-to-end test for the mathematics ordinal-numbers FACTS library
//! (`adj-facts-stdlib/mathematics/ordinal-numbers.adj`): a native `table` of
//! counting-number → ordinal-word resolves forward AND reverse binding queries
//! with the EF citation, and abstains on an unlisted number — 0 model calls.

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
fn mathematics_ordinals_recall_binds_word_forward_and_reverse() {
    let dir = scratch("ordinals");
    let src = facts_stdlib().join("mathematics/ordinal-numbers.adj");
    std::fs::copy(&src, dir.join("ordinal-numbers.adj")).expect("copy shipped ordinal-numbers.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"ordinal-numbers.adj\"\n\
         ? ordinal_number(1, $Word)\n\
         ? ordinal_number($N, third)\n\
         ? ordinal_number(11, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward: the ordinal word for 1 is "first".
    assert!(out.contains("\"Word\":\"first\""), "1 -> first: {out}");
    // Reverse: the number whose ordinal is "third" is 3 (binds the other column).
    assert!(out.contains("\"N\":\"3\""), "third -> 3: {out}");
    // The answer carries the EF citation (locator + trust) as its proof.
    assert!(
        out.contains("ef.edu") && out.contains("\"trust\":\"consensus\""),
        "carries the EF citation: {out}"
    );
    // 11 is not a listed row — honest abstention, no fabricated ordinal.
    assert!(out.contains("\"abstained\":true"), "11 abstains: {out}");
}
