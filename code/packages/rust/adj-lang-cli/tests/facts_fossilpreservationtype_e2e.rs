//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/fossil-preservation-type.adj`) driven through
//! the built CLI: a native `table` naming the three preservation structures
//! a fossil can be found as, quoted verbatim from the National Park
//! Service's "Mold Casts and Steinkerns" article. 0 answer-time model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fossil_preservation_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geology/fossil-preservation-type.adj");
    std::fs::copy(&src, dir.join("fossil-preservation-type.adj"))
        .expect("copy shipped fossil-preservation-type.adj");
}

#[test]
fn fossil_preservation_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fossil-preservation-type.adj\"\n\
         ? fossil_preservation_type(mold, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"three_dimensional_impression_of_all_or_part_of_a_body_fossil_or_trace_fossil\""),
        "mold means three_dimensional_impression_of_all_or_part_of_a_body_fossil_or_trace_fossil: {out}"
    );
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the National Park Service citation: {out}"
    );
}

#[test]
fn fossil_preservation_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fossil-preservation-type.adj\"\n\
         ? fossil_preservation_type($T, consists_of_the_evidence_of_living_organisms_but_not_the_actual_organism_itself)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"trace_fossil\""),
        "the shipped consists_of_the_evidence_of_living_organisms_but_not_the_actual_organism_itself example is trace_fossil: {out}"
    );
}

#[test]
fn fossil_preservation_type_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fossil-preservation-type.adj\"\n\
         ? fossil_preservation_type(steinkern, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "steinkern is a real, well-documented term, but the source frames it as a specific kind of cast, not a fourth preservation type -- honest abstention, never invented: {out}"
    );
}
