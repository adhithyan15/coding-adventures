//! End-to-end test for the engineering FACTS library
//! (`adj-facts-stdlib/engineering/engineering-design-step.adj`) driven
//! through the built CLI: a native `table` naming the six steps NASA's own
//! "Engineering Design Process" student worksheet gives -- identify the
//! problem, identify criteria and constraints, brainstorm possible
//! solutions, select a design, build/test/refine (a single combined
//! heading), and share the design. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_engdesignstep_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("engineering/engineering-design-step.adj");
    std::fs::copy(&src, dir.join("engineering-design-step.adj"))
        .expect("copy shipped engineering-design-step.adj");
}

#[test]
fn engineering_design_step_recall_binds_the_action_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"engineering-design-step.adj\"\n\
         ? engineering_design_step(step_1, $A)\n\
         ? engineering_design_step(step_3, $A)\n\
         ? engineering_design_step(step_8, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"A\":\"identify_the_problem\""),
        "step 1 means identify_the_problem: {out}"
    );
    assert!(
        out.contains("\"A\":\"brainstorm_possible_solutions\""),
        "step 3 means brainstorm_possible_solutions: {out}"
    );
    assert!(
        out.contains("\"A\":\"share_the_design\""),
        "step 8 means share_the_design: {out}"
    );
    assert!(
        out.contains("nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NASA citation: {out}"
    );
}

#[test]
fn engineering_design_step_reverse_binds_the_step_for_that_action() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"engineering-design-step.adj\"\n\
         ? engineering_design_step($S, select_a_design)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"step_4\""),
        "the shipped select_a_design action is step_4: {out}"
    );
}

#[test]
fn engineering_design_step_abstains_honestly_on_an_ungrouped_step_number() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"engineering-design-step.adj\"\n\
         ? engineering_design_step(step_5, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "step_5 is a real step number the source's own step count implies exists, but the \
         worksheet only ever names it as part of the COMBINED 'Steps 5-7' heading, never on \
         its own -- honest abstention, never invented: {out}"
    );
}
