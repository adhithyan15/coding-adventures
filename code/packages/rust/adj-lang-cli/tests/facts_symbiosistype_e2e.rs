//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/symbiosis-type.adj`) driven through the built
//! CLI: a native `table` naming three types of symbiotic relationship and
//! what actually defines each, quoted verbatim from Wikipedia's
//! "Symbiosis" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_symbiosis_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/symbiosis-type.adj");
    std::fs::copy(&src, dir.join("symbiosis-type.adj")).expect("copy shipped symbiosis-type.adj");
}

#[test]
fn symbiosis_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"symbiosis-type.adj\"\n\
         ? symbiosis_type(mutualism, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"both_parties_benefit\""),
        "mutualism means both_parties_benefit: {out}"
    );
    assert!(
        out.contains("en.wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wikipedia citation: {out}"
    );
}

#[test]
fn symbiosis_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"symbiosis-type.adj\"\n\
         ? symbiosis_type($T, the_parasite_benefits_while_the_host_is_harmed)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"parasitism\""),
        "the shipped the_parasite_benefits_while_the_host_is_harmed example is parasitism: {out}"
    );
}

#[test]
fn symbiosis_type_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"symbiosis-type.adj\"\n\
         ? symbiosis_type(amensalism, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "amensalism is a real interaction category the source names, but its own sentence bundles it together with competition rather than stating one clean fact -- honest abstention, never invented: {out}"
    );
}
