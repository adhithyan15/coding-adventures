//! End-to-end test for the first grade-school FACTS library
//! (`adj-facts-stdlib/gradeschool/roman-numerals.adj`) driven through the built
//! CLI: a native `table` of Roman symbol → numeric value resolves a binding-query
//! recall with the source's citation, and abstains on a letter that is not a
//! Roman numeral — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsg_{tag}_{}", std::process::id()));
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
fn gradeschool_roman_recall_binds_symbol_value_with_citation() {
    let dir = scratch("roman");
    // Copy the shipped grade-school table beside the entry program and import it.
    let src = facts_stdlib().join("gradeschool/roman-numerals.adj");
    std::fs::copy(&src, dir.join("roman-numerals.adj")).expect("copy shipped roman-numerals.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"roman-numerals.adj\"\n\
         ? roman_numeral_value(x, $V)\n\
         ? roman_numeral_value(m, $V)\n\
         ? roman_numeral_value(q, $V)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // X is ten; M is a thousand — the recalled values.
    assert!(out.contains("\"V\":\"10\""), "x → 10: {out}");
    assert!(out.contains("\"V\":\"1000\""), "m → 1000: {out}");
    // The answer carries the Wikipedia citation as its proof.
    assert!(
        out.contains("en.wikipedia.org/wiki/Roman_numerals")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // `q` is not a Roman numeral — honest abstention, never a fabricated value.
    assert!(out.contains("\"abstained\":true"), "q abstains: {out}");
}
