//! End-to-end test for the music FACTS library
//! (`adj-facts-stdlib/music/solfege-alt-name.adj`) driven through the
//! built CLI: a native `table` naming the alternate spelling/name the
//! SAME Wikipedia "Solfège" source span already states for two of the
//! seven solfège syllables -- a sibling to the already-shipped
//! `solfege.adj` (which only carries each syllable's scale degree),
//! decoding the alternate-name parentheticals already sitting unused
//! inside that table's own `source` field. Resolves binding-query recall
//! (both directions) with the source's citation, and abstains on a
//! syllable (mi) the cited span gives no alternate for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_solfegealtname_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("music/solfege-alt-name.adj");
    std::fs::copy(&src, dir.join("solfege-alt-name.adj"))
        .expect("copy shipped solfege-alt-name.adj");
}

#[test]
fn solfege_alt_name_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solfege-alt-name.adj\"\n\
         ? solfege_alt_name(do, $Alt)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"solfege_alt_name(do, doh)\""),
        "do is also spelt doh: {out}"
    );
    assert!(
        out.contains("wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wikipedia citation: {out}"
    );
}

#[test]
fn solfege_alt_name_recalls_backward_from_a_bound_alt_name() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solfege-alt-name.adj\"\n\
         ? solfege_alt_name($Syllable, si)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"solfege_alt_name(ti, si)\""),
        "si names the ti syllable: {out}"
    );
}

#[test]
fn solfege_alt_name_abstains_honestly_on_mi() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solfege-alt-name.adj\"\n\
         ? solfege_alt_name(mi, $Alt)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "mi's own cited span states no alternate name -- honest abstention: {out}"
    );
}
