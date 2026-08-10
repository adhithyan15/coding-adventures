//! End-to-end test for the LANGUAGE FACTS library
//! (`adj-facts-stdlib/language/word-families.adj`) driven through the built
//! CLI: a native `table` of the "-an" word family's core CVC members plus a
//! `rule` DERIVING `rhymes_with($Word1, $Word2)` from shared family
//! membership — the first `rule`-based (composed, not just recalled) literacy
//! fact in this loop's curriculum sweep, mirroring the discipline
//! `shape-composition.adj` already established for geometry. Honest
//! abstention on a word with no shipped row — 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_wordfam_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/word-families.adj");
    std::fs::copy(&src, dir.join("word-families.adj")).expect("copy shipped word-families.adj");
}

#[test]
fn word_family_recall_binds_the_family_directly() {
    let dir = scratch("family");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"word-families.adj\"\n\
         ? word_family(pan, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"F\":\"an\""), "pan is in the -an family: {out}");
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn rhymes_with_is_derived_from_shared_family_membership() {
    let dir = scratch("rhymes");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"word-families.adj\"\n\
         ? rhymes_with(pan, $W)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Every other member of the -an family rhymes with "pan" (plus itself, trivially).
    for w in ["pan", "fan", "ran", "man", "tan", "van"] {
        assert!(
            out.contains(&format!("\"W\":\"{w}\"")),
            "pan rhymes with {w}: {out}"
        );
    }
    // The derivation composes TWO citations: the rule's general principle AND
    // the table's specific family-membership fact, both from the same source.
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the rhyme is DERIVED, not a direct table row -- both a rule step and fact steps appear: {out}"
    );
    assert!(
        out.contains("look alike at the end if they sound alike at the end"),
        "the rule carries its own verbatim definitional citation: {out}"
    );
}

#[test]
fn rhymes_with_isolates_a_second_family_with_the_same_unmodified_rule() {
    let dir = scratch("second_family");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"word-families.adj\"\n\
         ? rhymes_with(cat, $W)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Every other member of the -at family rhymes with "cat" (plus itself).
    for w in ["cat", "bat", "fat", "sat", "rat", "pat", "mat", "hat"] {
        assert!(
            out.contains(&format!("\"W\":\"{w}\"")),
            "cat rhymes with {w}: {out}"
        );
    }
    // No cross-contamination: no -an family member should appear as a rhyme of "cat".
    for w in ["pan", "fan", "ran", "man", "tan", "van"] {
        assert!(
            !out.contains(&format!("\"W\":\"{w}\"")),
            "-an member {w} must NOT rhyme with -at member cat: {out}"
        );
    }
}

#[test]
fn word_family_abstains_honestly_on_an_unshipped_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"word-families.adj\"\n\
         ? word_family(dog, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"dog\" has no shipped row -- honest abstention, never invented: {out}"
    );
}
