//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/heart-chamber-vessel.adj`) driven through the
//! built CLI: a native `table` naming the vessel(s)/valve(s) each heart
//! chamber connects to, decoded from spans already sitting unused inside
//! the SAME StatPearls quotes `heart-chambers.adj`'s own header already
//! reproduces -- a sibling to that table, and a genuinely different axis
//! from `heart-valves.adj`'s valve-keyed `valve_separates` table. Resolves
//! binding-query recall (both directions) with the source's citation, and
//! abstains on a real cardiac structure (septum) that is not one of the
//! four chambers -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_heartchambervessel_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/heart-chamber-vessel.adj");
    std::fs::copy(&src, dir.join("heart-chamber-vessel.adj"))
        .expect("copy shipped heart-chamber-vessel.adj");
}

#[test]
fn heart_chamber_vessel_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heart-chamber-vessel.adj\"\n\
         ? heart_chamber_vessel(left_ventricle, $Vessel)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"heart_chamber_vessel(left_ventricle, aortic_valve)\""),
        "the left ventricle connects to the aortic valve: {out}"
    );
    assert!(
        out.contains("ncbi.nlm.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the StatPearls citation: {out}"
    );
}

#[test]
fn heart_chamber_vessel_recalls_backward_both_right_atrium_vessels() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heart-chamber-vessel.adj\"\n\
         ? heart_chamber_vessel($Chamber, superior_vena_cava)\n\
         ? heart_chamber_vessel($Chamber2, inferior_vena_cava)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"heart_chamber_vessel(right_atrium, superior_vena_cava)\""),
        "superior_vena_cava -> right_atrium: {out}"
    );
    assert!(
        out.contains("\"term\":\"heart_chamber_vessel(right_atrium, inferior_vena_cava)\""),
        "inferior_vena_cava -> right_atrium: {out}"
    );
}

#[test]
fn heart_chamber_vessel_abstains_honestly_on_septum() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heart-chamber-vessel.adj\"\n\
         ? heart_chamber_vessel(septum, $Vessel)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "septum is a real cardiac structure but not one of the four chambers -- honest abstention: {out}"
    );
}
