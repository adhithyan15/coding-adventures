//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/fable-moral.adj`) driven through the built
//! CLI: a native `table` naming three classic fables and their own
//! narrator-stated morals, quoted verbatim from George Fyler Townsend's
//! translation of Aesop's Fables (Project Gutenberg). The FIRST literacy
//! slice in this loop's sweep to ground a whole-text comprehension
//! artifact (a fable's stated lesson) rather than a word-level phonics or
//! spelling fact. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fablemoral_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/fable-moral.adj");
    std::fs::copy(&src, dir.join("fable-moral.adj"))
        .expect("copy shipped fable-moral.adj");
}

#[test]
fn fable_moral_recall_binds_the_moral_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fable-moral.adj\"\n\
         ? fable_moral(tortoise_and_the_hare, $Moral)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("Slow but steady wins the race."),
        "the tortoise and the hare's moral is about slow and steady: {out}"
    );
    assert!(
        out.contains("gutenberg.org") && out.contains("\"trust\":\"authoritative\""),
        "carries the Project Gutenberg citation: {out}"
    );
}

#[test]
fn fable_moral_reverse_binds_every_fable_and_moral() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fable-moral.adj\"\n\
         ? fable_moral($F, $Moral)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for fable in ["tortoise_and_the_hare", "shepherds_boy_and_the_wolf", "boy_and_the_filberts"] {
        assert!(
            out.contains(&format!("\"F\":\"{fable}\"")),
            "{fable} should be one of the three bound fables: {out}"
        );
    }
}

#[test]
fn fable_moral_abstains_honestly_on_a_fable_whose_closing_line_is_dialogue() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fable-moral.adj\"\n\
         ? fable_moral(the_fox_and_the_crow, $Moral)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the fox and the crow's closing line is character dialogue, not a narrator-stated moral -- honest abstention, never invented: {out}"
    );
}
