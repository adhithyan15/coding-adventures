//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/heart-valves.adj`) driven through the built CLI:
//! a native `table` of heart-valve → boundary resolves binding-query recalls
//! (forward and backward) with the source's NCI SEER citation, and abstains on
//! a valve that is not one of the four cardiac valves (the eustachian valve) —
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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

#[test]
fn anatomy_heart_valves_recall_binds_boundary_with_citation() {
    let dir = scratch("heartvalves");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/heart-valves.adj");
    std::fs::copy(&src, dir.join("heart-valves.adj")).expect("copy shipped heart-valves.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"heart-valves.adj\"\n\
         ? valve_separates(tricuspid, $B)\n\
         ? valve_separates(pulmonary, $B)\n\
         ? valve_separates($V, left_ventricle_and_aorta)\n\
         ? valve_separates(eustachian, $B)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The tricuspid is the right atrioventricular valve — between the right
    // atrium and right ventricle; the pulmonary sits at the pulmonary trunk
    // (the recalled boundaries, forward binds).
    assert!(
        out.contains("\"B\":\"right_atrium_and_right_ventricle\""),
        "tricuspid → right_atrium_and_right_ventricle: {out}"
    );
    assert!(
        out.contains("\"B\":\"right_ventricle_and_pulmonary_trunk\""),
        "pulmonary → right_ventricle_and_pulmonary_trunk: {out}"
    );
    // The relation runs BACKWARD: bind the boundary, recall the valve.
    assert!(
        out.contains("\"V\":\"aortic\""),
        "left_ventricle_and_aorta → aortic (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training citation as its proof.
    assert!(
        out.contains("training.seer.cancer.gov/anatomy/cardiovascular/heart/structure.html")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The eustachian valve is not one of the four cardiac valves in this table —
    // honest abstention, never a fabricated boundary.
    assert!(out.contains("\"abstained\":true"), "eustachian abstains: {out}");
}
