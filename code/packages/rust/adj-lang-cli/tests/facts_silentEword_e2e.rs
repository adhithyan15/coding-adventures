//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/silent-e-word.adj`) driven through the
//! built CLI: a native `table` naming which words a primary literacy
//! source uses as examples of the "silent e" / "magic e" (VCe) spelling
//! pattern, quoted verbatim from Reading Rockets' "Six Syllable Types"
//! article. The SECOND literacy slice in this loop's sweep to move beyond
//! CCSS RF.K.2 (following `compound-word-spelling-example.adj`'s
//! precedent) into ANOTHER spelling pattern. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_silentEword_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/silent-e-word.adj");
    std::fs::copy(&src, dir.join("silent-e-word.adj"))
        .expect("copy shipped silent-e-word.adj");
}

#[test]
fn silent_e_word_recall_binds_the_syllable_type_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-e-word.adj\"\n\
         ? silent_e_word(wake, $SyllableType)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"SyllableType\":\"vce_long_vowel\""),
        "wake is a VCe long-vowel example: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn silent_e_word_reverse_binds_every_example_word() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-e-word.adj\"\n\
         ? silent_e_word($W, vce_long_vowel)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for word in ["wake", "whale", "while", "yoke", "yore", "rude", "hare"] {
        assert!(
            out.contains(&format!("\"W\":\"{word}\"")),
            "{word} should be one of the seven bound example words: {out}"
        );
    }
}

#[test]
fn silent_e_word_abstains_honestly_on_an_uncited_vce_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-e-word.adj\"\n\
         ? silent_e_word(snake, $SyllableType)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "snake is a real VCe word but not one this source names -- honest abstention, never invented: {out}"
    );
}

const SILENT_E_WORD_PIN: &str = r#""bindings":{"SyllableType":"vce_long_vowel"},"citations":[{"source":"Also known as “magic e” syllable patterns, VCe syllables contain long vowels spelled with a single letter, followed by a single consonant, and a silent e. Examples of VCe syllables are found in wake, whale, while, yoke, yore, rude, and hare.","locator":"https://www.readingrockets.org/topics/spelling-and-word-study/articles/six-syllable-types","trust":"consensus""#;

#[test]
fn silent_e_word_citation_keeps_the_pages_curly_double_quotes() {
    let dir = scratch("glyph");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-e-word.adj\"
? silent_e_word(wake, $SyllableType)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The page renders CURLY DOUBLE quotes around "magic e"; the shipped
    // citation had ASCII ones. This class was invisible to every glyph screen
    // in this effort: they all keyed on an ASCII APOSTROPHE, and this value
    // contains none. The same sentence was flattened in two libraries.
    assert!(
        out.contains(SILENT_E_WORD_PIN),
        "the silent-e citation matches its page: {out}"
    );
}
