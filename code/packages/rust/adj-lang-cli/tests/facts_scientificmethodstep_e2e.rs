//! End-to-end test for the science FACTS library
//! (`adj-facts-stdlib/science/scientific-method-step.adj`) driven through the
//! built CLI: a native `table` naming the seven steps NASA Space Place's own
//! "Steps in Scientific Method" student page gives -- state a hypothesis,
//! define variables and controls, research, design the experiment, run the
//! experiment and record data, analyze the data, and draw conclusions.
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
    let dir = std::env::temp_dir().join(format!("adjcli_scimethodstep_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("science/scientific-method-step.adj");
    std::fs::copy(&src, dir.join("scientific-method-step.adj"))
        .expect("copy shipped scientific-method-step.adj");
}

#[test]
fn scientific_method_step_recall_binds_the_action_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-method-step.adj\"\n\
         ? scientific_method_step(step_1, $A)\n\
         ? scientific_method_step(step_4, $A)\n\
         ? scientific_method_step(step_7, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"A\":\"ask_a_question_or_state_a_hypothesis\""),
        "step 1 means ask_a_question_or_state_a_hypothesis: {out}"
    );
    assert!(
        out.contains("\"A\":\"design_the_experiment\""),
        "step 4 means design_the_experiment: {out}"
    );
    assert!(
        out.contains("\"A\":\"draw_conclusions_and_write_a_report\""),
        "step 7 means draw_conclusions_and_write_a_report: {out}"
    );
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn scientific_method_step_reverse_binds_the_step_for_that_action() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-method-step.adj\"\n\
         ? scientific_method_step($S, analyze_the_data)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"step_6\""),
        "the shipped analyze_the_data action is step_6: {out}"
    );
}

#[test]
fn scientific_method_step_abstains_honestly_on_an_undefined_step_number() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-method-step.adj\"\n\
         ? scientific_method_step(step_8, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the source page only ever numbers steps 1 through 7 -- there is no step 8 to \
         recall, so honest abstention, never invented: {out}"
    );
}
