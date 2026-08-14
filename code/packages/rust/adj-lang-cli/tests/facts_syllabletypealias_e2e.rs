//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/syllable-type-alias.adj`) driven through
//! the built CLI: a native `table` naming the alternate name for the VCe
//! ("magic e") syllable pattern, decoded from a span already sitting
//! unused inside the SAME Reading Rockets quote `silent-e-word.adj`'s own
//! `source` field already carries -- a sibling to that table. Resolves
//! binding-query recall (both directions) with the source's citation, and
//! abstains on a real syllable type the same source article covers but
//! silent-e-word.adj does not table -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_syllabletypealias_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/syllable-type-alias.adj");
    std::fs::copy(&src, dir.join("syllable-type-alias.adj"))
        .expect("copy shipped syllable-type-alias.adj");
}

#[test]
fn syllable_type_alias_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-type-alias.adj\"\n\
         ? syllable_type_alias(vce_long_vowel, $Alias)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"syllable_type_alias(vce_long_vowel, magic_e)\""),
        "VCe syllables are also known as magic e syllables: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn syllable_type_alias_recalls_backward_from_a_bound_alias() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-type-alias.adj\"\n\
         ? syllable_type_alias($SyllableType, magic_e)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"syllable_type_alias(vce_long_vowel, magic_e)\""),
        "the alias names vce_long_vowel: {out}"
    );
}

#[test]
fn syllable_type_alias_abstains_honestly_on_closed_syllable() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-type-alias.adj\"\n\
         ? syllable_type_alias(closed_syllable, $Alias)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "closed_syllable is a real type from the same source article, but not one silent-e-word.adj tables -- honest abstention: {out}"
    );
}
