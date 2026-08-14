//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/valve-alternate-name.adj`) driven through the
//! built CLI: a native `table` naming the everyday alternate name for the
//! mitral heart valve, decoded from a clause already sitting unused inside
//! `heart-valves.adj`'s / `valve-kind.adj`'s own already-quoted NCI SEER
//! source sentence -- a sibling to those tables. Resolves binding-query
//! recall (both directions) with the source's citation, and abstains on a
//! real, already-tabled valve (tricuspid) whose own quote never supplies an
//! alternate name -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_valvealternatename_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/valve-alternate-name.adj");
    std::fs::copy(&src, dir.join("valve-alternate-name.adj"))
        .expect("copy shipped valve-alternate-name.adj");
}

#[test]
fn valve_alternate_name_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"valve-alternate-name.adj\"\n\
         ? valve_alternate_name(mitral, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"valve_alternate_name(mitral, bicuspid)\""),
        "the mitral valve is also called the bicuspid valve: {out}"
    );
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NCI SEER citation: {out}"
    );
}

#[test]
fn valve_alternate_name_recalls_backward_to_mitral() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"valve-alternate-name.adj\"\n\
         ? valve_alternate_name($Valve, bicuspid)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"valve_alternate_name(mitral, bicuspid)\""),
        "bicuspid recalls the mitral valve: {out}"
    );
}

#[test]
fn valve_alternate_name_abstains_honestly_on_tricuspid() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"valve-alternate-name.adj\"\n\
         ? valve_alternate_name(tricuspid, $AltName)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "tricuspid is a real, already-tabled heart valve but its own quote never supplies an alternate name -- honest abstention: {out}"
    );
}
