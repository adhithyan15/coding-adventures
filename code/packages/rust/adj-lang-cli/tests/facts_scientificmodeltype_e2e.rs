//! End-to-end test for the science FACTS library
//! (`adj-facts-stdlib/science/scientific-model-type.adj`) driven through the
//! built CLI: a native `table` naming the three kinds of scientific model
//! CK-12's Middle School Earth Science FlexBook 2.0 defines -- physical,
//! conceptual, and mathematical -- each with its own defining sentence.
//! 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_scimodeltype_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("science/scientific-model-type.adj");
    std::fs::copy(&src, dir.join("scientific-model-type.adj"))
        .expect("copy shipped scientific-model-type.adj");
}

#[test]
fn scientific_model_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-model-type.adj\"\n\
         ? scientific_model_type(physical, $D)\n\
         ? scientific_model_type(conceptual, $D)\n\
         ? scientific_model_type(mathematical, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"smaller_and_simpler_representation_of_the_thing_being_studied\""),
        "physical means a smaller and simpler representation: {out}"
    );
    assert!(
        out.contains("\"D\":\"ties_together_many_ideas_to_explain_a_phenomenon_or_event\""),
        "conceptual means tying together many ideas: {out}"
    );
    assert!(
        out.contains(
            "\"D\":\"sets_of_equations_that_take_into_account_many_factors_to_represent_a_phenomenon\""
        ),
        "mathematical means sets of equations: {out}"
    );
    assert!(
        out.contains("k12.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the CK-12/LibreTexts citation at consensus trust: {out}"
    );
}

#[test]
fn scientific_model_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-model-type.adj\"\n\
         ? scientific_model_type($T, ties_together_many_ideas_to_explain_a_phenomenon_or_event)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"conceptual\""),
        "the shipped ties-together-many-ideas description is the conceptual type: {out}"
    );
}

#[test]
fn scientific_model_type_abstains_honestly_on_an_undefined_computer_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-model-type.adj\"\n\
         ? scientific_model_type(computer, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the source closes its taxonomy at physical/conceptual/mathematical -- there is no \
         separate `computer` type to recall, so honest abstention, never invented: {out}"
    );
}
