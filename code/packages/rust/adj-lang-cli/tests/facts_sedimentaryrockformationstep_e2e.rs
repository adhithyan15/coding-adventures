//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/sedimentary-rock-formation-step.adj`)
//! driven through the built CLI: a native `table` naming the three ordered
//! stages of sedimentary rock formation (weathering, erosion, compaction)
//! and each stage's step number, grounding National Geographic Education's
//! "The Rock Cycle" article. Runs the relation BACKWARD as a genuine
//! reverse recall (step number -> stage), and abstains honestly on
//! `deposition` (a term OTHER sources use for a stage between erosion and
//! compaction, but never named by this source's own paragraph) and
//! `melting` (a real term the SAME source uses, but only for the separate
//! igneous-rock path). 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "adjcli_factssedrockstep_{tag}_{}",
        std::process::id()
    ));
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
    let src = facts_stdlib().join("earth-science/sedimentary-rock-formation-step.adj");
    std::fs::copy(&src, dir.join("sedimentary-rock-formation-step.adj"))
        .expect("copy shipped sedimentary-rock-formation-step.adj");
}

#[test]
fn sedimentary_rock_formation_step_recall_binds_step_number_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sedimentary-rock-formation-step.adj\"\n\
         ? sedimentary_rock_formation_step(weathering, $N)\n\
         ? sedimentary_rock_formation_step(erosion, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Weathering starts the process; erosion is the second stage.
    assert!(out.contains("\"N\":\"1\""), "weathering -> 1: {out}");
    assert!(out.contains("\"N\":\"2\""), "erosion -> 2: {out}");
    // The answer carries the National Geographic Education citation as proof.
    assert!(
        out.contains("education.nationalgeographic.org")
            && out.contains("\"trust\":\"consensus\""),
        "carries the National Geographic Education citation: {out}"
    );
}

#[test]
fn sedimentary_rock_formation_step_reverse_binds_stage_from_step_number() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sedimentary-rock-formation-step.adj\"\n\
         ? sedimentary_rock_formation_step($S, 3)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The relation runs BACKWARD: binding step number 3 recalls compaction,
    // the final stage the source's own paragraph names.
    assert!(
        out.contains("sedimentary_rock_formation_step(compaction, 3)"),
        "step 3 is compaction: {out}"
    );
}

#[test]
fn sedimentary_rock_formation_step_abstains_honestly_on_deposition_and_melting() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sedimentary-rock-formation-step.adj\"\n\
         ? sedimentary_rock_formation_step(deposition, $N)\n\
         ? sedimentary_rock_formation_step(melting, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // `deposition` is a term OTHER K-8 sources use for a stage between
    // erosion and compaction, but this source's own paragraph never names
    // it as its own numbered stage -- honest abstention, never invented.
    // `melting` is a real term the SAME cited page uses, but only for the
    // separate igneous-rock path, not this sedimentary sequence.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "both deposition and melting abstain honestly: {out}"
    );
}
