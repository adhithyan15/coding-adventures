//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/valve-kind.adj`) driven through the built
//! CLI: a native `table` naming the physiological CLASS of each heart
//! valve (atrioventricular vs. semilunar), decoded from the classifying
//! adjective already sitting unused inside `heart-valves.adj`'s own
//! already-quoted NCI SEER sentences -- a sibling to that table. Resolves
//! binding-query recall (both directions, including a 2-answer backward
//! recall) with the source's citation, and abstains on a real cardiac
//! valve name (eustachian) that is not one of the four valves this table
//! covers -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_valvekind_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/valve-kind.adj");
    std::fs::copy(&src, dir.join("valve-kind.adj")).expect("copy shipped valve-kind.adj");
}

#[test]
fn valve_kind_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"valve-kind.adj\"\n\
         ? valve_kind(tricuspid, $Kind)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"valve_kind(tricuspid, atrioventricular)\""),
        "the tricuspid is an atrioventricular valve: {out}"
    );
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NCI SEER citation: {out}"
    );
}

#[test]
fn valve_kind_recalls_backward_both_semilunar_valves() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"valve-kind.adj\"\n\
         ? valve_kind($Valve, semilunar)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for valve in ["pulmonary", "aortic"] {
        assert!(
            out.contains(&format!("\"term\":\"valve_kind({valve}, semilunar)\"")),
            "backward recall should include {valve}: {out}"
        );
    }
}

#[test]
fn valve_kind_abstains_honestly_on_eustachian() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"valve-kind.adj\"\n\
         ? valve_kind(eustachian, $Kind)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "eustachian is a real cardiac valve name but not one of the four valves this table covers -- honest abstention: {out}"
    );
}
