//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/muscle-striation.adj`) driven through the
//! built CLI: a native `table` recording whether each of the three
//! muscle-tissue types is striated -- a sibling to the already-shipped
//! `muscle-types.adj` (which only carries one distinctive characteristic
//! per muscle type), decoding the striated/lacks-striations clause already
//! sitting unused inside that table's own per-muscle header quotes.
//! Resolves forward and backward recall queries with the source's
//! citation -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_musclestriation_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/muscle-striation.adj");
    std::fs::copy(&src, dir.join("muscle-striation.adj"))
        .expect("copy shipped muscle-striation.adj");
}

#[test]
fn muscle_striation_recalls_cardiac_as_striated_with_citation() {
    let dir = scratch("cardiac");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-striation.adj\"\n\
         ? muscle_striated(cardiac, $Striated)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"muscle_striated(cardiac, yes)\""),
        "cardiac muscle should recall as striated: {out}"
    );
    assert!(
        out.contains("seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NCI SEER citation: {out}"
    );
}

#[test]
fn muscle_striation_backward_recalls_smooth_as_the_only_non_striated_type() {
    let dir = scratch("nonstriated");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-striation.adj\"\n\
         ? muscle_striated($Muscle, no)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"muscle_striated(smooth, no)\""),
        "smooth muscle should be the only recalled non-striated type: {out}"
    );
    assert!(
        !out.contains("muscle_striated(skeletal, no)"),
        "skeletal muscle is striated, not the negative recall: {out}"
    );
    assert!(
        !out.contains("muscle_striated(cardiac, no)"),
        "cardiac muscle is striated, not the negative recall: {out}"
    );
}

#[test]
fn muscle_striation_covers_all_three_types_without_abstention() {
    let dir = scratch("noabstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-striation.adj\"\n\
         ? muscle_striated(skeletal, $S1)\n\
         ? muscle_striated(smooth, $S2)\n\
         ? muscle_striated(cardiac, $S3)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        !out.contains("\"abstained\":true"),
        "all three muscle-tissue types have a striation fact on record -- no abstention expected: {out}"
    );
}
