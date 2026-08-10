//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/compound-word-spelling-example.adj`) driven
//! through the built CLI: a native `table` naming which compound words a
//! primary literacy source uses as a beginner multisyllable-spelling
//! teaching example, quoted verbatim from Reading Rockets' "How Spelling
//! Supports Reading" article. The FIRST literacy slice in this loop's sweep
//! to move beyond CCSS RF.K.2 (rhyming/syllables/onset-rime/initial-sound/
//! phoneme-substitution, all five parts of which are now shipped) into a
//! SPELLING pattern. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_compoundwordspellingexample_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/compound-word-spelling-example.adj");
    std::fs::copy(&src, dir.join("compound-word-spelling-example.adj"))
        .expect("copy shipped compound-word-spelling-example.adj");
}

#[test]
fn compound_word_spelling_example_recall_binds_the_teaching_use_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"compound-word-spelling-example.adj\"\n\
         ? compound_word_spelling_example(catfish, $Use)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Use\":\"beginner_multisyllable_spelling\""),
        "catfish is a beginner multisyllable-spelling example: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn compound_word_spelling_example_reverse_binds_every_example_word() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"compound-word-spelling-example.adj\"\n\
         ? compound_word_spelling_example($W, beginner_multisyllable_spelling)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for word in ["catfish", "hotdog", "playground", "yellowtail"] {
        assert!(
            out.contains(&format!("\"W\":\"{word}\"")),
            "{word} should be one of the four bound example words: {out}"
        );
    }
}

#[test]
fn compound_word_spelling_example_abstains_honestly_on_an_uncited_compound() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"compound-word-spelling-example.adj\"\n\
         ? compound_word_spelling_example(cupcake, $Use)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "cupcake is a real compound word but not one this source names -- honest abstention, never invented: {out}"
    );
}
