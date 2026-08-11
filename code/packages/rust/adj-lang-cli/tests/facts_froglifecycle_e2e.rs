//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/frog-life-cycle.adj`) driven through the
//! built CLI: a native `table` naming each of the frog's three life-cycle
//! stages as a number, quoted verbatim from National Geographic Kids UK's
//! "The Frog Life Cycle for Kids" page -- a sibling library to the
//! already-shipped `monarch-life-cycle.adj`, applying the SAME plain
//! numbered life-cycle-stage recall shape to a different organism. 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_froglifecycle_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/frog-life-cycle.adj");
    std::fs::copy(&src, dir.join("frog-life-cycle.adj"))
        .expect("copy shipped frog-life-cycle.adj");
}

#[test]
fn frog_life_stage_recall_binds_the_order_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"frog-life-cycle.adj\"\n\
         ? frog_life_stage(tadpole, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"N\":\"2\""),
        "tadpole is the second life stage: {out}"
    );
    assert!(
        out.contains("natgeokids.com") && out.contains("\"trust\":\"consensus\""),
        "carries the National Geographic Kids citation: {out}"
    );
}

#[test]
fn frog_life_stage_reverse_binds_the_stage_for_that_order() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"frog-life-cycle.adj\"\n\
         ? frog_life_stage($S, 1)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"egg\""),
        "egg is the first life stage: {out}"
    );
}

#[test]
fn frog_life_stage_abstains_honestly_on_an_untabled_stage_name() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"frog-life-cycle.adj\"\n\
         ? frog_life_stage(adult, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"adult\" is not the word this specific source uses for the third stage (it says \"frog\") -- honest abstention, never invented: {out}"
    );
}
