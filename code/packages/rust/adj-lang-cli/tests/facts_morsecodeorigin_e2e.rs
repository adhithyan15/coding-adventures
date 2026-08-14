//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/morse-code-origin.adj`) driven through the
//! built CLI: a native `table` naming who proposed the code International
//! Morse code was derived from, and when, decoded from a span already
//! sitting unused inside the SAME Wikipedia "Morse code" quote
//! `morse-code.adj` and `morse-code-standard.adj` already cite -- a sibling
//! to both. Resolves binding-query recall (both directions) with the
//! source's citation, and abstains on a code system (american_morse_code)
//! the cited span does not name -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_morsecodeorigin_{tag}_{}", std::process::id()));
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

fn place_lib(dir: &Path) {
    let src = facts_stdlib().join("language/morse-code-origin.adj");
    std::fs::copy(&src, dir.join("morse-code-origin.adj"))
        .expect("copy shipped morse-code-origin.adj");
}

#[test]
fn morse_code_origin_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"morse-code-origin.adj\"\n\
         ? morse_code_origin(international_morse_code, $Originator, $Year)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"morse_code_origin(international_morse_code, friedrich_gerke, 1848)\""),
        "international Morse code derives from Friedrich Gerke's 1848 proposal: {out}"
    );
    assert!(
        out.contains("wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wikipedia citation: {out}"
    );
}

#[test]
fn morse_code_origin_recalls_backward_from_a_bound_originator() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"morse-code-origin.adj\"\n\
         ? morse_code_origin($CodeSystem, friedrich_gerke, $Year)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"morse_code_origin(international_morse_code, friedrich_gerke, 1848)\""),
        "Friedrich Gerke's proposal names international Morse code: {out}"
    );
}

#[test]
fn morse_code_origin_abstains_honestly_on_american_morse_code() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"morse-code-origin.adj\"\n\
         ? morse_code_origin(american_morse_code, $Originator, $Year)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the cited span names no originator/year for american_morse_code -- honest abstention: {out}"
    );
}
