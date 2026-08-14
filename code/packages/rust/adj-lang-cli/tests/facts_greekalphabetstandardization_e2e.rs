//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/greek-alphabet-standardization.adj`) driven
//! through the built CLI: a native `table` naming when the Euclidean Greek
//! alphabet became standard, decoded from a span already sitting unused
//! inside the SAME Wikipedia "Greek alphabet" quote `greek-alphabet.adj`
//! already cites -- a sibling to that table. Resolves binding-query recall
//! (both directions) with the source's citation, and abstains on an
//! alphabet variant (attic_alphabet) the cited span does not name -- 0
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
    let dir = std::env::temp_dir().join(format!("adjcli_greekalphabetstandardization_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/greek-alphabet-standardization.adj");
    std::fs::copy(&src, dir.join("greek-alphabet-standardization.adj"))
        .expect("copy shipped greek-alphabet-standardization.adj");
}

#[test]
fn greek_alphabet_standardization_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"greek-alphabet-standardization.adj\"\n\
         ? greek_alphabet_standardization(euclidean_alphabet, $Period)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"greek_alphabet_standardization(euclidean_alphabet, fourth_century_bc)\""),
        "the Euclidean alphabet became standard by the 4th century BC: {out}"
    );
    assert!(
        out.contains("wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wikipedia citation: {out}"
    );
}

#[test]
fn greek_alphabet_standardization_recalls_backward_from_a_bound_period() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"greek-alphabet-standardization.adj\"\n\
         ? greek_alphabet_standardization($AlphabetName, fourth_century_bc)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"greek_alphabet_standardization(euclidean_alphabet, fourth_century_bc)\""),
        "the period names the Euclidean alphabet: {out}"
    );
}

#[test]
fn greek_alphabet_standardization_abstains_honestly_on_attic_alphabet() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"greek-alphabet-standardization.adj\"\n\
         ? greek_alphabet_standardization(attic_alphabet, $Period)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the cited span names no period for attic_alphabet -- honest abstention: {out}"
    );
}
