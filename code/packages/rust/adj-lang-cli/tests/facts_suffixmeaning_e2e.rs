//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/suffix-meaning.adj`) driven through the
//! built CLI: a native `table` naming nine common derivational suffixes
//! and what each actually means, quoted verbatim from Reading Rockets'
//! "Common Suffixes" chart. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_suffix_meaning_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/suffix-meaning.adj");
    std::fs::copy(&src, dir.join("suffix-meaning.adj"))
        .expect("copy shipped suffix-meaning.adj");
}

#[test]
fn suffix_meaning_recall_binds_the_meaning_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"suffix-meaning.adj\"\n\
         ? suffix_meaning(_ful, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"M\":\"full_of\""),
        "-ful means full_of: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn suffix_meaning_reverse_binds_the_suffix_for_that_meaning() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"suffix-meaning.adj\"\n\
         ? suffix_meaning($S, without)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"_less\""),
        "the shipped without example is _less: {out}"
    );
}

#[test]
fn suffix_meaning_reverse_binds_both_spellings_sharing_one_meaning() {
    let dir = scratch("reverse_shared");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"suffix-meaning.adj\"\n\
         ? suffix_meaning($S, is_or_can_be)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"_able\""),
        "the source's own bundled -able/-ible row must bind _able: {out}"
    );
    assert!(
        out.contains("\"S\":\"_ible\""),
        "the source's own bundled -able/-ible row must ALSO bind _ible: {out}"
    );
}

#[test]
fn suffix_meaning_recall_binds_a_material_category_row() {
    let dir = scratch("direct_material");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"suffix-meaning.adj\"\n\
         ? suffix_meaning(_en, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"M\":\"made_of\""),
        "-en means made_of: {out}"
    );
}

#[test]
fn suffix_meaning_abstains_honestly_on_an_untabled_suffix() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"suffix-meaning.adj\"\n\
         ? suffix_meaning(_ic, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "_ic is a real suffix the same source chart also covers, but not one of the nine tabled here -- honest abstention, never invented: {out}"
    );
}

#[test]
fn suffix_meaning_recall_binds_the_disambiguated_agentive_er_sense() {
    let dir = scratch("direct_agentive_er");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"suffix-meaning.adj\"\n\
         ? suffix_meaning(_er_agentive, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"M\":\"one_who_or_person_connected_with\""),
        "the disambiguated agentive -er sense (\"one who; person connected with\", as in \
         \"teacher\") must resolve, kept genuinely distinct in spelling from the excluded \
         comparative -er (\"more\", as in \"taller\") this table never tables under a bare \
         `_er` atom: {out}"
    );
}

#[test]
fn suffix_meaning_reverse_binds_both_agentive_spellings_sharing_one_meaning() {
    let dir = scratch("reverse_agentive");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"suffix-meaning.adj\"\n\
         ? suffix_meaning($S, one_who_or_person_connected_with)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"_er_agentive\""),
        "the source's own bundled -er, -or row must bind _er_agentive: {out}"
    );
    assert!(
        out.contains("\"S\":\"_or\""),
        "the source's own bundled -er, -or row must ALSO bind _or: {out}"
    );
}
