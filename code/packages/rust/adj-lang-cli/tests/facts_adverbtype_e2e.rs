//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/adverb-type.adj`) driven through the built
//! CLI: a native `table` naming four adverb types and what each actually
//! describes, quoted verbatim from Grammarly's "What Is an Adverb?"
//! article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_adverb_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/adverb-type.adj");
    std::fs::copy(&src, dir.join("adverb-type.adj")).expect("copy shipped adverb-type.adj");
}

#[test]
fn adverb_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"adverb-type.adj\"\n\
         ? adverb_type(manner, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"describes_how_an_action_is_performed\""),
        "manner means describes_how_an_action_is_performed: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn adverb_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"adverb-type.adj\"\n\
         ? adverb_type($T, describes_how_long_an_action_lasts)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"duration\""),
        "the shipped describes_how_long_an_action_lasts example is duration: {out}"
    );
}

#[test]
fn adverb_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"adverb-type.adj\"\n\
         ? adverb_type(time, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "time is a real adverb type the same source table names, but its sentence bundles when/how-long/how-often rather than one clean fact -- honest abstention, never invented: {out}"
    );
}
